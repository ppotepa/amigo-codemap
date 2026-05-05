use std::io::{self, Read};
use std::path::Path;

use anyhow::Result;

use super::diff::parse_patch_files;
use super::model::{FileOpReport, NextAction, Risk, RiskLevel};

pub fn print_patch_preview(from: Option<&Path>, limit: usize) -> Result<()> {
    let text = if let Some(path) = from {
        std::fs::read_to_string(path)?
    } else {
        let mut text = String::new();
        io::stdin().read_to_string(&mut text)?;
        text
    };

    let scope = vec![format!(
        "files: {}",
        if from.is_some() { "from file" } else { "stdin" }
    )];
    let files = parse_patch_files(&text);

    let mut findings = Vec::new();
    findings.push("areas:".to_string());

    for file in files.iter().take(limit) {
        findings.push(format!("  {} -> {}", file.old_path, file.new_path));
        findings.push(format!("    added hunks: {}", file.added_lines.len()));
    }

    if files.is_empty() {
        findings.push("  no patch hunks found".to_string());
    }

    let mut risks = Vec::new();
    if files.iter().any(|file| file.added_lines.len() > 20) {
        risks.push(Risk {
            level: RiskLevel::Medium,
            message: "some files have large added blocks".to_string(),
        });
    }

    if !findings.is_empty() && findings.len() > 1 {
        risks.push(Risk {
            level: RiskLevel::Low,
            message: "review symbols in large files before apply".to_string(),
        });
    }

    super::model::print_report(&FileOpReport {
        task: "patch-preview".to_string(),
        scope,
        findings,
        risks,
        verify: vec![
            "npm test".to_string(),
            "npm run build".to_string(),
            "cargo test -p amigo-editor --lib".to_string(),
        ],
        next: vec![
            NextAction {
                label: "inspect risky files first".to_string(),
            },
            NextAction {
                label: "apply patch for low-risk files first".to_string(),
            },
            NextAction {
                label: "run fallout on build output".to_string(),
            },
        ],
    });

    Ok(())
}
