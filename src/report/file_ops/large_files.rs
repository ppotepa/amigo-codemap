use crate::model::CodeMap;
use crate::report::common::slash_path;

use super::model::{FileOpReport, NextAction};

pub fn print_large_files(map: &CodeMap, top: usize) {
    let mut files: Vec<_> = map.files.iter().collect();
    files.sort_by_key(|file| std::cmp::Reverse(file.lines));

    let scope = vec![format!("top: {top}")];
    let mut findings = Vec::new();
    for file in files.into_iter().take(top) {
        let symbol_count = map
            .symbols
            .iter()
            .filter(|symbol| symbol.file_id == file.id)
            .count();
        findings.push(format!(
            "{} (lines: {})",
            slash_path(&file.path),
            file.lines
        ));
        findings.push(format!("  symbols: {}", symbol_count));
        findings.push(format!(
            "  split hints: {}",
            split_hint(&slash_path(&file.path))
        ));
    }

    super::model::print_report(&FileOpReport {
        task: "large-files".to_string(),
        scope,
        findings,
        risks: Vec::new(),
        verify: vec!["npm run build".to_string()],
        next: vec![
            NextAction {
                label: "split biggest candidate first".to_string(),
            },
            NextAction {
                label: "run move-plan for candidate file".to_string(),
            },
        ],
    });
}

fn split_hint(path: &str) -> &'static str {
    if path.contains("/commands/") {
        "group by command domain"
    } else if path.contains("ProjectExplorer") {
        "tree/actions/node UI"
    } else if path.contains("builtinComponents") {
        "component registry/feature panels"
    } else {
        "group by symbols/import clusters"
    }
}
