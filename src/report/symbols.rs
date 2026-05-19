use std::path::Path;

use anyhow::{Result, bail};
use serde::Serialize;

use crate::model::{CodeMap, FileEntry, SymbolEntry};
use crate::query::Query;

#[derive(Debug, Clone, Serialize)]
struct SymbolRecord {
    path: String,
    name: String,
    kind: String,
    visibility: String,
    owner: Option<String>,
    line: usize,
    line_end: usize,
    body_open_line: Option<usize>,
    body_close_line: Option<usize>,
    signature: String,
    params: Vec<String>,
    return_type: Option<String>,
    generics: Vec<String>,
    confidence: u8,
}

#[derive(Debug, Clone, Serialize)]
struct SymbolListJsonFilters {
    path: Option<String>,
    name: Option<String>,
    kind: Option<String>,
    owner: Option<String>,
    visibility: Option<String>,
    changed_only: bool,
}

#[derive(Debug, Clone, Serialize)]
struct SymbolListJsonOutput {
    query: Option<String>,
    filters: SymbolListJsonFilters,
    count: usize,
    limit: usize,
    truncated: bool,
    items: Vec<SymbolRecord>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SymbolListFilters<'a> {
    pub file_filter: Option<&'a Path>,
    pub name: Option<&'a str>,
    pub kind: Option<&'a str>,
    pub owner: Option<&'a str>,
    pub visibility: Option<&'a str>,
    pub changed_only: bool,
    pub metadata: bool,
    pub json: bool,
}

pub fn print_symbols(
    map: &CodeMap,
    query: Option<&str>,
    filters: SymbolListFilters<'_>,
    limit: usize,
) -> Result<()> {
    let query = Query::parse(query);
    let changed_ids = map
        .git
        .changed
        .iter()
        .filter_map(|change| change.file_id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let files = map
        .files
        .iter()
        .map(|file| (file.id.as_str(), file))
        .collect::<std::collections::BTreeMap<_, _>>();
    let file_id_filter = match filters.file_filter {
        Some(path) => Some(resolve_file_id(map, path)?),
        None => None,
    };

    let mut matched = Vec::new();
    for symbol in &map.symbols {
        if filters.changed_only && !changed_ids.contains(&symbol.file_id) {
            continue;
        }
        if file_id_filter
            .as_deref()
            .is_some_and(|id| id != symbol.file_id)
        {
            continue;
        }
        if !matches_filters(symbol, &query, filters) {
            continue;
        }
        matched.push(symbol);
    }

    let truncated = matched.len() > limit;
    let selected = matched.into_iter().take(limit).collect::<Vec<_>>();
    let mut emitted = 0usize;
    let mut records = Vec::new();
    for symbol in selected {
        let file = files.get(symbol.file_id.as_str());
        if filters.json {
            records.push(symbol_record(symbol, file.copied()));
        } else if filters.metadata {
            print_symbol_metadata(symbol, file);
        } else {
            println!(
                "{}\t{}\t{}\t{}:{}-{}\t{}\t{}",
                symbol.kind,
                symbol.name,
                symbol.visibility,
                file.map(path_of).unwrap_or_else(|| "-".to_string()),
                symbol.line,
                symbol.line_end,
                symbol.owner.as_deref().unwrap_or("-"),
                symbol.signature,
            );
        }
        emitted += 1;
    }

    if filters.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&SymbolListJsonOutput {
                query: (!query.terms.is_empty()).then(|| {
                    query
                        .terms
                        .iter()
                        .map(|term| match (&term.key, term.negated) {
                            (Some(key), true) => format!("!{key}:{}", term.value),
                            (Some(key), false) => format!("{key}:{}", term.value),
                            (None, true) => format!("!{}", term.value),
                            (None, false) => term.value.clone(),
                        })
                        .collect::<Vec<_>>()
                        .join(",")
                }),
                filters: SymbolListJsonFilters {
                    path: filters.file_filter.map(normalize_path),
                    name: filters.name.map(str::to_string),
                    kind: filters.kind.map(str::to_string),
                    owner: filters.owner.map(str::to_string),
                    visibility: filters.visibility.map(str::to_string),
                    changed_only: filters.changed_only,
                },
                count: records.len(),
                limit,
                truncated,
                items: records,
            })?
        );
        return Ok(());
    }

    if emitted == 0 {
        println!("no symbols matched");
    }

    Ok(())
}

pub fn resolve_file_id(map: &CodeMap, path: &Path) -> Result<String> {
    let query = normalize_path(path);

    let exact = map
        .files
        .iter()
        .find(|file| normalize_path(&file.path) == query);
    if let Some(file) = exact {
        return Ok(file.id.clone());
    }

    let matches = map
        .files
        .iter()
        .filter(|file| normalize_path(&file.path).contains(&query))
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [file] => Ok(file.id.clone()),
        [] => bail!("file not found in codemap: {}", path.display()),
        _ => bail!("file is ambiguous in codemap: {}", path.display()),
    }
}

fn symbol_record(symbol: &SymbolEntry, file: Option<&FileEntry>) -> SymbolRecord {
    SymbolRecord {
        path: file
            .map(|entry| normalize_path(&entry.path))
            .unwrap_or_else(|| "-".to_string()),
        name: symbol.name.clone(),
        kind: symbol.kind.clone(),
        visibility: symbol.visibility.clone(),
        owner: symbol.owner.clone(),
        line: symbol.line,
        line_end: symbol.line_end,
        body_open_line: symbol.body_open_line,
        body_close_line: symbol.body_close_line,
        signature: symbol.signature.clone(),
        params: symbol.params.clone(),
        return_type: symbol.return_type.clone(),
        generics: symbol.generics.clone(),
        confidence: symbol.confidence,
    }
}

fn matches_filters(symbol: &SymbolEntry, query: &Query, filters: SymbolListFilters<'_>) -> bool {
    if !query.matches_symbol(
        &symbol.name,
        &symbol.kind,
        &symbol.visibility,
        symbol.owner.as_deref(),
        &symbol.tags,
    ) {
        return false;
    }
    if let Some(name) = filters.name
        && normalize_symbol_name(&symbol.name) != normalize_symbol_name(name)
    {
        return false;
    }
    if let Some(kind) = filters.kind
        && !symbol.kind.eq_ignore_ascii_case(kind)
    {
        return false;
    }
    if let Some(owner) = filters.owner
        && !symbol
            .owner
            .as_deref()
            .is_some_and(|value| contains_ci(value, owner))
    {
        return false;
    }
    if let Some(visibility) = filters.visibility
        && !symbol.visibility.eq_ignore_ascii_case(visibility)
    {
        return false;
    }
    true
}

fn path_of(file: &&FileEntry) -> String {
    normalize_path(&file.path)
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn print_symbol_metadata(symbol: &SymbolEntry, file: Option<&&FileEntry>) {
    println!("{} {}", symbol.kind, symbol.name);
    println!(
        "  file: {}",
        file.map(path_of).unwrap_or_else(|| "-".to_string())
    );
    println!(
        "  range: {}-{} ({} lines)",
        symbol.line, symbol.line_end, symbol.line_count
    );
    println!(
        "  body: {}-{}",
        symbol
            .body_open_line
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string()),
        symbol
            .body_close_line
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string())
    );
    println!("  visibility: {}", symbol.visibility);
    println!("  owner: {}", symbol.owner.as_deref().unwrap_or("-"));
    println!("  confidence: {}", symbol.confidence);
    println!("  params: {}", display_list(&symbol.params));
    println!(
        "  returns: {}",
        symbol.return_type.as_deref().unwrap_or("-")
    );
    println!("  generics: {}", display_list(&symbol.generics));
    println!("  tags: {}", display_list(&symbol.tags));
    println!("  signature: {}", symbol.signature);
}

fn display_list(values: &[String]) -> String {
    if values.is_empty() {
        "-".to_string()
    } else {
        values.join(", ")
    }
}

fn normalize_symbol_name(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn contains_ci(value: &str, needle: &str) -> bool {
    value
        .to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}
