#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::model::{CodeMap, FileEntry, GitChange, PackageEntry, SymbolEntry};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextRef {
    pub path: String,
    pub file_id: String,
    pub lines: Vec<(usize, String)>,
    pub changed: bool,
}

pub fn files_by_id(map: &CodeMap) -> BTreeMap<&str, &FileEntry> {
    map.files
        .iter()
        .map(|file| (file.id.as_str(), file))
        .collect()
}

pub fn files_by_path(map: &CodeMap) -> BTreeMap<String, &FileEntry> {
    map.files
        .iter()
        .map(|file| (slash_path(&file.path), file))
        .collect()
}

pub fn changed_by_path(map: &CodeMap) -> BTreeMap<String, &GitChange> {
    map.git
        .changed
        .iter()
        .map(|change| (slash_path(&change.path), change))
        .collect()
}

pub fn changed_by_file_id(map: &CodeMap) -> BTreeSet<String> {
    map.git
        .changed
        .iter()
        .filter_map(|change| change.file_id.clone())
        .collect()
}

pub fn symbols_by_file_id(map: &CodeMap) -> BTreeMap<&str, Vec<&SymbolEntry>> {
    let mut symbols = BTreeMap::<&str, Vec<&SymbolEntry>>::new();
    for symbol in &map.symbols {
        symbols.entry(&symbol.file_id).or_default().push(symbol);
    }
    symbols
}

pub fn symbols_matching<'a>(map: &'a CodeMap, query: &str) -> Vec<&'a SymbolEntry> {
    let query_lower = query.to_ascii_lowercase();
    map.symbols
        .iter()
        .filter(|symbol| {
            symbol.name == query || symbol.name.to_ascii_lowercase().contains(&query_lower)
        })
        .collect()
}

pub fn text_refs(root: &Path, map: &CodeMap, needle: &str, limit: usize) -> Result<Vec<TextRef>> {
    let changed = changed_by_path(map);
    let needle_lower = needle.to_ascii_lowercase();
    let mut refs = Vec::new();
    for file in &map.files {
        if refs.len() >= limit {
            break;
        }
        let Ok(text) = fs::read_to_string(root.join(&file.path)) else {
            continue;
        };
        let lines = text
            .lines()
            .enumerate()
            .filter_map(|(index, line)| {
                line.to_ascii_lowercase()
                    .contains(&needle_lower)
                    .then(|| (index + 1, trim_line(line)))
            })
            .collect::<Vec<_>>();
        if lines.is_empty() {
            continue;
        }
        let path = slash_path(&file.path);
        refs.push(TextRef {
            changed: changed.contains_key(&path),
            path,
            file_id: file.id.clone(),
            lines,
        });
    }
    Ok(refs)
}

pub fn group_path(path: &Path, group: Option<&str>) -> String {
    let path_text = slash_path(path);
    if group == Some("feature") {
        return feature_group(&path_text);
    }
    let parts = path_text.split('/').collect::<Vec<_>>();
    if parts.first() == Some(&"crates") && parts.len() >= 3 {
        if parts.get(1) == Some(&"apps") || parts.get(1) == Some(&"tools") {
            return parts.iter().take(3).copied().collect::<Vec<_>>().join("/");
        }
        return parts.iter().take(2).copied().collect::<Vec<_>>().join("/");
    }
    parts.first().copied().unwrap_or("root").to_string()
}

pub fn feature_group(path: &str) -> String {
    let path = path.replace('\\', "/");
    let rules = [
        ("src/app/store/", "app/store"),
        ("src/app/selection", "app/selection"),
        ("src/app/", "app"),
        ("src/main-window/", "main-window"),
        ("src/window-bus/", "window-bus"),
        ("src/startup/", "startup"),
        ("src/features/assets/", "feature/assets"),
        ("src/features/cache/", "feature/cache"),
        ("src/features/diagnostics/", "feature/diagnostics"),
        ("src/features/events/", "feature/events"),
        ("src/features/project/", "feature/project"),
        ("src/features/files/", "feature/files"),
        ("src/features/inspector/", "feature/inspector"),
        ("src/features/scenes/", "feature/scenes"),
        ("src/features/scripting/", "feature/scripting"),
        ("src/features/tasks/", "feature/tasks"),
        ("src/properties/", "properties"),
        ("src/settings/", "settings"),
        ("src/components/", "shared/components"),
        ("src/editor-components/", "editor-components"),
        ("src-tauri/", "tauri"),
    ];
    if let Some(group) = rules
        .iter()
        .find_map(|(needle, group)| path.contains(needle).then(|| (*group).to_string()))
    {
        return group;
    }
    if let Some(rest) = path.strip_prefix("crates/apps/amigo-editor/src/") {
        return rest.split('/').next().unwrap_or("src").to_string();
    }
    group_path(Path::new(&path), Some("path"))
}

pub fn package_for_file(map: &CodeMap, file: &FileEntry) -> Option<String> {
    package_for_path(map, &file.path)
}

pub fn area_for_file(map: &CodeMap, file: &FileEntry) -> Option<String> {
    map.areas
        .iter()
        .find(|area| area.files.iter().any(|id| id == &file.id))
        .map(|area| area.name.clone())
}

pub fn package_for_path(map: &CodeMap, path: &Path) -> Option<String> {
    let path = slash_path(path);
    map.packages
        .iter()
        .filter_map(|package| {
            let prefix = package_prefix(package);
            path.starts_with(&prefix)
                .then(|| (prefix.len(), package.name.clone()))
        })
        .max_by_key(|(len, _)| *len)
        .map(|(_, name)| name)
}

pub fn package_prefix(package: &PackageEntry) -> String {
    slash_path(
        package
            .manifest_path
            .parent()
            .unwrap_or_else(|| Path::new("")),
    )
}

pub fn is_editor_frontend(path: &Path) -> bool {
    let path = slash_path(path);
    path.starts_with("crates/apps/amigo-editor/src/") && (path.ends_with(".ts") || path.ends_with(".tsx"))
}

pub fn is_editor_tauri(path: &Path) -> bool {
    let path = slash_path(path);
    path.starts_with("crates/apps/amigo-editor/src-tauri/") && path.ends_with(".rs")
}

pub fn is_codemap(path: &Path) -> bool {
    slash_path(path).starts_with("crates/tools/amigo-codemap/")
}

pub fn is_docs(path: &Path) -> bool {
    let path = slash_path(path);
    path.ends_with(".md") || path.ends_with(".txt") || path.starts_with("docs/") || path.contains("/docs/")
}

pub fn is_rust_source(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "rs")
}

pub fn is_ts_source(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "ts" || ext == "tsx")
}

pub fn is_test_file(path: &Path) -> bool {
    let path = slash_path(path);
    path.contains(".test.") || path.contains(".spec.") || path.contains("/tests/")
}

pub fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub fn trim_line(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.len() > 140 {
        format!("{}...", &trimmed[..140])
    } else {
        trimmed.to_string()
    }
}

pub fn read_stdin() -> Result<String> {
    use std::io::Read;
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    Ok(input)
}

pub fn read_to_string(path: Option<&PathBuf>) -> Result<String> {
    match path {
        Some(path) => Ok(fs::read_to_string(path)?),
        None => read_stdin(),
    }
}

pub fn print_next(items: &[&str]) {
    println!("next:");
    for (index, item) in items.iter().enumerate() {
        println!("  {}. {item}", index + 1);
    }
}

pub fn sorted_counts(counts: BTreeMap<String, usize>) -> Vec<(String, usize)> {
    let mut counts = counts.into_iter().collect::<Vec<_>>();
    counts.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    counts
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{feature_group, is_editor_frontend, is_editor_tauri};

    #[test]
    fn groups_editor_store_as_app_store() {
        assert_eq!(
            feature_group("crates/apps/amigo-editor/src/app/store/editorReducer.ts"),
            "app/store"
        );
    }

    #[test]
    fn groups_editor_src_fallback_to_first_src_folder() {
        assert_eq!(
            feature_group("crates/apps/amigo-editor/src/foo/bar.ts"),
            "foo"
        );
    }

    #[test]
    fn detects_editor_frontend_ts() {
        assert!(is_editor_frontend(&PathBuf::from(
            "crates/apps/amigo-editor/src/app/editorStore.tsx"
        )));
    }

    #[test]
    fn detects_editor_tauri_rs() {
        assert!(is_editor_tauri(&PathBuf::from(
            "crates/apps/amigo-editor/src-tauri/src/lib.rs"
        )));
    }
}
