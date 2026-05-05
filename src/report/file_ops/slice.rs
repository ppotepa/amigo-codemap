use std::path::Path;

use anyhow::Result;

use crate::model::CodeMap;

use super::common::{
    find_file_by_path, import_block, is_changed, line_window, read_text_at_root, slash_path,
    symbols_in_file,
};
use super::model::{FileOpReport, NextAction, Risk, RiskLevel};

pub fn print_slice(
    root: &Path,
    map: &CodeMap,
    query: &str,
    symbol: Option<&str>,
    radius: usize,
) -> Result<()> {
    let file = find_file_by_path(map, query)
        .ok_or_else(|| anyhow::anyhow!("slice requires an existing file path: {query}"))?;
    let path = file.path.clone();
    let text = read_text_at_root(root, &path)?;
    let symbols = symbols_in_file(map, &file.id);
    let target = symbol
        .and_then(|name| symbols.iter().find(|entry| entry.name == name))
        .or_else(|| symbols.first());

    let mut scope = vec![
        format!("file: {}", slash_path(&path)),
        format!("lines: {}", file.lines),
        format!("language: {}", file.language),
    ];
    if let Some(target) = target {
        scope.push(format!(
            "symbol: {}:{}:{}",
            target.kind, target.name, target.line
        ));
    }

    let mut findings = Vec::new();
    findings.push("imports:".to_string());
    for (line, text) in import_block(&text).into_iter().take(20) {
        findings.push(format!("  {line}: {text}"));
    }

    findings.push("symbol window:".to_string());
    if let Some(target) = target {
        for (line_no, line_text) in line_window(&text, target.line, radius) {
            findings.push(format!("  {line_no}: {}", line_text));
        }
    } else {
        for (line_no, line_text) in line_window(&text, 1, radius) {
            findings.push(format!("  {line_no}: {}", line_text));
        }
    }

    if !symbols.is_empty() {
        let mut local_deps = std::collections::BTreeSet::new();
        for dependency in &map.dependencies {
            if dependency.from == file.id {
                local_deps.insert(dependency.to.clone());
            }
        }
        findings.push("local deps:".to_string());
        if local_deps.is_empty() {
            findings.push("  none".to_string());
        } else {
            for item in local_deps.iter() {
                findings.push(format!("  {item}"));
            }
        }
    }

    let mut risks = Vec::new();
    if is_changed(map, &path) {
        risks.push(Risk {
            level: RiskLevel::Medium,
            message: "local context changed; include verify-plan before edit".to_string(),
        });
    }
    if target.is_none() {
        risks.push(Risk {
            level: RiskLevel::Low,
            message: "symbol not found; showing file window only".to_string(),
        });
    }

    let mut next = Vec::new();
    if target.is_some() {
        next.push(NextAction {
            label: "inspect this slice".to_string(),
        });
    } else {
        next.push(NextAction {
            label: "find a precise symbol name with --symbol".to_string(),
        });
    }
    next.push(NextAction {
        label: "run impact for changed exported symbol".to_string(),
    });

    super::model::print_report(&FileOpReport {
        task: format!("slice {query}"),
        scope,
        findings,
        risks,
        verify: vec![
            "npm run build".to_string(),
            "npm test".to_string(),
            "cargo test -p amigo-editor --lib".to_string(),
        ],
        next,
    });

    Ok(())
}
