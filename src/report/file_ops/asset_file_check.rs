use std::collections::BTreeSet;
use std::path::Path;

use anyhow::Result;

use super::common::{read_text_at_root, slash_path};
use super::model::{FileOpReport, NextAction};

pub fn print_asset_file_check(root: &Path, query: &str, _limit: usize) -> Result<()> {
    let mut assets = 0usize;
    let mut fonts = 0usize;
    let mut scenes = 0usize;
    let mut missing: BTreeSet<String> = BTreeSet::new();
    let mut duplicates = BTreeSet::new();
    let mut unused: Vec<String> = Vec::new();

    let query_lc = query.to_ascii_lowercase();
    let query_path = root.join("crates");
    if query_path.exists() {
        let entries = walk_files(&query_path)?;
        for path in entries {
            let path_text = slash_path(&path.strip_prefix(root).unwrap_or(&path).to_path_buf());
            if path_text.contains(&query_lc) {
                let Ok(text) = read_text_at_root(root, &path) else {
                    continue;
                };
                for value in yaml_values(&text, &["spritesheets", "fonts", "scenes"]) {
                    if value.ends_with(".png") || value.ends_with(".json") {
                        if !path_text.contains("assets") {
                            missing.insert(value);
                        }
                    }
                }
                assets += text.matches("spritesheet").count();
                fonts += text.matches("font").count();
                scenes += text.matches("scene").count();
            }
        }
    }

    if !duplicates.insert(query.to_string()) {
        unused.push("duplicate source id".to_string());
    }

    let mut findings = vec![
        format!("assets: {assets}"),
        format!("fonts: {fonts}"),
        format!("scenes: {scenes}"),
        format!("missing sources: {}", missing.len()),
        format!(
            "duplicate ids: {}",
            if duplicates.is_empty() { 0 } else { 1 }
        ),
    ];

    if !unused.is_empty() {
        findings.push("unused raw files:".to_string());
        for item in unused {
            findings.push(format!("  {item}"));
        }
    }

    super::model::print_report(&FileOpReport {
        task: format!("asset-file-check {query}"),
        scope: vec![format!("query: {query}")],
        findings,
        risks: Vec::new(),
        verify: vec!["scene-preview smoke".to_string()],
        next: vec![
            NextAction {
                label: "inspect missing/unused assets".to_string(),
            },
            NextAction {
                label: "run focused scene runtime checks".to_string(),
            },
        ],
    });

    Ok(())
}

fn walk_files(path: &std::path::Path) -> Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let child = entry.path();
        if child.is_dir() {
            files.extend(walk_files(&child)?);
        } else {
            let is_asset = child
                .extension()
                .is_some_and(|ext| ext == "yml" || ext == "yaml" || ext == "json");
            if is_asset {
                files.push(child);
            }
        }
    }
    Ok(files)
}

fn yaml_values(text: &str, keys: &[&str]) -> Vec<String> {
    let mut values = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        for key in keys {
            let needle = format!("{key}:");
            if let Some(rest) = trimmed.strip_prefix(&needle) {
                let value = rest.trim().trim_matches('"').trim_matches('\'');
                if !value.is_empty() {
                    values.push(value.to_string());
                }
            }
        }
    }
    values
}
