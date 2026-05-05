use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::Result;

use super::common::slash_path;
use super::model::{FileOpReport, NextAction};

pub fn print_asset_file_check(root: &Path, query: &str, limit: usize) -> Result<()> {
    let module_root = root.join(query);
    let mut spritesheets = 0usize;
    let mut fonts = 0usize;
    let mut scenes = 0usize;
    let mut missing_sources = BTreeSet::new();
    let mut duplicate_ids: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut referenced_raw = BTreeSet::new();

    let manifest_files = if module_root.exists() {
        walk_asset_files(&module_root)?
    } else {
        Vec::new()
    };

    for manifest in &manifest_files {
        let text = std::fs::read_to_string(manifest).unwrap_or_default();
        let relative_manifest = slash_path(manifest.strip_prefix(root).unwrap_or(manifest));

        for id in yaml_field_values(&text, &["id"]) {
            duplicate_ids
                .entry(id)
                .or_default()
                .push(relative_manifest.clone());
        }

        for value in yaml_field_values(
            &text,
            &["path", "source", "image", "scene", "spritesheet", "font"],
        ) {
            let resolved = manifest
                .parent()
                .unwrap_or_else(|| Path::new(""))
                .join(&value);
            referenced_raw.insert(slash_path(&resolved.strip_prefix(root).unwrap_or(&resolved)));
            if looks_like_asset_path(&value) && !resolved.exists() {
                missing_sources.insert(value);
            }
        }

        spritesheets += text.matches("spritesheet").count();
        fonts += text.matches("font").count();
        scenes += text.matches("scene").count();
    }

    let raw_files = walk_raw_files(&module_root.join("raw"))?;
    let unused_raw = raw_files
        .into_iter()
        .filter_map(|path| {
            let relative = slash_path(&path.strip_prefix(root).unwrap_or(&path));
            (!referenced_raw.contains(&relative)).then_some(relative)
        })
        .take(limit)
        .collect::<Vec<_>>();

    let duplicate_values = duplicate_ids
        .into_iter()
        .filter_map(|(id, files)| (files.len() > 1).then_some((id, files)))
        .collect::<Vec<_>>();

    let mut findings = vec![
        format!("spritesheets: {spritesheets}"),
        format!("fonts: {fonts}"),
        format!("scenes: {scenes}"),
        "missing sources:".to_string(),
    ];

    if missing_sources.is_empty() {
        findings.push("  none".to_string());
    } else {
        for value in missing_sources.iter().take(limit) {
            findings.push(format!("  {value}"));
        }
    }

    findings.push("duplicate ids:".to_string());
    if duplicate_values.is_empty() {
        findings.push("  none".to_string());
    } else {
        for (id, files) in duplicate_values.into_iter().take(limit) {
            findings.push(format!("  {id}"));
            for file in files {
                findings.push(format!("    {file}"));
            }
        }
    }

    findings.push("unused raw files:".to_string());
    if unused_raw.is_empty() {
        findings.push("  none".to_string());
    } else {
        for item in unused_raw {
            findings.push(format!("  {item}"));
        }
    }

    super::model::print_report(&FileOpReport {
        task: format!("asset-file-check {query}"),
        scope: vec![format!("module: {query}")],
        findings,
        risks: Vec::new(),
        verify: vec!["scene-preview smoke".to_string()],
        next: vec![
            NextAction {
                label: "inspect missing sources first".to_string(),
            },
            NextAction {
                label: "remove duplicate ids or unused raw files".to_string(),
            },
        ],
    });

    Ok(())
}

fn walk_asset_files(path: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !path.exists() {
        return Ok(files);
    }

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let child = entry.path();
        if child.is_dir() {
            files.extend(walk_asset_files(&child)?);
        } else if child
            .extension()
            .is_some_and(|ext| ext == "yml" || ext == "yaml" || ext == "json")
        {
            files.push(child);
        }
    }
    Ok(files)
}

fn walk_raw_files(path: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !path.exists() {
        return Ok(files);
    }

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let child = entry.path();
        if child.is_dir() {
            files.extend(walk_raw_files(&child)?);
        } else {
            files.push(child);
        }
    }
    Ok(files)
}

fn looks_like_asset_path(value: &str) -> bool {
    value.contains('/')
        || value.ends_with(".png")
        || value.ends_with(".jpg")
        || value.ends_with(".json")
        || value.ends_with(".yaml")
        || value.ends_with(".yml")
}

fn yaml_field_values(text: &str, keys: &[&str]) -> Vec<String> {
    let mut values = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        for key in keys {
            let prefix = format!("{key}:");
            if let Some(rest) = trimmed.strip_prefix(&prefix) {
                let value = rest.trim().trim_matches('"').trim_matches('\'');
                if !value.is_empty() {
                    values.push(value.to_string());
                }
            }
        }
    }
    values
}

#[cfg(test)]
mod tests {
    use super::yaml_field_values;

    #[test]
    fn collects_yaml_fields() {
        let values = yaml_field_values("id: hero\nscene: scenes/start.scene.json\n", &["id", "scene"]);
        assert_eq!(values, vec!["hero".to_string(), "scenes/start.scene.json".to_string()]);
    }
}
