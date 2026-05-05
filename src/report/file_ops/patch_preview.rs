use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Read};
use std::path::Path;
use std::fmt::Write as _;

use anyhow::Result;

use crate::model::CodeMap;
use crate::report::common::{feature_group, slash_path};
use crate::report::verify_plan::plan_for_paths;

use super::diff::parse_patch_files;
use super::model::{render_report, FileOpReport, NextAction, Risk, RiskLevel};

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

    print!("{}", render_patch_preview_with_source(map, &text, limit, from.is_some()));
    let _ = root;
    Ok(())
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn render_patch_preview(map: &CodeMap, input: &str, limit: usize) -> String {
    render_patch_preview_with_source(map, input, limit, true)
}

fn render_patch_preview_with_source(
    map: &CodeMap,
    input: &str,
    limit: usize,
    from_file: bool,
) -> String {
    let report = build_patch_preview_report(map, input, limit, from_file);
    render_report(&report)
}

fn build_patch_preview_report(
    map: &CodeMap,
    input: &str,
    limit: usize,
    from_file: bool,
) -> FileOpReport {
    let files = parse_patch_files(input);
    let touched = files
        .iter()
        .map(|file| std::path::PathBuf::from(&file.new_path))
        .collect::<Vec<_>>();
    let plan = plan_for_paths(touched.clone());

    let mut area_counts = BTreeMap::<String, usize>::new();
    let mut symbols = BTreeSet::<String>::new();

    for file in files.iter().take(limit) {
        *area_counts
            .entry(patch_area(&file.new_path))
            .or_default() += 1;

        if let Some(entry) = map
            .files
            .iter()
            .find(|candidate| slash_path(&candidate.path) == file.new_path)
        {
            let file_symbols = map
                .symbols
                .iter()
                .filter(|symbol| symbol.file_id == entry.id)
                .collect::<Vec<_>>();
            for added_line in &file.added_lines {
                if let Some(symbol) = nearest_symbol(&file_symbols, *added_line) {
                    symbols.insert(symbol.name.clone());
                }
            }
        }
        if file.old_path != file.new_path {
            if let Some(rename) = format_rename(&file.old_path, &file.new_path) {
                symbols.insert(rename);
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

    FileOpReport {
        task: "patch-preview".to_string(),
        scope: vec![format!(
            "files: {}",
            if from_file { "from file" } else { "stdin" }
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
    }
}

fn nearest_symbol<'a>(
    symbols: &[&'a crate::model::SymbolEntry],
    line: usize,
) -> Option<&'a crate::model::SymbolEntry> {
    symbols
        .iter()
        .copied()
        .filter(|symbol| symbol.line <= line)
        .max_by_key(|symbol| symbol.line)
        .or_else(|| symbols.iter().copied().min_by_key(|symbol| symbol.line))
}

fn patch_area(path: &str) -> String {
    let path = path.replace('\\', "/");
    if path.contains("/app/store/") {
        "app/store".to_string()
    } else if path.contains("/main-window/") {
        "main-window".to_string()
    } else if path.contains("/properties/") {
        "properties".to_string()
    } else if path.contains("/src-tauri/") {
        "tauri".to_string()
    } else if path.contains("/amigo-codemap/") {
        "codemap".to_string()
    } else if path.ends_with(".md") || path.contains("/docs/") {
        "docs".to_string()
    } else {
        feature_group(&path)
    }
}

fn format_rename(old_path: &str, new_path: &str) -> Option<String> {
    (old_path != new_path).then(|| {
        let mut line = String::new();
        write!(line, "rename {} -> {}", old_path, new_path).unwrap();
        line
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use crate::model::{CodeMap, FileEntry, GitInfo, SymbolEntry};

    use super::render_patch_preview;

    fn sample_map() -> CodeMap {
        CodeMap {
            root_name: "repo".to_string(),
            stats: BTreeMap::new(),
            files: vec![
                FileEntry {
                    id: "f1".to_string(),
                    path: PathBuf::from("crates/apps/amigo-editor/src/app/store/editorReducer.ts"),
                    language: "ts".to_string(),
                    lines: 40,
                    hash: String::new(),
                    size: 0,
                },
                FileEntry {
                    id: "f2".to_string(),
                    path: PathBuf::from("crates/apps/amigo-editor/src/main-window/MainEditorWindow.tsx"),
                    language: "tsx".to_string(),
                    lines: 30,
                    hash: String::new(),
                    size: 0,
                },
            ],
            packages: Vec::new(),
            symbols: vec![
                SymbolEntry {
                    name: "reducer".to_string(),
                    kind: "fn".to_string(),
                    file_id: "f1".to_string(),
                    line: 10,
                    visibility: "export".to_string(),
                },
                SymbolEntry {
                    name: "MainEditorWindow".to_string(),
                    kind: "component".to_string(),
                    file_id: "f2".to_string(),
                    line: 5,
                    visibility: "export".to_string(),
                },
            ],
            dependencies: Vec::new(),
            areas: Vec::new(),
            git: GitInfo {
                branch: "main".to_string(),
                rev: "abc".to_string(),
                dirty: true,
                changed: Vec::new(),
            },
        }
    }

    #[test]
    fn snapshot_patch_preview() {
        let patch = r#"diff --git a/crates/apps/amigo-editor/src/app/store/editorReducer.ts b/crates/apps/amigo-editor/src/app/store/editorReducer.ts
@@ -10,0 +11,2 @@
+export const x = 1;
diff --git a/crates/apps/amigo-editor/src/main-window/MainEditorWindow.tsx b/crates/apps/amigo-editor/src/main-window/MainEditorWindow.tsx
@@ -4,0 +5,2 @@
+const view = true;
"#;
        assert_eq!(
            render_patch_preview(&sample_map(), patch, 80).trim(),
            include_str!("../../../tests/snapshots/patch_preview.snap").trim()
        );
    }
}
