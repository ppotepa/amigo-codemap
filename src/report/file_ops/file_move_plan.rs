use anyhow::Result;
use std::collections::BTreeSet;
use std::path::Path;

use crate::model::CodeMap;

use super::common::{
    find_file_by_path, read_text_at_root, relative_module_from_paths, resolve_relative_import,
    slash_path,
};
use super::imports::parse_imports;
use super::model::{FileOpReport, NextAction, Risk, RiskLevel};

pub fn print_file_move_plan(
    root: &Path,
    map: &CodeMap,
    query: &str,
    to: &Path,
    limit: usize,
) -> Result<()> {
    let from = find_file_by_path(map, query)
        .ok_or_else(|| anyhow::anyhow!("file-move-plan requires a real file path"))?;
    let text = read_text_at_root(root, &from.path)?;
    let out_to = if to.is_absolute() {
        to.to_path_buf()
    } else {
        root.join(to)
    };
    let to_relative = out_to
        .strip_prefix(root)
        .map_or_else(|_| out_to.clone(), Path::to_path_buf);
    let to_display = slash_path(&to_relative);

    let mut imports = parse_imports(&from.path, &text);
    let scope = vec![
        format!("from: {}", slash_path(&from.path)),
        format!("to: {to_display}"),
        format!("lines: {}", from.lines),
    ];

    let mut findings = Vec::new();
    findings.push("imports to rewrite:".to_string());
    for import in imports.iter_mut().take(limit) {
        if import.specifier.starts_with(".") {
            import.resolved = resolve_relative_import(root, &from.path, &import.specifier);
            findings.push(format!(
                "  {} -> {}",
                import.specifier,
                if let Some(resolved) = &import.resolved {
                    format!("{} (exists)", slash_path(resolved))
                } else {
                    "missing".to_string()
                }
            ));
        }
    }

    let mut rewrite_targets = BTreeSet::new();
    if let (Some(spec), Some(to_rel)) = (
        relative_module_from_paths(root, &from.path, &to_relative),
        relative_module_from_paths(root, &from.path, &out_to),
    ) {
        rewrite_targets.insert((spec, to_rel));
    }

    for (old, new) in rewrite_targets {
        findings.push(format!("  {old} -> {new}"));
    }

    findings.push("inbound imports:".to_string());
    for other in map.files.iter() {
        if other.id == from.id {
            continue;
        }
        let other_text = read_text_at_root(root, &other.path).unwrap_or_default();
        for import in parse_imports(&other.path, &other_text) {
            if !import.specifier.starts_with('.') {
                continue;
            }
            if let Some(resolved) = resolve_relative_import(root, &other.path, &import.specifier) {
                if resolved == from.path {
                    findings.push(format!("  {}:{}", slash_path(&other.path), import.line));
                }
            }
        }
    }

    let risks = vec![
        Risk {
            level: RiskLevel::Medium,
            message: "relative path depth may change imports".to_string(),
        },
        Risk {
            level: RiskLevel::Medium,
            message: "public boundary imports should be verified".to_string(),
        },
    ];

    super::model::print_report(&FileOpReport {
        task: format!("file-move-plan {}", query),
        scope,
        findings,
        risks,
        verify: vec!["npm run build".to_string()],
        next: vec![
            NextAction {
                label: "move file".to_string(),
            },
            NextAction {
                label: "update inbound imports".to_string(),
            },
            NextAction {
                label: "run verify-plan".to_string(),
            },
        ],
    });

    Ok(())
}
