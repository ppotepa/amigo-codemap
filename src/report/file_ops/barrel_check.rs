use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;

use crate::model::CodeMap;

use super::common::{read_text_at_root, slash_path};
use super::exports::parse_ts_exports;
use super::model::{FileOpReport, NextAction, Risk, RiskLevel};

pub fn print_barrel_check(root: &Path, map: &CodeMap, query: &str, limit: usize) -> Result<()> {
    let dir = root.join(query);
    let mut scope = vec![format!("dir: {}", slash_path(&dir))];
    let mut findings = Vec::new();
    let mut duplicates = BTreeMap::<String, Vec<String>>::new();
    let mut has_index = false;

    if dir.exists() && dir.is_dir() {
        for entry in walk_dir(&dir)? {
            if let Some(file_name) = entry.file_name().and_then(|name| name.to_str()) {
                if file_name == "index.ts" || file_name == "index.tsx" {
                    has_index = true;
                }
                if file_name.ends_with(".ts") || file_name.ends_with(".tsx") {
                    let rel = entry
                        .strip_prefix(root)
                        .unwrap_or(&entry)
                        .to_string_lossy()
                        .replace('\\', "/");
                    let text = read_text_at_root(root, &entry)?;
                    let exports = parse_ts_exports(&text);
                    for export in exports {
                        if export.name.is_empty() || export.name == "pub use" {
                            continue;
                        }
                        duplicates.entry(export.name).or_default().push(rel.clone());
                    }
                }
            }
        }
    } else {
        bail_or_file_not_found(query)?;
    }

    if has_index {
        findings.push("index exists: yes".to_string());
    } else {
        findings.push("index exists: no".to_string());
    }
    findings.push("exports:".to_string());
    for (name, paths) in duplicates.iter().take(limit) {
        findings.push(format!("  {name} -> {}", paths.join(", ")));
    }

    let duplicates_count = duplicates.values().filter(|paths| paths.len() > 1).count();
    findings.push(format!("duplicates: {duplicates_count}"));
    if duplicates_count > 0 {
        findings.push("unused candidates:".to_string());
        for (name, paths) in duplicates.iter() {
            if paths.len() == 1 {
                continue;
            }
            findings.push(format!("  {name}: {}", paths.join(" | ")));
        }
    }

    let mut risks = Vec::new();
    if !has_index {
        risks.push(Risk {
            level: RiskLevel::Medium,
            message: "missing barrel may increase import churn".to_string(),
        });
    }
    if duplicates_count > 0 {
        risks.push(Risk {
            level: RiskLevel::High,
            message: "duplicate exports found across directory".to_string(),
        });
    }

    if !map.git.changed.is_empty() {
        scope.push(format!("changed in scope: {}", map.git.changed.len()));
    }
    super::model::print_report(&FileOpReport {
        task: format!("barrel-check {query}"),
        scope,
        findings,
        risks,
        verify: vec!["npm run build".to_string()],
        next: vec![
            NextAction {
                label: "add index.ts only if imports need consolidation".to_string(),
            },
            NextAction {
                label: "remove duplicate exports".to_string(),
            },
        ],
    });
    Ok(())
}

fn walk_dir(dir: &std::path::Path) -> Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(walk_dir(&path)?);
        } else {
            files.push(path);
        }
    }
    Ok(files)
}

fn bail_or_file_not_found(query: &str) -> Result<()> {
    if query.trim().is_empty() {
        anyhow::bail!("barrel-check requires a directory or file path")
    } else {
        anyhow::bail!("barrel-check path not found: {query}")
    }
}
