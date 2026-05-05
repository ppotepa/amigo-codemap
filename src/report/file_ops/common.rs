use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

use super::model::FileRef;
use crate::model::{CodeMap, FileEntry, SymbolEntry};

pub fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub fn read_text(path: &Path) -> Result<String> {
    Ok(fs::read_to_string(path)?)
}

pub fn read_text_at_root(root: &Path, relative: &Path) -> Result<String> {
    read_text(&root.join(relative))
}

pub fn text_refs_like(
    root: &Path,
    map: &CodeMap,
    query: &str,
    limit: usize,
) -> Result<Vec<FileRef>> {
    let mut refs = Vec::new();
    if query.is_empty() {
        return Ok(refs);
    }
    let changed = changed_by_path(map);
    let query = query.to_string();

    for file in &map.files {
        if refs.len() >= limit {
            break;
        }

        let path_text = slash_path(&file.path);
        let text = read_text_at_root(root, &file.path)?;
        for (line, content) in text.lines().enumerate() {
            if content.contains(&query) {
                refs.push(FileRef {
                    path: file.path.clone(),
                    line: line + 1,
                    text: trim_line(content),
                    changed: changed.contains(&path_text),
                });

                if refs.len() >= limit {
                    break;
                }
            }
        }
    }

    Ok(refs)
}

pub fn find_file_by_path<'a>(map: &'a CodeMap, query: &str) -> Option<&'a FileEntry> {
    let normalized = query.replace('\\', "/");
    map.files.iter().find(|file| {
        let path = slash_path(&file.path);
        path == normalized || path.ends_with(&normalized)
    })
}

pub fn symbols_in_file<'a>(map: &'a CodeMap, file_id: &str) -> Vec<&'a SymbolEntry> {
    map.symbols
        .iter()
        .filter(|symbol| symbol.file_id == file_id)
        .collect()
}

pub fn changed_by_path(map: &CodeMap) -> BTreeSet<String> {
    map.git
        .changed
        .iter()
        .map(|change| slash_path(&change.path))
        .collect()
}

pub fn changed_status_by_path(map: &CodeMap) -> BTreeMap<String, String> {
    map.git
        .changed
        .iter()
        .map(|change| (slash_path(&change.path), change.status.clone()))
        .collect()
}

pub fn changed_status<'a>(
    changed: &'a BTreeMap<String, String>,
    path: &Path,
) -> Option<&'a str> {
    changed.get(&slash_path(path)).map(String::as_str)
}

pub fn is_changed(map: &CodeMap, path: &Path) -> bool {
    changed_by_path(map).contains(&slash_path(path))
}

pub fn line_window(text: &str, center: usize, radius: usize) -> Vec<(usize, String)> {
    let start = center.saturating_sub(radius).max(1);
    let end = center + radius;
    text.lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let line_no = index + 1;
            (line_no >= start && line_no <= end).then_some((line_no, line.to_string()))
        })
        .collect()
}

pub fn import_block(text: &str) -> Vec<(usize, String)> {
    text.lines()
        .enumerate()
        .take_while(|(_, line)| {
            let trimmed = line.trim();
            trimmed.is_empty()
                || trimmed.starts_with("import ")
                || trimmed.starts_with("use ")
                || trimmed.starts_with("mod ")
                || trimmed.starts_with("pub mod ")
        })
        .filter_map(|(index, line)| {
            let trimmed = line.trim();
            (!trimmed.is_empty()).then_some((index + 1, line.to_string()))
        })
        .collect()
}

pub fn resolve_relative_import(
    root: &Path,
    source_file: &Path,
    specifier: &str,
) -> Option<PathBuf> {
    if !specifier.starts_with('.') {
        return None;
    }

    let source = root.join(source_file);
    let source_dir = source.parent()?;
    let base = source_dir.join(specifier);

    let candidates = vec![
        base.clone(),
        base.with_extension("ts"),
        base.with_extension("tsx"),
        base.join("index.ts"),
        base.join("index.tsx"),
    ];

    let normalized_root = root.canonicalize().ok()?;
    candidates
        .into_iter()
        .find(|candidate| candidate.exists())
        .and_then(|candidate| {
            candidate
                .canonicalize()
                .ok()
                .as_ref()
                .and_then(|canonical| canonical.strip_prefix(&normalized_root).ok())
                .map(PathBuf::from)
                .or_else(|| {
                    candidate
                .strip_prefix(&normalized_root)
                .ok()
                .map(PathBuf::from)
                })
        })
}

pub fn relative_module_from_paths(root: &Path, from: &Path, to: &Path) -> Option<String> {
    let from_path = if from.is_absolute() {
        from.strip_prefix(root).ok()?.to_path_buf()
    } else {
        from.to_path_buf()
    };
    let to_path = if to.is_absolute() {
        to.strip_prefix(root).ok()?.to_path_buf()
    } else {
        to.to_path_buf()
    };

    let from_dir = from_path.parent()?;
    let mut from_parts = from_dir
        .iter()
        .map(|component| component.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    let mut to_parts = to_path
        .iter()
        .map(|component| component.to_string_lossy().to_string())
        .collect::<Vec<_>>();

    if let Some(last) = to_parts.last() {
        if last == "index.ts" || last == "index.tsx" {
            to_parts.pop();
        }
    }
    if let Some(last) = to_parts.last()
        && (last == "index.ts" || last == "index.tsx")
    {
        to_parts.pop();
    }

    while !from_parts.is_empty() && !to_parts.is_empty() {
        if from_parts[0] == to_parts[0] {
            from_parts.remove(0);
            to_parts.remove(0);
        } else {
            break;
        }
    }

    if from_parts.is_empty() && to_parts.is_empty() {
        return Some("./".to_string());
    }

    let mut relative = String::new();
    for _ in 0..from_parts.len() {
        relative.push_str("../");
    }
    let tail = to_parts
        .iter()
        .map(|component| component.as_str())
        .collect::<Vec<_>>()
        .join("/");
    relative.push_str(&tail);

    if to_parts.is_empty() {
        return None;
    }

    let mut out = relative
        .trim_end_matches('/')
        .trim_end_matches(".ts")
        .trim_end_matches(".tsx")
        .trim_end_matches('/')
        .to_string();
    if out.is_empty() {
        out.push('.');
        out.push('/');
    } else if !out.starts_with('.') {
        out = format!("./{out}");
    }
    Some(out.replace('\\', "/"))
}

fn trim_line(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.len() > 160 {
        format!("{}...", &trimmed[..160])
    } else {
        trimmed.to_string()
    }
}
