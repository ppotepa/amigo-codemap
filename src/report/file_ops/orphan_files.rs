use std::collections::BTreeSet;
use std::path::Path;

use anyhow::Result;

use crate::model::CodeMap;

use super::common::{read_text_at_root, slash_path};
use super::model::{FileOpReport, NextAction, Risk, RiskLevel};

pub fn print_orphan_files(root: &Path, map: &CodeMap, query: &str, limit: usize) -> Result<()> {
    let prefix = query.replace('\\', "/");
    let scope = vec![format!("scope: {query}")];
    let mut findings = Vec::new();
    let mut candidates = Vec::new();

    let mut inbound_ids = BTreeSet::new();
    for dep in &map.dependencies {
        inbound_ids.insert(dep.to.clone());
    }

    for file in map.files.iter() {
        let path = slash_path(&file.path);
        if !path.starts_with(&prefix) {
            continue;
        }
        if is_entry_point(&path) {
            continue;
        }

        let has_ref = inbound_ids.contains(&file.id);
        if has_ref {
            continue;
        }

        let text = read_text_at_root(root, &file.path).unwrap_or_default();
        let inbound = text.lines().count();
        if inbound == 0 {
            candidates.push((file, 0usize));
        } else {
            findings.push(format!("{path}: {} inbound-like lines", inbound));
        }
    }

    if candidates.is_empty() {
        findings.push("candidates: none".to_string());
    } else {
        findings.push("candidates:".to_string());
        for (file, _count) in candidates.into_iter().take(limit) {
            findings.push(format!("  {} (inbound: 0)", slash_path(&file.path)));
        }
    }

    let mut risks = Vec::new();
    if findings.iter().any(|item| item.contains("candidates")) {
        risks.push(Risk {
            level: RiskLevel::Medium,
            message: "one-line shim candidates might be intentional compatibility".to_string(),
        });
    }

    super::model::print_report(&FileOpReport {
        task: format!("orphan-files {query}"),
        scope,
        findings,
        risks,
        verify: vec!["npm test".to_string()],
        next: vec![
            NextAction {
                label: "inspect 1-line shim candidates".to_string(),
            },
            NextAction {
                label: "delete if no compatibility layer needed".to_string(),
            },
        ],
    });
    Ok(())
}

fn is_entry_point(path: &str) -> bool {
    path.ends_with("/main.rs")
        || path.ends_with("/lib.rs")
        || path.ends_with("/mod.rs")
        || path.ends_with("/main.tsx")
        || path.ends_with("/index.ts")
        || path.ends_with("/index.tsx")
}
