use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;
use serde_json::Value;

use crate::model::{FileEntry, PackageEntry};

pub fn scan_packages(root: &Path, files: &[FileEntry]) -> Result<Vec<PackageEntry>> {
    let mut packages = Vec::new();
    packages.extend(scan_cargo_metadata(root)?);
    packages.extend(scan_package_jsons(root, files)?);
    packages.sort_by(|a, b| {
        (&a.kind, &a.manifest_path, &a.name).cmp(&(&b.kind, &b.manifest_path, &b.name))
    });
    packages.dedup_by(|a, b| a.kind == b.kind && a.manifest_path == b.manifest_path);
    for (index, package) in packages.iter_mut().enumerate() {
        package.id = format!("p{}", index + 1);
    }
    Ok(packages)
}

fn scan_cargo_metadata(root: &Path) -> Result<Vec<PackageEntry>> {
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(root)
        .output();
    let Ok(output) = output else {
        return Ok(Vec::new());
    };
    if !output.status.success() {
        return Ok(Vec::new());
    }
    let value: Value = serde_json::from_slice(&output.stdout)?;
    let packages = value
        .get("packages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|package| cargo_package(root, package))
        .collect();
    Ok(packages)
}

fn cargo_package(root: &Path, value: &Value) -> Option<PackageEntry> {
    let name = value.get("name")?.as_str()?.to_owned();
    let version = value
        .get("version")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let manifest_path = value
        .get("manifest_path")
        .and_then(Value::as_str)
        .map(PathBuf::from)?;
    let dependencies = value
        .get("dependencies")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|dependency| {
            dependency
                .get("name")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .collect();

    Some(PackageEntry {
        id: String::new(),
        kind: "cargo".to_owned(),
        name,
        version,
        manifest_path: relative_path(root, &manifest_path),
        dependencies,
        scripts: Vec::new(),
    })
}

fn scan_package_jsons(root: &Path, files: &[FileEntry]) -> Result<Vec<PackageEntry>> {
    let mut packages = Vec::new();
    for file in files.iter().filter(|file| file.language == "package") {
        let text = std::fs::read_to_string(root.join(&file.path))?;
        let Ok(value) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        let name = value
            .get("name")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| {
                file.path
                    .parent()
                    .and_then(Path::file_name)
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| "package".to_owned())
            });
        let version = value
            .get("version")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let dependencies = [
            "dependencies",
            "devDependencies",
            "peerDependencies",
            "optionalDependencies",
        ]
        .into_iter()
        .flat_map(|key| object_keys(value.get(key)))
        .collect();
        let scripts = object_keys(value.get("scripts"));

        packages.push(PackageEntry {
            id: String::new(),
            kind: "package".to_owned(),
            name,
            version,
            manifest_path: file.path.clone(),
            dependencies,
            scripts,
        });
    }
    Ok(packages)
}

fn object_keys(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_object)
        .map(|object| object.keys().cloned().collect())
        .unwrap_or_default()
}

fn relative_path(root: &Path, path: &Path) -> PathBuf {
    if let Ok(relative) = path.strip_prefix(root) {
        return relative.to_path_buf();
    }
    let root_text = normalize_path_text(root);
    let path_text = normalize_path_text(path);
    path_text
        .strip_prefix(root_text.trim_end_matches('/'))
        .map(|relative| relative.trim_start_matches('/'))
        .filter(|relative| !relative.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| path.to_path_buf())
}

fn normalize_path_text(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches("//?/")
        .to_owned()
}
