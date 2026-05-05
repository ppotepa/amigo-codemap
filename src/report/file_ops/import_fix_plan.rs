use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;

use crate::model::CodeMap;

use super::common::{
    changed_by_path, changed_status, changed_status_by_path, read_text_at_root, resolve_relative_import,
    slash_path,
};
use super::imports::parse_ts_imports;
use super::model::{FileOpReport, NextAction};

pub fn print_import_fix_plan(
    root: &Path,
    map: &CodeMap,
    changed_only: bool,
    limit: usize,
) -> Result<()> {
    let changed_ids = changed_by_path(map);
    let changed_statuses = changed_status_by_path(map);
    let mut broken = 0usize;
    let mut stale = 0usize;
    let mut findings = Vec::new();

    for file in &map.files {
        if changed_only && !changed_ids.contains(&slash_path(&file.path)) {
            continue;
        }
        if !super::super::common::is_ts_source(&file.path)
            && !super::super::common::is_rust_source(&file.path)
        {
            continue;
        }

        let text = read_text_at_root(root, &file.path)?;
        for import in parse_ts_imports(&file.path, &text) {
            if !import.specifier.starts_with('.') {
                continue;
            }

            if let Some(resolved) = resolve_relative_import(root, &file.path, &import.specifier) {
                match changed_status(&changed_statuses, &resolved) {
                    Some("D") => {
                        findings.push(format!(
                            "stale import: {} in {}:{}",
                            import.specifier,
                            slash_path(&file.path),
                            import.line
                        ));
                        stale += 1;
                    }
                    Some(_) => {
                        findings.push(format!(
                            "target changed: {} in {}:{}",
                            import.specifier,
                            slash_path(&file.path),
                            import.line
                        ));
                    }
                    None => {}
                }
            } else if is_deleted_relative_target(&changed_statuses, root, &file.path, &import.specifier)
            {
                findings.push(format!(
                    "stale import: {} in {}:{}",
                    import.specifier,
                    slash_path(&file.path),
                    import.line
                ));
                stale += 1;
            } else {
                findings.push(format!(
                    "missing import {} in {}:{}",
                    import.specifier,
                    slash_path(&file.path),
                    import.line
                ));
                broken += 1;
            }
        }
    }

    let mut depth_map = BTreeMap::<String, usize>::new();
    for file in &map.files {
        let depth = file.path.components().count();
        let bucket = if depth > 8 {
            "deep".to_string()
        } else {
            "flat".to_string()
        };
        *depth_map.entry(bucket).or_default() += 1;
    }

    let scope = vec![format!(
        "files scanned: {}",
        if changed_only { "changed" } else { "all" }
    )];

    let mut sorted: Vec<String> = findings.into_iter().take(limit).collect();
    let mut lines = Vec::new();
    lines.push("broken imports:".to_string());
    if sorted.is_empty() {
        lines.push("  none".to_string());
    } else {
        lines.extend(sorted.drain(..).map(|item| format!("  {item}")));
    }
    lines.push("relative depth warnings:".to_string());
    for (bucket, count) in depth_map {
        lines.push(format!("  {bucket}: {count}"));
    }

    let risks = vec![
        super::model::Risk {
            level: super::model::RiskLevel::High,
            message: format!("{broken} broken imports"),
        },
        super::model::Risk {
            level: super::model::RiskLevel::Medium,
            message: format!("{stale} stale file references"),
        },
    ];

    super::model::print_report(&FileOpReport {
        task: "import-fix-plan".to_string(),
        scope,
        findings: lines,
        risks,
        verify: vec!["npm run build".to_string()],
        next: vec![
            NextAction {
                label: "fix missing import targets".to_string(),
            },
            NextAction {
                label: "rerun changed build and fallout".to_string(),
            },
            NextAction {
                label: "run verify-plan".to_string(),
            },
        ],
    });

    Ok(())
}

fn is_deleted_relative_target(
    changed_statuses: &std::collections::BTreeMap<String, String>,
    root: &Path,
    source_file: &Path,
    specifier: &str,
) -> bool {
    if !specifier.starts_with('.') {
        return false;
    }

    let source = root.join(source_file);
    let Some(source_dir) = source.parent() else {
        return false;
    };

    let base = source_dir.join(specifier);
    let candidates = [
        base.clone(),
        base.with_extension("ts"),
        base.with_extension("tsx"),
        base.join("index.ts"),
        base.join("index.tsx"),
    ];

    candidates.into_iter().any(|candidate| is_deleted_candidate(&candidate, root, changed_statuses))
}

fn is_deleted_candidate(
    candidate: &Path,
    root: &Path,
    changed_statuses: &std::collections::BTreeMap<String, String>,
) -> bool {
    let normalized = normalize_path(candidate);
    let normalized_without_root = candidate
        .strip_prefix(root)
        .map_or_else(|_| normalized.clone(), |path| normalize_path(path));

    for (path, status) in changed_statuses {
        if status != "D" {
            continue;
        }

        if path == &normalized || path == &normalized_without_root {
            return true;
        }

        if path.ends_with(&normalized_without_root) {
            return true;
        }
    }

    false
}

fn normalize_path(path: &Path) -> String {
    let mut parts: Vec<String> = Vec::new();

    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !parts.is_empty() {
                    parts.pop();
                }
            }
            std::path::Component::Normal(part) => {
                parts.push(part.to_string_lossy().to_string());
            }
            std::path::Component::RootDir => {}
            std::path::Component::Prefix(prefix) => {
                parts.push(prefix.as_os_str().to_string_lossy().to_string());
            }
        }
    }

    parts.join("/")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use super::is_deleted_relative_target;

    #[test]
    fn detects_deleted_relative_target() {
        let root = PathBuf::from("repo");
        let source_file = PathBuf::from("crates/apps/amigo-editor/src/app/store/editorStore.ts");
        let mut statuses = BTreeMap::new();
        statuses.insert("crates/apps/amigo-editor/src/app/b.ts".to_string(), "D".to_string());

        assert!(is_deleted_relative_target(
            &statuses,
            &root,
            &source_file,
            "../b",
        ));
    }

    #[test]
    fn ignores_present_or_unchanged_relative_target() {
        let root = PathBuf::from("repo");
        let source_file = PathBuf::from("crates/apps/amigo-editor/src/app/store/editorStore.ts");
        let mut statuses = BTreeMap::new();
        statuses.insert("other.ts".to_string(), "M".to_string());

        assert!(!is_deleted_relative_target(
            &statuses,
            &root,
            &source_file,
            "../b",
        ));
    }
}
