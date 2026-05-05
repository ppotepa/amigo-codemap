use crate::model::{CodeMap, SymbolEntry};
use crate::report::common::slash_path;

use super::model::{FileOpReport, NextAction, Risk, RiskLevel};

pub fn print_diff_scope(map: &CodeMap, limit: usize) {
    let scope = vec![format!("changed files: {}", map.git.changed.len())];
    let mut findings = Vec::new();
    let mut risks = Vec::new();
    let mut total_symbols = 0usize;

    for change in map.git.changed.iter().take(limit) {
        let path = slash_path(&change.path);
        findings.push(format!("{path} [{}]", change.status));
        let symbols = symbols_for_file(map, change.file_id.as_deref());
        total_symbols += symbols.len();
        if symbols.is_empty() {
            findings.push("  no indexed symbols".to_string());
        } else {
            for symbol in symbols.iter().take(6) {
                findings.push(format!("  {} {} {}", symbol.kind, symbol.name, symbol.line));
            }
        }
    }

    if map.git.changed.iter().any(|change| change.status == "D") {
        risks.push(Risk {
            level: RiskLevel::High,
            message: "removed files may leave stale imports".to_string(),
        });
    }

    if total_symbols > limit {
        risks.push(Risk {
            level: RiskLevel::Medium,
            message: "many changed symbols; run stale on removed names".to_string(),
        });
    }

    super::model::print_report(&FileOpReport {
        task: "diff-scope".to_string(),
        scope,
        findings,
        risks,
        verify: vec!["cargo test -p amigo-editor --lib".to_string()],
        next: vec![
            NextAction {
                label: "run stale on removed symbol names".to_string(),
            },
            NextAction {
                label: "run impact on changed exported types".to_string(),
            },
            NextAction {
                label: "run verify-plan".to_string(),
            },
        ],
    });
}

fn symbols_for_file<'a>(map: &'a CodeMap, file_id: Option<&str>) -> Vec<&'a SymbolEntry> {
    file_id
        .map(|id| {
            map.symbols
                .iter()
                .filter(|symbol| symbol.file_id == id)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}
