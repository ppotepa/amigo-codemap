use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;

use crate::model::CodeMap;

use super::common::{
    changed_by_path, changed_status, changed_status_by_path, read_text_at_root,
    resolve_relative_import, slash_path,
};
use super::imports::parse_ts_imports;
use super::model::{render_report, FileOpReport, NextAction, Risk, RiskLevel};

pub fn print_import_fix_plan(
    root: &Path,
    map: &CodeMap,
    changed_only: bool,
    limit: usize,
) -> Result<()> {
    let report = build_import_fix_report(root, map, changed_only, limit)?;
    print!("{}", render_report(&report));
    Ok(())
}

fn build_import_fix_report(
    root: &Path,
    map: &CodeMap,
    changed_only: bool,
    limit: usize,
) -> Result<FileOpReport> {
    let changed_ids = changed_by_path(map);
    let changed_statuses = changed_status_by_path(map);
    let mut broken = Vec::new();
    let mut stale = Vec::new();
    let mut depth_map = BTreeMap::<String, usize>::new();

    for file in &map.files {
        if changed_only && !changed_ids.contains(&slash_path(&file.path)) {
            continue;
        }
        if !super::super::common::is_ts_source(&file.path) {
            continue;
        }

        let text = read_text_at_root(root, &file.path)?;
        for import in parse_ts_imports(&file.path, &text) {
            if !import.specifier.starts_with('.') {
                continue;
            }

            let depth = import.specifier.matches("../").count();
            *depth_map
                .entry(if depth >= 2 { "deep".to_string() } else { "flat".to_string() })
                .or_default() += 1;

            if let Some(resolved) = resolve_relative_import(root, &file.path, &import.specifier) {
                if matches!(changed_status(&changed_statuses, &resolved), Some("D")) {
                    stale.push(format!(
                        "{}:{} {}",
                        slash_path(&file.path),
                        import.line,
                        import.specifier
                    ));
                }
            } else if is_deleted_relative_target(&changed_statuses, root, &file.path, &import.specifier) {
                stale.push(format!(
                    "{}:{} {}",
                    slash_path(&file.path),
                    import.line,
                    import.specifier
                ));
            } else {
                let candidate = guess_candidate(map, &import.specifier);
                broken.push(format!(
                    "{}:{} {}\n    target: missing{}",
                    slash_path(&file.path),
                    import.line,
                    import.specifier,
                    candidate
                        .map(|value| format!("\n    candidate: {value}"))
                        .unwrap_or_default()
                ));
            }
        }
    }

    let mut findings = vec![
        format!("broken candidates: {}", broken.len()),
        "broken imports:".to_string(),
    ];
    if broken.is_empty() {
        findings.push("  none".to_string());
    } else {
        for item in broken.iter().take(limit) {
            findings.push(format!("  {item}"));
        }
    }

    findings.push("stale imports:".to_string());
    if stale.is_empty() {
        findings.push("  none".to_string());
    } else {
        for item in stale.iter().take(limit) {
            findings.push(format!("  {item}"));
        }
    }

    findings.push("relative depth warnings:".to_string());
    if depth_map.is_empty() {
        findings.push("  none".to_string());
    } else {
        for (bucket, count) in depth_map {
            findings.push(format!("  {bucket}: {count}"));
        }
    }

    let risks = vec![
        Risk {
            level: RiskLevel::High,
            message: format!("{} broken imports", broken.len()),
        },
        Risk {
            level: RiskLevel::Medium,
            message: format!("{} stale file references", stale.len()),
        },
    ];

    Ok(FileOpReport {
        task: "import-fix-plan".to_string(),
        scope: vec![format!(
            "files scanned: {}",
            if changed_only { "changed" } else { "all" }
        )],
        findings,
        risks,
        verify: vec!["npm run build".to_string()],
        next: vec![
            NextAction {
                label: "fix missing import targets".to_string(),
            },
            NextAction {
                label: "replace stale imports to deleted files".to_string(),
            },
            NextAction {
                label: "rerun build and fallout".to_string(),
            },
        ],
    })
}

fn guess_candidate(map: &CodeMap, specifier: &str) -> Option<String> {
    let needle = specifier
        .trim_start_matches("./")
        .trim_start_matches("../")
        .rsplit('/')
        .next()?;

    map.files.iter().find_map(|file| {
        let path = slash_path(&file.path);
        (path.contains(needle) && (path.ends_with(".ts") || path.ends_with(".tsx"))).then_some(path)
    })
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

    candidates
        .into_iter()
        .any(|candidate| is_deleted_candidate(&candidate, root, changed_statuses))
}

fn is_deleted_candidate(
    candidate: &Path,
    root: &Path,
    changed_statuses: &std::collections::BTreeMap<String, String>,
) -> bool {
    let normalized = normalize_path(candidate);
    let normalized_without_root = candidate
        .strip_prefix(root)
        .map_or_else(|_| normalized.clone(), normalize_path);

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

    use crate::model::{CodeMap, FileEntry, GitChange, GitInfo};

    use super::{build_import_fix_report, is_deleted_relative_target};

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

    #[test]
    fn reports_deleted_and_missing_imports_with_candidate() {
        let root = temp_root("import-fix");
        std::fs::create_dir_all(root.join("src/app")).expect("create app dir");
        std::fs::create_dir_all(root.join("src/features")).expect("create features dir");
        std::fs::write(
            root.join("src/app/MainEditorWindow.tsx"),
            "import { Panels } from \"./workspacePanels\";\nimport { Next } from \"./SelectionPanel\";\n",
        )
        .expect("write source");
        std::fs::write(
            root.join("src/features/SelectionPanel.tsx"),
            "export const SelectionPanel = () => null;\n",
        )
        .expect("write candidate");

        let map = CodeMap {
            root_name: "repo".to_string(),
            stats: BTreeMap::new(),
            files: vec![
                FileEntry {
                    id: "f1".to_string(),
                    path: PathBuf::from("src/app/MainEditorWindow.tsx"),
                    language: "tsx".to_string(),
                    lines: 2,
                    hash: String::new(),
                    size: 0,
                },
                FileEntry {
                    id: "f2".to_string(),
                    path: PathBuf::from("src/features/SelectionPanel.tsx"),
                    language: "tsx".to_string(),
                    lines: 1,
                    hash: String::new(),
                    size: 0,
                },
            ],
            packages: Vec::new(),
            symbols: Vec::new(),
            dependencies: Vec::new(),
            areas: Vec::new(),
            git: GitInfo {
                branch: "main".to_string(),
                rev: "abc".to_string(),
                dirty: true,
                changed: vec![
                    GitChange {
                        status: "M".to_string(),
                        path: PathBuf::from("src/app/MainEditorWindow.tsx"),
                        file_id: Some("f1".to_string()),
                    },
                    GitChange {
                        status: "D".to_string(),
                        path: PathBuf::from("src/app/workspacePanels.tsx"),
                        file_id: None,
                    },
                ],
            },
        };

        let report = build_import_fix_report(root.as_path(), &map, true, 20)
            .expect("import fix report should build");
        let rendered = crate::report::file_ops::model::render_report(&report);
        assert!(rendered.contains("stale imports:"));
        assert!(rendered.contains("./workspacePanels"));
        assert!(rendered.contains("broken imports:"));
        assert!(rendered.contains("./SelectionPanel"));
        assert!(rendered.contains("candidate: src/features/SelectionPanel.tsx"));
    }

    fn temp_root(name: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time should advance")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("amigo-codemap-{name}-{unique}"));
        std::fs::create_dir_all(&root).expect("create temp root");
        root
    }
}
