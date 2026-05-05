use std::path::Path;

use super::model::{FileOpReport, NextAction, Risk, RiskLevel};
use crate::model::CodeMap;

use super::common::{changed_by_path, slash_path};

pub fn print_text_check(root: &Path, map: &CodeMap, changed_only: bool, limit: usize) {
    let changed = changed_by_path(map);
    let mut scope = vec![format!(
        "changed files: {}",
        if changed_only {
            changed.len()
        } else {
            map.files.len()
        }
    )];
    let mut findings = Vec::new();
    let mut large = 0usize;
    let mut crlf = 0usize;
    let mut lf = 0usize;
    let mut binaries = 0usize;

    for file in &map.files {
        if changed_only && !changed.contains(&slash_path(&file.path)) {
            continue;
        }
        let path = root.join(&file.path);
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        if bytes.contains(&0) {
            binaries += 1;
            findings.push(format!("binary: {}", slash_path(&file.path)));
            continue;
        }
        let text = String::from_utf8_lossy(&bytes);
        if bytes.windows(2).any(|window| window == b"\r\n") {
            crlf += 1;
        }
        if text.contains('\n') {
            lf += 1;
        }
        if text.lines().count() > 1500 {
            large += 1;
            findings.push(format!("large: {} lines", slash_path(&file.path)));
        }
    }
    scope.push(format!("line endings CRLF/LF: {crlf}/{lf}"));
    scope.push(format!("binaries: {binaries}"));
    findings.push(format!(
        "changed files: {}",
        if changed_only {
            changed.len()
        } else {
            map.files.len()
        }
    ));
    findings.push(format!("large text files: {large}"));

    let mut risks = Vec::new();
    if crlf > lf {
        risks.push(Risk {
            level: RiskLevel::Low,
            message: "mixed line endings might cause patch noise".to_string(),
        });
    }
    if binaries > 0 {
        risks.push(Risk {
            level: RiskLevel::Medium,
            message: "binary files detected in scan".to_string(),
        });
    }
    if large > 0 {
        risks.push(Risk {
            level: RiskLevel::Medium,
            message: "large text files may need focused checks".to_string(),
        });
    }

    if findings.len() > limit {
        findings.truncate(limit);
    }

    super::model::print_report(&FileOpReport {
        task: "text-check".to_string(),
        scope,
        findings,
        risks,
        verify: vec!["npm run build".to_string()],
        next: vec![
            NextAction {
                label: "normalize problematic line endings".to_string(),
            },
            NextAction {
                label: "ignore lockfiles unless dependency work".to_string(),
            },
        ],
    });
}
