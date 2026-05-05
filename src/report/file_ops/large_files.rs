use crate::model::CodeMap;
use crate::report::common::slash_path;

use super::model::{FileOpReport, NextAction};

pub fn print_large_files(map: &CodeMap, top: usize, with_split_hints: bool) {
    let changed = map
        .git
        .changed
        .iter()
        .filter_map(|change| change.file_id.as_deref())
        .collect::<std::collections::BTreeSet<_>>();

    let mut ranked = map
        .files
        .iter()
        .filter(|file| is_rankable_file(&slash_path(&file.path)))
        .map(|file| {
            let path = slash_path(&file.path);
            let symbol_count = map
                .symbols
                .iter()
                .filter(|symbol| symbol.file_id == file.id)
                .count();
            let import_count = 0usize;
            let is_changed = changed.contains(file.id.as_str());
            let score = split_score(&path, file.lines, symbol_count, import_count, is_changed);
            (file, path, symbol_count, is_changed, score)
        })
        .collect::<Vec<_>>();

    ranked.sort_by(|left, right| {
        right
            .4
            .cmp(&left.4)
            .then_with(|| right.0.lines.cmp(&left.0.lines))
    });

    let mut findings = vec!["top:".to_string()];
    for (file, path, symbol_count, is_changed, _) in ranked.into_iter().take(top) {
        findings.push(format!("  {path}"));
        findings.push(format!("    lines: {}", file.lines));
        findings.push(format!("    symbols: {symbol_count}"));
        findings.push(format!("    changed: {}", if is_changed { "yes" } else { "no" }));
        if with_split_hints {
            findings.push(format!("    split hints: {}", split_hint(&path)));
        }
    }

    super::model::print_report(&FileOpReport {
        task: "large-files".to_string(),
        scope: vec![format!("top: {top}")],
        findings,
        risks: Vec::new(),
        verify: vec!["npm run build".to_string()],
        next: vec![
            NextAction {
                label: "run move-plan on the top candidate".to_string(),
            },
            NextAction {
                label: "split by import or symbol cluster".to_string(),
            },
        ],
    });
}

fn is_rankable_file(path: &str) -> bool {
    let is_code = path.ends_with(".rs")
        || path.ends_with(".ts")
        || path.ends_with(".tsx")
        || path.ends_with(".css");
    if !is_code {
        return false;
    }

    !path.contains("/target/")
        && !path.contains("/dist/")
        && !path.contains("/build/")
        && !path.contains("package-lock.json")
        && !path.contains("/gen/schemas/")
}

fn split_score(
    path: &str,
    lines: usize,
    symbol_count: usize,
    import_count: usize,
    is_changed: bool,
) -> i32 {
    let mut score = lines.min(2000) as i32;
    score += (symbol_count as i32) * 10;
    score += (import_count as i32) * 2;
    if is_changed {
        score += 75;
    }
    if path.contains("/commands/") || path.contains("ProjectExplorer") || path.contains("builtinComponents") {
        score += 100;
    }
    if path.ends_with(".css") {
        score -= 150;
    }
    score
}

fn split_hint(path: &str) -> &'static str {
    if path.contains("/commands/") {
        "command domain"
    } else if path.contains("ProjectExplorer") {
        "tree/actions/node strip"
    } else if path.contains("builtinComponents") {
        "registry/panels"
    } else if path.ends_with("/loader.rs") {
        "dto/loading/validation/errors"
    } else if path.ends_with("/scanner.rs") {
        "scan/graph/io"
    } else {
        "symbols/import clusters"
    }
}

#[cfg(test)]
mod tests {
    use super::{is_rankable_file, split_hint};

    #[test]
    fn filters_generated_and_lockfiles() {
        assert!(!is_rankable_file("crates/apps/amigo-editor/package-lock.json"));
        assert!(!is_rankable_file("crates/apps/amigo-editor/src-tauri/gen/schemas/x.json"));
        assert!(is_rankable_file("crates/apps/amigo-editor/src/app/store/editorState.ts"));
    }

    #[test]
    fn maps_split_hints() {
        assert_eq!(split_hint("src-tauri/src/commands/mod.rs"), "command domain");
        assert_eq!(split_hint("src/features/project/ProjectExplorerPanel.tsx"), "tree/actions/node strip");
        assert_eq!(split_hint("src/editor-components/builtinComponents.tsx"), "registry/panels");
        assert_eq!(split_hint("src-tauri/src/sheet/loader.rs"), "dto/loading/validation/errors");
        assert_eq!(split_hint("src-tauri/src/asset_registry/scanner.rs"), "scan/graph/io");
    }
}
