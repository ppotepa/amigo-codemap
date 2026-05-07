use std::fs;
use std::io::Read;
use std::path::Path;

use anyhow::Result;

use crate::model::FileEntry;

const IGNORED_DIRS: &[&str] = &[
    ".git",
    ".amigo",
    ".cache",
    ".cargo",
    ".idea",
    ".vscode",
    "build",
    "dist",
    "node_modules",
    "out",
    "target",
];

const IGNORED_EXTS: &[&str] = &[
    "gif", "jpg", "jpeg", "map", "min.js", "png", "svg", "wasm", "webp", "zip",
];

pub fn scan_files(root: &Path) -> Result<Vec<FileEntry>> {
    let mut entries = Vec::new();
    scan_dir(root, root, &mut entries)?;
    Ok(entries)
}

fn scan_dir(root: &Path, dir: &Path, entries: &mut Vec<FileEntry>) -> Result<()> {
    let mut children = fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
    children.sort_by_key(|entry| entry.path());

    for child in children {
        let path = child.path();
        let metadata = match child.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        if metadata.is_dir() {
            if should_ignore_dir(&path) {
                continue;
            }
            scan_dir(root, &path, entries)?;
        } else if metadata.is_file() && should_index_file(&path) {
            if let Some(entry) = read_file_entry(root, &path, metadata.len())? {
                entries.push(entry);
            }
        }
    }

    Ok(())
}

fn read_file_entry(root: &Path, path: &Path, size: u64) -> Result<Option<FileEntry>> {
    let mut file = fs::File::open(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    if bytes.contains(&0) {
        return Ok(None);
    }
    let text = String::from_utf8_lossy(&bytes);
    let lines = text.lines().count();
    let hash = short_hash(&bytes);
    let relative = path.strip_prefix(root).unwrap_or(path).to_path_buf();
    let language = language_for(path);

    let tags = classify_file_tags(
        path.strip_prefix(root).unwrap_or(path),
        &language,
        lines,
        size,
    );
    Ok(Some(FileEntry {
        id: String::new(),
        path: relative,
        language,
        lines,
        hash,
        size,
        tags,
    }))
}

pub fn language_for(path: &Path) -> String {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();
    if file_name == "Cargo.toml" {
        return "cargo".to_string();
    }
    if file_name == "package.json" {
        return "package".to_string();
    }
    if file_name.ends_with(".min.js") {
        return "minjs".to_string();
    }
    path.extension()
        .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_else(|| "txt".to_string())
}

fn should_ignore_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| IGNORED_DIRS.contains(&name))
        .unwrap_or(false)
}

fn should_index_file(path: &Path) -> bool {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if file_name == "cargo.lock" || file_name.ends_with(".lock") {
        return false;
    }
    if IGNORED_EXTS.iter().any(|ext| file_name.ends_with(ext)) {
        return false;
    }

    matches!(
        language_for(path).as_str(),
        "rs" | "ts"
            | "tsx"
            | "js"
            | "jsx"
            | "json"
            | "toml"
            | "cargo"
            | "package"
            | "yaml"
            | "yml"
            | "rhai"
            | "md"
            | "css"
            | "html"
    )
}

fn short_hash(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")[..8].to_string()
}

pub fn classify_file_tags(path: &Path, language: &str, lines: usize, size: u64) -> Vec<String> {
    let path_text = slash_path(path);
    let mut tags = Vec::<String>::new();

    push_tag(&mut tags, &format!("lang:{language}"));
    push_tag(&mut tags, &format!("ext:{language}"));
    push_tag(&mut tags, classify_layer(&path_text));
    push_tag(&mut tags, classify_kind(&path_text, language));

    for domain in classify_domains(&path_text) {
        push_tag(&mut tags, &format!("domain:{domain}"));
    }

    if lines > 500 || size > 80_000 {
        push_tag(&mut tags, "risk:large");
    }

    tags.sort();
    tags.dedup();
    tags
}

fn classify_layer(path: &str) -> &'static str {
    if path.starts_with("crates/apps/") {
        "layer:app"
    } else if path.starts_with("crates/tools/") {
        "layer:tool"
    } else if path.starts_with("crates/engine/") {
        "layer:engine"
    } else if path.starts_with("crates/ui/") {
        "layer:ui"
    } else if path.starts_with("mods/") {
        "layer:mod"
    } else if path.ends_with(".md") || path.starts_with("docs/") {
        "layer:docs"
    } else {
        "layer:root"
    }
}

fn classify_kind(path: &str, language: &str) -> &'static str {
    if path.contains("/tests/") || path.contains(".test.") || path.contains(".spec.") {
        "kind:test"
    } else if path.contains("/fixtures/") {
        "kind:fixture"
    } else if language == "css" {
        "kind:style"
    } else if matches!(
        language,
        "yaml" | "yml" | "toml" | "json" | "cargo" | "package"
    ) {
        "kind:config"
    } else if path.ends_with(".md") {
        "kind:docs"
    } else {
        "kind:source"
    }
}

fn classify_domains(path: &str) -> Vec<&'static str> {
    let mut domains = Vec::new();
    let rules = [
        ("src/main-window/", "workspace"),
        ("src/dock/", "dock"),
        ("src/features/scenes/", "scenes"),
        ("src/features/assets/", "assets"),
        ("src/features/files/", "files"),
        ("src/features/project/", "project"),
        ("src/features/inspector/", "inspector"),
        ("src/properties/", "properties"),
        ("src/startup/", "startup"),
        ("src-tauri/src/commands/", "tauri-commands"),
        ("editor_mode", "editor-mode"),
        ("ui_document", "ui-document"),
        ("ui-document", "ui-document"),
        ("crates/tools/amigo-codemap/", "codemap"),
    ];
    for (needle, domain) in rules {
        if path.contains(needle) {
            domains.push(domain);
        }
    }
    domains
}

fn push_tag(tags: &mut Vec<String>, tag: &str) {
    if !tag.is_empty() && !tags.iter().any(|item| item == tag) {
        tags.push(tag.to_string());
    }
}

fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::{classify_file_tags, language_for};
    use std::path::Path;

    #[test]
    fn maps_known_languages() {
        assert_eq!(language_for(Path::new("src/lib.rs")), "rs");
        assert_eq!(language_for(Path::new("src/App.tsx")), "tsx");
        assert_eq!(language_for(Path::new("Cargo.toml")), "cargo");
    }

    #[test]
    fn classifies_codemap_tool_file() {
        let tags = classify_file_tags(
            Path::new("crates/tools/amigo-codemap/src/main.rs"),
            "rs",
            100,
            10_000,
        );
        assert!(tags.contains(&"layer:tool".to_string()));
        assert!(tags.contains(&"domain:codemap".to_string()));
        assert!(tags.contains(&"kind:source".to_string()));
    }
}
