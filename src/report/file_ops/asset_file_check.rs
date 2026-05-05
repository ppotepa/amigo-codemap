use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde_yaml::Value;

use super::common::slash_path;
use super::model::{render_report, FileOpReport, NextAction};

pub fn print_asset_file_check(root: &Path, query: &str, limit: usize) -> Result<()> {
    let report = build_asset_file_report(root, query, limit)?;
    print!("{}", render_report(&report));
    Ok(())
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn render_asset_file_check(root: &Path, query: &str, limit: usize) -> Result<String> {
    let report = build_asset_file_report(root, query, limit)?;
    Ok(render_report(&report))
}

fn build_asset_file_report(root: &Path, query: &str, limit: usize) -> Result<FileOpReport> {
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
        let parsed = parse_yaml_document(&text);
        let values = yaml_field_values(
            &text,
            &["path", "source", "image", "scene", "spritesheet", "font"],
        );
        let structured_values = parsed
            .as_ref()
            .map(|value| collect_yaml_field_values(value, &["path", "source", "image", "scene", "spritesheet", "font"]))
            .unwrap_or_default();
        let all_values = if structured_values.is_empty() {
            values
        } else {
            structured_values
        };
        let ids_in_file = parsed
            .as_ref()
            .map(|value| collect_yaml_field_values(value, &["id"]).into_iter().collect::<BTreeSet<_>>())
            .unwrap_or_else(|| yaml_field_values(&text, &["id"]).into_iter().collect::<BTreeSet<_>>());

        for id in ids_in_file {
            duplicate_ids
                .entry(id)
                .or_default()
                .push(relative_manifest.clone());
        }

        for value in &all_values {
            let resolved = manifest
                .parent()
                .unwrap_or_else(|| Path::new(""))
                .join(value);
            referenced_raw.insert(normalize_repo_relative(root, &resolved));
            if looks_like_asset_path(value) && !resolved.exists() {
                missing_sources.insert(value.clone());
            }
        }

        if let Some(parsed) = parsed.as_ref() {
            spritesheets += count_yaml_field_occurrences(parsed, &["spritesheet"]);
            fonts += count_yaml_field_occurrences(parsed, &["font"]);
            scenes += count_yaml_field_occurrences(parsed, &["scene"]);
        } else {
            spritesheets += yaml_field_values(&text, &["spritesheet"]).len();
            fonts += yaml_field_values(&text, &["font"]).len();
            scenes += yaml_field_values(&text, &["scene"]).len();
        }
    }

    let raw_files = walk_raw_files(&module_root.join("raw"))?;
    let unused_raw = raw_files
        .into_iter()
        .filter_map(|path| {
            let relative = normalize_repo_relative(root, &path);
            (!referenced_raw.contains(&relative)).then_some(relative)
        })
        .take(limit)
        .collect::<Vec<_>>();

    let duplicate_values = duplicate_ids
        .into_iter()
        .filter_map(|(id, files)| {
            let unique = files.into_iter().collect::<BTreeSet<_>>();
            (unique.len() > 1).then_some((id, unique.into_iter().collect::<Vec<_>>()))
        })
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

    Ok(FileOpReport {
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
    })
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

fn parse_yaml_document(text: &str) -> Option<Value> {
    serde_yaml::from_str::<Value>(text).ok()
}

fn normalize_repo_relative(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    normalize_path(relative)
}

fn normalize_path(path: &Path) -> String {
    let mut parts: Vec<String> = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !parts.is_empty() {
                    parts.pop();
                }
            }
            std::path::Component::Normal(part) => {
                parts.push(part.to_string_lossy().to_string());
            }
            std::path::Component::RootDir => {}
            std::path::Component::Prefix(prefix) => {
                parts.push(prefix.as_os_str().to_string_lossy().to_string());
            }
        }
    }
    parts.join("/")
}

fn collect_yaml_field_values(value: &Value, keys: &[&str]) -> Vec<String> {
    let wanted = keys.iter().copied().collect::<BTreeSet<_>>();
    let mut out = Vec::new();
    collect_yaml_field_values_inner(value, &wanted, &mut out);
    out
}

fn count_yaml_field_occurrences(value: &Value, keys: &[&str]) -> usize {
    let wanted = keys.iter().copied().collect::<BTreeSet<_>>();
    count_yaml_field_occurrences_inner(value, &wanted)
}

fn count_yaml_field_occurrences_inner(value: &Value, keys: &BTreeSet<&str>) -> usize {
    match value {
        Value::Mapping(map) => map
            .iter()
            .map(|(key, value)| {
                let direct = matches!(key, Value::String(name) if keys.contains(name.as_str())) as usize;
                direct + count_yaml_field_occurrences_inner(value, keys)
            })
            .sum(),
        Value::Sequence(items) => items
            .iter()
            .map(|item| count_yaml_field_occurrences_inner(item, keys))
            .sum(),
        _ => 0,
    }
}

fn collect_yaml_field_values_inner(
    value: &Value,
    keys: &BTreeSet<&str>,
    out: &mut Vec<String>,
) {
    match value {
        Value::Mapping(map) => {
            for (key, value) in map {
                if let Value::String(name) = key {
                    if keys.contains(name.as_str()) {
                        collect_yaml_scalars(value, out);
                    }
                }
                collect_yaml_field_values_inner(value, keys, out);
            }
        }
        Value::Sequence(items) => {
            for item in items {
                collect_yaml_field_values_inner(item, keys, out);
            }
        }
        _ => {}
    }
}

fn collect_yaml_scalars(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(text) => out.push(text.clone()),
        Value::Number(number) => out.push(number.to_string()),
        Value::Bool(flag) => out.push(flag.to_string()),
        Value::Mapping(map) => {
            for (_, value) in map {
                collect_yaml_scalars(value, out);
            }
        }
        Value::Sequence(items) => {
            for item in items {
                collect_yaml_scalars(item, out);
            }
        }
        _ => {}
    }
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
    use std::fs;
    use std::path::PathBuf;

    use super::{
        build_asset_file_report, collect_yaml_field_values, count_yaml_field_occurrences,
        normalize_repo_relative, parse_yaml_document, render_asset_file_check, yaml_field_values,
    };

    #[test]
    fn collects_yaml_fields() {
        let values = yaml_field_values("id: hero\nscene: scenes/start.scene.json\n", &["id", "scene"]);
        assert_eq!(values, vec!["hero".to_string(), "scenes/start.scene.json".to_string()]);
    }

    #[test]
    fn fixture_reports_missing_duplicate_and_unused_assets() {
        let root = temp_root("asset-check");
        let module = root.join("mods/test-mod");
        fs::create_dir_all(module.join("defs")).expect("create defs dir");
        fs::create_dir_all(module.join("raw/images")).expect("create raw dir");
        fs::write(
            module.join("defs/assets.yml"),
            "id: hero\nspritesheet: ../raw/images/hero.png\nscene: ../raw/scenes/start.scene.json\n",
        )
        .expect("write assets manifest");
        fs::write(
            module.join("defs/duplicate.yml"),
            "id: hero\nfont: ../raw/fonts/main.ttf\nimage: ../raw/images/missing.png\n",
        )
        .expect("write duplicate manifest");
        fs::create_dir_all(module.join("raw/fonts")).expect("create fonts dir");
        fs::write(module.join("raw/images/hero.png"), "png").expect("write hero image");
        fs::write(module.join("raw/fonts/main.ttf"), "ttf").expect("write font");
        fs::write(module.join("raw/images/unused.png"), "png").expect("write unused image");

        let report = build_asset_file_report(root.as_path(), "mods/test-mod", 20)
            .expect("asset report should build");
        let rendered = crate::report::file_ops::model::render_report(&report);
        assert!(rendered.contains("missing sources:"));
        assert!(rendered.contains("../raw/scenes/start.scene.json"));
        assert!(rendered.contains("../raw/images/missing.png"));
        assert!(rendered.contains("duplicate ids:"));
        assert!(rendered.contains("hero"));
        assert!(rendered.contains("unused raw files:"));
        assert!(rendered.contains("mods/test-mod/raw/images/unused.png"));
    }

    #[test]
    fn extracts_nested_yaml_fields_with_parser() {
        let parsed = parse_yaml_document(
            "scene:\n  id: demo-scene\n  files:\n    - raw/scenes/demo.scene.json\nfont:\n  - raw/fonts/main.ttf\n",
        )
        .expect("yaml should parse");
        let values = collect_yaml_field_values(&parsed, &["scene", "font"]);
        assert!(values.contains(&"demo-scene".to_string()) || values.contains(&"raw/scenes/demo.scene.json".to_string()));
        assert!(values.contains(&"raw/fonts/main.ttf".to_string()));
    }

    #[test]
    fn counts_yaml_field_occurrences_by_key() {
        let parsed = parse_yaml_document(
            "scene:\n  id: demo-scene\n  files:\n    - raw/scenes/demo.scene.json\nfont:\n  - raw/fonts/main.ttf\n",
        )
        .expect("yaml should parse");
        assert_eq!(count_yaml_field_occurrences(&parsed, &["scene"]), 1);
        assert_eq!(count_yaml_field_occurrences(&parsed, &["font"]), 1);
    }

    #[test]
    fn normalizes_repo_relative_paths() {
        let root = PathBuf::from("repo");
        let path = root.join("mods/test-mod/defs/../raw/images/hero.png");
        assert_eq!(
            normalize_repo_relative(&root, &path),
            "mods/test-mod/raw/images/hero.png"
        );
    }

    #[test]
    fn snapshot_asset_file_check() {
        let root = temp_root("asset-snapshot");
        let module = root.join("mods/test-mod");
        fs::create_dir_all(module.join("defs")).expect("create defs dir");
        fs::create_dir_all(module.join("raw/images")).expect("create raw dir");
        fs::create_dir_all(module.join("raw/fonts")).expect("create fonts dir");
        fs::write(
            module.join("defs/assets.yml"),
            "id: hero\nspritesheet: ../raw/images/hero.png\nscene:\n  id: demo-scene\n  files:\n    - ../raw/scenes/start.scene.json\n",
        )
        .expect("write assets manifest");
        fs::write(
            module.join("defs/duplicate.yml"),
            "id: hero\nfont: ../raw/fonts/main.ttf\nimage: ../raw/images/missing.png\n",
        )
        .expect("write duplicate manifest");
        fs::write(module.join("raw/images/hero.png"), "png").expect("write hero image");
        fs::write(module.join("raw/fonts/main.ttf"), "ttf").expect("write font");
        fs::write(module.join("raw/images/unused.png"), "png").expect("write unused image");

        assert_eq!(
            render_asset_file_check(root.as_path(), "mods/test-mod", 20)
                .expect("asset report should render")
                .trim(),
            include_str!("../../../tests/snapshots/asset_file_check.snap").trim()
        );
    }

    fn temp_root(name: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time should advance")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("amigo-codemap-{name}-{unique}"));
        fs::create_dir_all(&root).expect("create temp root");
        root
    }
}
