use std::path::Path;

use crate::model::CodeMap;
use anyhow::Result;

use super::common::{changed_by_path, find_file_by_path, is_changed, slash_path, text_refs_like};
use super::model::{FileOpReport, NextAction, Risk, RiskLevel};

pub fn print_delete_plan(root: &Path, map: &CodeMap, query: &str, limit: usize) -> Result<()> {
    let mut scope = Vec::new();
    let mut findings = Vec::new();
    let mut inbound_total = 0usize;

    if let Some(file) = find_file_by_path(map, query) {
        scope.push(format!("path: {}", slash_path(&file.path)));
        scope.push(format!(
            "status: {}",
            if is_changed(map, &file.path) {
                "changed"
            } else {
                "index-only"
            }
        ));
        scope.push(format!("lines: {}", file.lines));
        scope.push("symbols:".to_string());
        for symbol in map
            .symbols
            .iter()
            .filter(|symbol| symbol.file_id == file.id)
        {
            findings.push(format!("  {} {}", symbol.kind, symbol.name));
        }
        for symbol in map
            .symbols
            .iter()
            .filter(|symbol| symbol.file_id == file.id)
        {
            let refs = text_refs_like(root, map, &symbol.name, limit * 4)?
                .into_iter()
                .filter(|reference| reference.path != slash_path(&file.path))
                .collect::<Vec<_>>();
            if !refs.is_empty() {
                inbound_total += refs.len();
                findings.push(format!("inbound refs: {} for {}", refs.len(), symbol.name));
            }
            for reference in refs.into_iter().take(2) {
                findings.push(format!(
                    "  {}:{} (changed: {})",
                    slash_path(&reference.path),
                    reference.line,
                    reference.changed
                ));
            }
        }
    } else {
        scope.push(format!("path: {query}"));
        scope.push("status: unknown".to_string());
        findings.push("not indexed; treat as not found".to_string());
        let refs = text_refs_like(root, map, query, limit)?;
        inbound_total = refs.len();
        findings.push(format!("path refs: {}", inbound_total));
        for reference in refs.into_iter().take(8) {
            findings.push(format!(
                "  {}:{}",
                slash_path(&reference.path),
                reference.line
            ));
        }
    }

    let safe = inbound_total == 0;
    findings.push(format!("safe delete: {}", if safe { "yes" } else { "no" }));

    let risks = if safe {
        Vec::new()
    } else {
        vec![Risk {
            level: RiskLevel::High,
            message: "migrate inbound refs before deletion".to_string(),
        }]
    };

    let changed_refs = changed_by_path(map).len();
    let mut verify = vec!["npm run build".to_string(), "npm test".to_string()];
    if changed_refs == 0 {
        verify.push("cargo test -p amigo-editor --lib".to_string());
    }

    let next = if safe {
        vec![
            NextAction {
                label: "keep file deleted if behavior allows".to_string(),
            },
            NextAction {
                label: "run stale on old symbol names".to_string(),
            },
        ]
    } else {
        vec![
            NextAction {
                label: "migrate inbound references first".to_string(),
            },
            NextAction {
                label: "rerun delete-plan".to_string(),
            },
        ]
    };

    super::model::print_report(&FileOpReport {
        task: format!("delete-plan {query}"),
        scope,
        findings,
        risks,
        verify,
        next,
    });
    Ok(())
}
