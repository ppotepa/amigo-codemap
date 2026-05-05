use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;

use crate::model::CodeMap;

use super::model::{FileOpReport, NextAction};

pub fn print_commit_files(
    _root: &Path,
    map: &CodeMap,
    _changed_only: bool,
    _limit: usize,
) -> Result<()> {
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for change in &map.git.changed {
        let path = change.path.to_string_lossy();
        let bucket = commit_bucket(&path);
        groups
            .entry(bucket.to_string())
            .or_default()
            .push(path.to_string());
    }

    let mut findings = Vec::new();
    findings.push("suggested commits:".to_string());

    for (bucket, files) in &groups {
        findings.push(format!("  {bucket} ({} files)", files.len()));
        for file in files {
            findings.push(format!("    - {file}"));
        }
    }

    if findings.len() == 1 {
        findings.push("  no grouped files".to_string());
    }

    super::model::print_report(&FileOpReport {
        task: "commit-files".to_string(),
        scope: vec![format!("changed files: {}", map.git.changed.len())],
        findings,
        risks: Vec::new(),
        verify: vec![
            "npm test".to_string(),
            "npm run build".to_string(),
            "cargo test -p amigo-editor --lib".to_string(),
        ],
        next: vec![
            NextAction {
                label: "commit grouped files in this order".to_string(),
            },
            NextAction {
                label: "update commit-summary after split".to_string(),
            },
        ],
    });

    Ok(())
}

fn commit_bucket(path: &str) -> &'static str {
    if path.contains("crates/apps/amigo-editor/src/app/store") || path.contains("EditorSelection") {
        "editor selection refactor"
    } else if path.contains("src-tauri/src/commands") || path.contains("src-tauri/src/lib.rs") {
        "tauri command split"
    } else if path.contains("properties") || path.contains("features/inspector") {
        "properties and inspector"
    } else if path.contains("operations.md") || path.contains("README") {
        "docs"
    } else if path.contains("crates/tools/amigo-codemap") {
        "codemap tool"
    } else {
        "other"
    }
}
