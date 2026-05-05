use std::path::Path;

use crate::model::CodeMap;

use super::common::{changed_by_path, read_text_at_root, slash_path};
use super::model::{FileOpReport, NextAction, Risk, RiskLevel};
use anyhow::Result;

pub fn print_shim_check(
    root: &Path,
    map: &CodeMap,
    changed_only: bool,
    _limit: usize,
) -> Result<()> {
    let scope_paths = changed_by_path(map);
    let mut findings = Vec::new();
    let mut shims = 0usize;
    let mut compat = 0usize;

    for file in &map.files {
        if changed_only && !scope_paths.contains(&slash_path(&file.path)) {
            continue;
        }
        let Ok(text) = read_text_at_root(root, &file.path) else {
            continue;
        };
        if let Some(class) = classify_shim(&text) {
            if class == "re-export/mod shim" {
                shims += 1;
                findings.push(format!("shim: {}", slash_path(&file.path)));
            } else {
                compat += 1;
                findings.push(format!("compat: {}", slash_path(&file.path)));
            }
        }
    }

    let task = if changed_only {
        "changed shims"
    } else {
        "all shims"
    };
    let mut risks = Vec::new();
    if shims > 0 {
        risks.push(Risk {
            level: RiskLevel::Medium,
            message: "safe to delete zero-ref shims first".to_string(),
        });
    }
    if compat > 0 {
        risks.push(Risk {
            level: RiskLevel::Low,
            message: "compatibility shims may still be required".to_string(),
        });
    }

    super::model::print_report(&FileOpReport {
        task: format!("shim-check {task}"),
        scope: vec!["scan shims".to_string()],
        findings,
        risks,
        verify: vec!["npm test".to_string(), "npm run build".to_string()],
        next: vec![
            NextAction {
                label: "delete zero-ref shims".to_string(),
            },
            NextAction {
                label: "keep compatibility shims".to_string(),
            },
        ],
    });
    Ok(())
}

fn classify_shim(text: &str) -> Option<&'static str> {
    let lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(20)
        .collect::<Vec<_>>();
    if lines.is_empty() {
        return None;
    }

    if lines.len() > 5 {
        return None;
    }

    if lines.iter().all(|line| {
        line.starts_with("export ")
            || line.starts_with("pub use ")
            || line.starts_with("pub mod ")
            || line.starts_with("mod ")
    }) {
        return Some("re-export/mod shim");
    }
    if lines.iter().all(|line| line.contains("Placeholder")) {
        return Some("compat shim");
    }
    None
}
