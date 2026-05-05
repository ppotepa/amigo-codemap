use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::model::CodeMap;

use super::common::{read_text_at_root, slash_path};
use super::imports::parse_ts_imports;
use super::model::{FileOpReport, NextAction, Risk, RiskLevel};

pub fn print_case_check(
    root: &Path,
    map: &CodeMap,
    changed_only: bool,
    _limit: usize,
) -> Result<()> {
    let mut findings = Vec::new();
    let mut case_mismatches = 0usize;
    let mut changed_scope = Vec::new();
    if changed_only {
        for change in &map.git.changed {
            changed_scope.push(slash_path(&change.path));
        }
    }

    for file in &map.files {
        let target = slash_path(&file.path);
        if changed_only && !changed_scope.iter().any(|scope| scope == &target) {
            continue;
        }
        let text = read_text_at_root(root, &file.path)?;
        for import in parse_ts_imports(&file.path, &text) {
            if !import.specifier.starts_with('.') {
                continue;
            }
            if let Some((actual, declared)) = classify_case(root, &target, &import.specifier) {
                findings.push(format!(
                    "import {} -> expected {}",
                    import.specifier,
                    actual.display()
                ));
                findings.push(format!("  in {} {}", target, declared.to_string_lossy()));
                case_mismatches += 1;
            }
        }
    }

    let collisions = collisions(root)?;
    let mut risks = Vec::new();
    if case_mismatches > 0 {
        risks.push(Risk {
            level: RiskLevel::Medium,
            message: format!("{case_mismatches} case mismatches likely windows-only risk"),
        });
    }
    if !collisions.is_empty() {
        risks.push(Risk {
            level: RiskLevel::Medium,
            message: format!(
                "{} path collisions in case-insensitive map",
                collisions.len()
            ),
        });
    }

    super::model::print_report(&FileOpReport {
        task: "case-check".to_string(),
        scope: vec![format!("changed_only: {changed_only}")],
        findings,
        risks,
        verify: vec!["npm run build".to_string()],
        next: vec![
            NextAction {
                label: "fix import casing and rerun build".to_string(),
            },
            NextAction {
                label: "run fallout on remaining mismatch".to_string(),
            },
        ],
    });
    Ok(())
}

fn classify_case(root: &Path, source: &str, specifier: &str) -> Option<(PathBuf, PathBuf)> {
    if !specifier.starts_with('.') {
        return None;
    }
    let source_path = Path::new(source);
    let base_dir = source_path.parent()?;
    let candidate = base_dir.join(specifier);
    let (actual, expected) = resolve_case(root, &candidate)?;
    if expected
        .to_string_lossy()
        .as_ref()
        .eq(actual.to_string_lossy().as_ref())
    {
        None
    } else {
        Some((actual, expected))
    }
}

fn resolve_case(root: &Path, expected: &Path) -> Option<(PathBuf, PathBuf)> {
    let absolute = if expected.is_absolute() {
        expected.to_path_buf()
    } else {
        root.join(expected)
    };
    let mut current = PathBuf::new();
    for component in absolute.components() {
        let part = component.as_os_str().to_string_lossy().to_lowercase();
        let children = if current.as_os_str().is_empty() {
            std::fs::read_dir(root).ok()?
        } else {
            std::fs::read_dir(root.join(&current)).ok()?
        };
        let found = children.flatten().find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case(&part)
        })?;
        current.push(found.file_name());
    }
    Some((absolute, current))
}

fn collisions(root: &Path) -> Result<Vec<(String, usize)>> {
    let mut counts = HashMap::<String, usize>::new();
    for file in walk(root)? {
        let normalized = slash_path(&file).to_lowercase();
        *counts.entry(normalized).or_default() += 1;
    }
    Ok(counts
        .into_iter()
        .filter_map(|(path, count)| (count > 1).then_some((path, count)))
        .collect())
}

fn walk(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk(&path)?);
        } else {
            out.push(path);
        }
    }
    Ok(out)
}
