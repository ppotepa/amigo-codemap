use std::cmp::Reverse;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use regex::Regex;

use crate::model::{CodeMap, FileEntry};
use crate::report::common::{area_for_file, package_for_file, slash_path};

use super::common::{find_file_by_path, import_block, read_text_at_root, symbols_in_file};
use super::model::{FileOpReport, NextAction, Risk, RiskLevel, print_report};

#[derive(Debug, Clone)]
struct AppendAnchor {
    line: usize,
    label: String,
    detail: String,
}

pub fn print_append_plan(
    root: &Path,
    map: &CodeMap,
    query: &str,
    task: Option<&str>,
    limit: usize,
) -> Result<()> {
    if query.trim().is_empty() {
        bail!("append-plan requires a file path");
    }

    let file = find_file_by_path(map, query)
        .ok_or_else(|| anyhow::anyhow!("append-plan could not find target file `{query}`"))?;
    let text = read_text_at_root(root, &file.path)?;
    let lines = text.lines().count().max(1);
    let task = task.unwrap_or("generic");
    let scope = build_scope(map, file, task);
    let findings = build_findings(root, map, file, &text, task, limit);
    let risks = build_risks(file, task, lines);
    let verify = verify_steps(file);
    let next = build_next(task);

    print_report(&FileOpReport {
        task: format!("append-plan {}", slash_path(&file.path)),
        scope,
        findings,
        risks,
        verify,
        next,
    });

    Ok(())
}

fn build_scope(map: &CodeMap, file: &FileEntry, task: &str) -> Vec<String> {
    let mut scope = vec![
        format!("target: {}", slash_path(&file.path)),
        format!("language: {}", file.language),
        format!("task: {task}"),
        format!("lines: {}", file.lines),
    ];
    if let Some(package) = package_for_file(map, file) {
        scope.push(format!("package: {package}"));
    }
    if let Some(area) = area_for_file(map, file) {
        scope.push(format!("area: {area}"));
    }
    scope
}

fn build_findings(
    root: &Path,
    map: &CodeMap,
    file: &FileEntry,
    text: &str,
    task: &str,
    limit: usize,
) -> Vec<String> {
    let mut findings = Vec::new();
    let anchors = detect_anchors(map, file, text, task);
    findings.push("append anchors:".to_string());
    for anchor in anchors.iter().take(limit.max(3)) {
        findings.push(format!(
            "  line {}: {} [{}]",
            anchor.line, anchor.label, anchor.detail
        ));
    }

    findings.push("symbol context:".to_string());
    let symbols = symbols_in_file(map, &file.id);
    if symbols.is_empty() {
        findings.push("  none".to_string());
    } else {
        for symbol in symbols.iter().take(6) {
            findings.push(format!("  {} {} {}", symbol.kind, symbol.name, symbol.line));
        }
    }

    let donors = donor_candidates(map, file, limit);
    findings.push("donor candidates:".to_string());
    if donors.is_empty() {
        findings.push("  none".to_string());
    } else {
        for donor in donors {
            findings.push(format!("  {}", slash_path(&donor.path)));
        }
    }

    let companion = companion_files(root, file, task);
    findings.push("companion files:".to_string());
    if companion.is_empty() {
        findings.push("  none".to_string());
    } else {
        for item in companion {
            findings.push(format!("  {item}"));
        }
    }

    findings
}

fn detect_anchors(map: &CodeMap, file: &FileEntry, text: &str, task: &str) -> Vec<AppendAnchor> {
    let mut anchors = Vec::new();
    let line_count = text.lines().count().max(1);
    if let Some((line, _)) = import_block(text).last() {
        anchors.push(AppendAnchor {
            line: *line,
            label: "after import block".to_string(),
            detail: "safe for new imports or module-level helpers".to_string(),
        });
    }

    if let Some(symbol) = symbols_in_file(map, &file.id)
        .into_iter()
        .max_by_key(|symbol| symbol.line)
    {
        anchors.push(AppendAnchor {
            line: symbol.line,
            label: format!("after last symbol `{}`", symbol.name),
            detail: "good for new top-level declarations".to_string(),
        });
    }

    anchors.extend(task_specific_anchors(text, task));

    anchors.push(AppendAnchor {
        line: line_count,
        label: "before file end".to_string(),
        detail: "append at file end when no better structural anchor exists".to_string(),
    });

    anchors.sort_by_key(|anchor| (anchor.line, anchor.label.clone()));
    anchors.dedup_by(|left, right| left.line == right.line && left.label == right.label);
    anchors
}

fn task_specific_anchors(text: &str, task: &str) -> Vec<AppendAnchor> {
    let mut anchors = Vec::new();
    let task = task.to_ascii_lowercase();

    if task.contains("component") || task.contains("registry") || task.contains("array") {
        if let Some(anchor) = array_close_anchor(text, "EditorComponentDefinition[]") {
            anchors.push(anchor);
        }
        if let Some(anchor) = array_close_anchor(text, "rawBuiltinEditorComponents") {
            anchors.push(anchor);
        }
        if let Some(anchor) = array_close_anchor(text, "components") {
            anchors.push(anchor);
        }
    }

    if task.contains("route") {
        if let Some(anchor) = before_default_case(text) {
            anchors.push(anchor);
        }
    }

    if task.contains("style") || task.contains("css") {
        anchors.push(AppendAnchor {
            line: text.lines().count().max(1),
            label: "after last selector block".to_string(),
            detail: "prefer appending new rule at the end of the stylesheet".to_string(),
        });
    }

    if task.contains("test") {
        if let Some(line) = last_test_anchor(text) {
            anchors.push(AppendAnchor {
                line,
                label: "after last test block".to_string(),
                detail: "keeps new scenario close to existing tests".to_string(),
            });
        }
    }

    anchors
}

fn array_close_anchor(text: &str, needle: &str) -> Option<AppendAnchor> {
    let escaped = regex::escape(needle);
    let declaration = Regex::new(&format!(r"{escaped}[^\n]*=\s*\[")).ok()?;
    let mut seen = false;
    for (index, line) in text.lines().enumerate() {
        if declaration.is_match(line) || line.contains(needle) {
            seen = true;
            continue;
        }
        if seen && line.trim() == "];" {
            return Some(AppendAnchor {
                line: index + 1,
                label: format!("before `{needle}` array close"),
                detail: "append new item before the collection terminator".to_string(),
            });
        }
    }
    None
}

fn before_default_case(text: &str) -> Option<AppendAnchor> {
    text.lines().enumerate().find_map(|(index, line)| {
        line.trim_start()
            .starts_with("default:")
            .then_some(AppendAnchor {
                line: index + 1,
                label: "before switch default case".to_string(),
                detail: "append new route/branch before the default branch".to_string(),
            })
    })
}

fn last_test_anchor(text: &str) -> Option<usize> {
    let lines = text.lines().collect::<Vec<_>>();
    lines.iter().enumerate().rev().find_map(|(index, line)| {
        let trimmed = line.trim_start();
        (trimmed.starts_with("it(")
            || trimmed.starts_with("test(")
            || trimmed.starts_with("#[test]"))
        .then_some(index + 1)
    })
}

fn donor_candidates<'a>(map: &'a CodeMap, file: &FileEntry, limit: usize) -> Vec<&'a FileEntry> {
    let directory = file.path.parent().map(PathBuf::from);
    let extension = file
        .path
        .extension()
        .map(|ext| ext.to_string_lossy().to_string());
    let file_name = file
        .path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_default();

    let mut candidates = map
        .files
        .iter()
        .filter(|candidate| candidate.id != file.id)
        .filter(|candidate| candidate.path.parent().map(|path| path.to_path_buf()) == directory)
        .filter(|candidate| {
            candidate
                .path
                .extension()
                .map(|ext| ext.to_string_lossy().to_string())
                == extension
        })
        .collect::<Vec<_>>();

    candidates.sort_by_key(|candidate| {
        let candidate_stem = candidate
            .path
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
            .unwrap_or_default();
        (
            Reverse(shared_prefix_len(&file_name, &candidate_stem)),
            Reverse(candidate.lines),
            slash_path(&candidate.path),
        )
    });
    candidates.truncate(limit.min(5));
    candidates
}

fn companion_files(root: &Path, file: &FileEntry, task: &str) -> Vec<String> {
    let path = slash_path(&file.path);
    let mut items = Vec::new();
    let extension = file
        .path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default();

    if extension == "tsx" || extension == "ts" {
        let css = file.path.with_extension("css");
        if root.join(&css).exists() {
            items.push(slash_path(&css));
        }
    }

    if extension == "css" {
        for candidate in [
            file.path.with_extension("tsx"),
            file.path.with_extension("ts"),
        ] {
            if root.join(&candidate).exists() {
                items.push(slash_path(&candidate));
            }
        }
    }

    if path.ends_with("builtinComponents.tsx") {
        items.push("crates/apps/amigo-editor/src/editor-components/componentTypes.ts".to_string());
        items.push("crates/apps/amigo-editor/src/features/editorFeatures.ts".to_string());
    }

    if task.contains("route") || path.ends_with("App.tsx") {
        items.push("crates/apps/amigo-editor/src/App.tsx".to_string());
    }

    if task.contains("window") {
        items.push("crates/apps/amigo-editor/src/api/editorApi.ts".to_string());
    }

    items.sort();
    items.dedup();
    items
}

fn build_risks(file: &FileEntry, task: &str, line_count: usize) -> Vec<Risk> {
    let mut risks = vec![Risk {
        level: RiskLevel::Medium,
        message: "append at file end can bypass local registry or switch ordering".to_string(),
    }];

    if file.lines > 400 {
        risks.push(Risk {
            level: RiskLevel::Medium,
            message: "large file; prefer the narrowest anchor instead of blind EOF append"
                .to_string(),
        });
    }

    if task.contains("component") || task.contains("registry") {
        risks.push(Risk {
            level: RiskLevel::High,
            message: "registry-style files often need import + registration + docs in one edit"
                .to_string(),
        });
    }

    if line_count < 12 {
        risks.push(Risk {
            level: RiskLevel::Low,
            message: "small file; anchor ranking is coarse because there are few structural cues"
                .to_string(),
        });
    }

    risks
}

fn verify_steps(file: &FileEntry) -> Vec<String> {
    match file.language.as_str() {
        "tsx" | "ts" | "css" => vec!["npm run build".to_string(), "npm test".to_string()],
        "rs" => vec!["cargo test -p amigo-codemap".to_string()],
        _ => vec!["run the smallest affected check".to_string()],
    }
}

fn build_next(task: &str) -> Vec<NextAction> {
    let mut next = vec![
        NextAction {
            label: "pick the first structural anchor instead of appending blindly at EOF"
                .to_string(),
        },
        NextAction {
            label: "reuse one donor candidate if the change is mostly mechanical".to_string(),
        },
    ];

    if task.contains("component") || task.contains("registry") {
        next.push(NextAction {
            label: "check companion files for imports, registration, and styles".to_string(),
        });
    } else {
        next.push(NextAction {
            label: "run verify only for the touched surface".to_string(),
        });
    }

    next
}

fn shared_prefix_len(left: &str, right: &str) -> usize {
    left.chars()
        .zip(right.chars())
        .take_while(|(left, right)| left == right)
        .count()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::model::{CodeMap, FileEntry, GitInfo, SymbolEntry};

    use super::{array_close_anchor, before_default_case, detect_anchors, donor_candidates};

    #[test]
    fn finds_array_close_anchor() {
        let text = "const items: EditorComponentDefinition[] = [\n  foo,\n];\n";
        let anchor = array_close_anchor(text, "EditorComponentDefinition[]").expect("anchor");
        assert_eq!(anchor.line, 3);
    }

    #[test]
    fn finds_default_case_anchor() {
        let text = "switch (route) {\n  case \"a\": break;\n  default:\n}\n";
        let anchor = before_default_case(text).expect("default anchor");
        assert_eq!(anchor.line, 3);
    }

    #[test]
    fn includes_import_and_symbol_anchors() {
        let map = CodeMap {
            root_name: "repo".to_string(),
            stats: Default::default(),
            files: vec![FileEntry {
                id: "f1".to_string(),
                path: PathBuf::from("src/demo.tsx"),
                language: "tsx".to_string(),
                lines: 12,
                hash: "x".to_string(),
                size: 12,
                ..Default::default()
            }],
            packages: vec![],
            symbols: vec![SymbolEntry {
                name: "Demo".to_string(),
                kind: "component".to_string(),
                file_id: "f1".to_string(),
                line: 8,
                visibility: "export".to_string(),
                ..Default::default()
            }],
            dependencies: vec![],
            areas: vec![],
            git: GitInfo::default(),
            ..Default::default()
        };

        let anchors = detect_anchors(
            &map,
            &map.files[0],
            "import { X } from 'y';\n\nexport function Demo() {}\n",
            "generic",
        );
        assert!(
            anchors
                .iter()
                .any(|anchor| anchor.label.contains("import block"))
        );
        assert!(
            anchors
                .iter()
                .any(|anchor| anchor.label.contains("last symbol"))
        );
    }

    #[test]
    fn donor_candidates_stay_in_same_directory() {
        let map = CodeMap {
            root_name: "repo".to_string(),
            stats: Default::default(),
            files: vec![
                FileEntry {
                    id: "a".to_string(),
                    path: PathBuf::from("src/startup/StartupDialog.tsx"),
                    language: "tsx".to_string(),
                    lines: 100,
                    hash: "a".to_string(),
                    size: 100,
                    ..Default::default()
                },
                FileEntry {
                    id: "b".to_string(),
                    path: PathBuf::from("src/startup/ModsPanel.tsx"),
                    language: "tsx".to_string(),
                    lines: 60,
                    hash: "b".to_string(),
                    size: 60,
                    ..Default::default()
                },
                FileEntry {
                    id: "c".to_string(),
                    path: PathBuf::from("src/main-window/MainEditorWindow.tsx"),
                    language: "tsx".to_string(),
                    lines: 300,
                    hash: "c".to_string(),
                    size: 300,
                    ..Default::default()
                },
            ],
            packages: vec![],
            symbols: vec![],
            dependencies: vec![],
            areas: vec![],
            git: GitInfo::default(),
            ..Default::default()
        };

        let donors = donor_candidates(&map, &map.files[0], 4);
        assert_eq!(donors.len(), 1);
        assert_eq!(slash_path(&donors[0].path), "src/startup/ModsPanel.tsx");
    }

    fn slash_path(path: &Path) -> String {
        path.to_string_lossy().replace('\\', "/")
    }

    use std::path::Path;
}
