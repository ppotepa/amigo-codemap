use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Read};
use std::path::Path;

use anyhow::Result;

use crate::model::CodeMap;
use crate::report::common::{feature_group, slash_path};
use crate::report::verify_plan::plan_for_paths;

use super::diff::parse_patch_files;
use super::model::{FileOpReport, NextAction, Risk, RiskLevel};

pub fn print_patch_preview(
    root: &Path,
    map: &CodeMap,
    from: Option<&Path>,
    limit: usize,
) -> Result<()> {
    let text = if let Some(path) = from {
        std::fs::read_to_string(path)?
    } else {
        let mut text = String::new();
        io::stdin().read_to_string(&mut text)?;
        text
    };

    let files = parse_patch_files(&text);
    let touched = files
        .iter()
        .map(|file| std::path::PathBuf::from(&file.new_path))
        .collect::<Vec<_>>();
    let plan = plan_for_paths(touched.clone());

    let mut area_counts = BTreeMap::<String, usize>::new();
    let mut symbols = BTreeSet::<String>::new();

    for file in files.iter().take(limit) {
        *area_counts
            .entry(feature_group(&file.new_path))
            .or_default() += 1;

        if let Some(entry) = map
            .files
            .iter()
            .find(|candidate| slash_path(&candidate.path) == file.new_path)
        {
            for symbol in map.symbols.iter().filter(|symbol| symbol.file_id == entry.id) {
                if file.added_lines.iter().any(|line| *line >= symbol.line) {
                    symbols.insert(symbol.name.clone());
                }
            }
        }
    }

    let mut findings = vec![format!("files touched: {}", files.len()), "areas:".to_string()];
    if area_counts.is_empty() {
        findings.push("  none".to_string());
    } else {
        for (area, count) in area_counts {
            findings.push(format!("  {area}: {count} files"));
        }
    }

    findings.push("symbols likely touched:".to_string());
    if symbols.is_empty() {
        findings.push("  none".to_string());
    } else {
        for symbol in symbols.into_iter().take(limit) {
            findings.push(format!("  {symbol}"));
        }
    }

    let mut risks = Vec::new();
    if files.iter().any(|file| file.new_path.contains("/app/store/")) {
        risks.push(Risk {
            level: RiskLevel::High,
            message: "reducer/action compatibility".to_string(),
        });
    }
    if files
        .iter()
        .any(|file| file.new_path.contains("/properties/") || file.new_path.contains("registry"))
    {
        risks.push(Risk {
            level: RiskLevel::Medium,
            message: "properties or registry behavior".to_string(),
        });
    }
    if files.iter().any(|file| file.new_path.contains("/src-tauri/src/commands/")) {
        risks.push(Risk {
            level: RiskLevel::Medium,
            message: "Tauri command registration/import risk".to_string(),
        });
    }

    super::model::print_report(&FileOpReport {
        task: "patch-preview".to_string(),
        scope: vec![format!(
            "files: {}",
            if from.is_some() { "from file" } else { "stdin" }
        )],
        findings,
        risks,
        verify: plan.required.into_iter().collect(),
        next: vec![
            NextAction {
                label: "inspect highest-risk area first".to_string(),
            },
            NextAction {
                label: "apply patch".to_string(),
            },
            NextAction {
                label: "run fallout on build output".to_string(),
            },
        ],
    });

    let _ = root;
    Ok(())
}
