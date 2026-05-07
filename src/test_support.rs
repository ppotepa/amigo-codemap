#![cfg(test)]
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::model::{
    AreaEntry, CodeMap, CodemapTagEntry, DependencyEntry, FileEntry, GitInfo, PackageEntry,
    RelationEntry, SymbolEntry, TextOccurrenceEntry,
};

pub fn test_file(id: &str, path: &str) -> FileEntry {
    let language = language_from_path(path);

    FileEntry {
        id: id.to_string(),
        path: PathBuf::from(path),
        language: language.to_string(),
        lines: 10,
        hash: "testhash".to_string(),
        size: 100,
        tags: vec![format!("lang:{language}"), "kind:source".to_string()],
    }
}

pub fn test_file_with_tags(id: &str, path: &str, tags: &[&str]) -> FileEntry {
    let mut file = test_file(id, path);
    for tag in tags {
        if !file.tags.iter().any(|existing| existing == tag) {
            file.tags.push((*tag).to_string());
        }
    }
    file.tags.sort();
    file
}

pub fn test_symbol(name: &str, file_id: &str, line: usize) -> SymbolEntry {
    SymbolEntry {
        name: name.to_string(),
        kind: "fn".to_string(),
        file_id: file_id.to_string(),
        line,
        line_end: line,
        line_count: 1,
        signature: format!("fn {name}()"),
        params: Vec::new(),
        return_type: None,
        generics: Vec::new(),
        visibility: "private".to_string(),
        owner: None,
        tags: vec!["kind:fn".to_string(), "visibility:private".to_string()],
        confidence: 80,
    }
}

pub fn test_symbol_with_kind(name: &str, kind: &str, file_id: &str, line: usize) -> SymbolEntry {
    let mut symbol = test_symbol(name, file_id, line);
    symbol.kind = kind.to_string();
    symbol.tags.retain(|tag| !tag.starts_with("kind:"));
    symbol.tags.push(format!("kind:{kind}"));
    symbol.tags.sort();
    symbol
}

pub fn test_symbol_with_range(
    name: &str,
    kind: &str,
    file_id: &str,
    line: usize,
    line_end: usize,
) -> SymbolEntry {
    let mut symbol = test_symbol_with_kind(name, kind, file_id, line);
    symbol.line_end = line_end;
    symbol.line_count = line_end.saturating_sub(line).saturating_add(1);
    symbol
}

pub fn test_occurrence(value: &str, kind: &str, file_id: &str, line: usize) -> TextOccurrenceEntry {
    TextOccurrenceEntry {
        id: format!("tx-{file_id}-{line}"),
        value: value.to_string(),
        normalized_value: value.to_ascii_lowercase(),
        kind: kind.to_string(),
        file_id: file_id.to_string(),
        line,
        column: 1,
        owner: None,
        context: value.to_string(),
        tags: vec![format!("kind:{kind}")],
        confidence: 70,
    }
}

pub fn test_map(files: Vec<FileEntry>, symbols: Vec<SymbolEntry>) -> CodeMap {
    CodeMap {
        root_name: "test".to_string(),
        stats: BTreeMap::new(),
        files,
        packages: Vec::<PackageEntry>::new(),
        symbols,
        text_occurrences: Vec::<TextOccurrenceEntry>::new(),
        tags: Vec::<CodemapTagEntry>::new(),
        dependencies: Vec::<DependencyEntry>::new(),
        relations: Vec::<RelationEntry>::new(),
        areas: Vec::<AreaEntry>::new(),
        git: GitInfo::default(),
    }
}

fn language_from_path(path: &str) -> &'static str {
    if path.ends_with(".rs") {
        "rs"
    } else if path.ends_with(".tsx") {
        "tsx"
    } else if path.ends_with(".ts") {
        "ts"
    } else if path.ends_with(".rhai") {
        "rhai"
    } else if path.ends_with(".yml") || path.ends_with(".yaml") {
        "yaml"
    } else if path.ends_with(".css") {
        "css"
    } else if path.ends_with(".toml") {
        "toml"
    } else {
        "txt"
    }
}
