use std::path::Path;

use anyhow::{Result, bail};
use serde::Serialize;

use crate::model::{CodeMap, FileEntry, SymbolEntry};

#[derive(Debug, Clone, Serialize)]
struct SymbolRangeRecord {
    path: String,
    name: String,
    kind: String,
    owner: Option<String>,
    line: usize,
    line_end: usize,
    body_open_line: Option<usize>,
    body_close_line: Option<usize>,
    signature: String,
}

pub fn print_range_for_symbol(map: &CodeMap, query: &str, limit: usize, json: bool) -> Result<()> {
    if query.trim().is_empty() {
        bail!("range-for-symbol requires a symbol query");
    }

    let files = map
        .files
        .iter()
        .map(|file| (file.id.as_str(), file))
        .collect::<std::collections::BTreeMap<_, _>>();
    let query_lower = query.to_ascii_lowercase();
    let matches = map
        .symbols
        .iter()
        .filter(|symbol| {
            symbol.name == query || symbol.name.to_ascii_lowercase().contains(&query_lower)
        })
        .collect::<Vec<_>>();

    if json {
        let records = matches
            .into_iter()
            .take(limit)
            .map(|symbol| {
                let file = files.get(symbol.file_id.as_str()).copied();
                symbol_range_record(symbol, file)
            })
            .collect::<Vec<_>>();
        println!("{}", serde_json::to_string_pretty(&records)?);
        return Ok(());
    }

    println!("range-for-symbol: {query}");
    if matches.is_empty() {
        println!("no symbols matched");
        return Ok(());
    }

    for symbol in matches.into_iter().take(limit) {
        let file = files.get(symbol.file_id.as_str());
        print_symbol_range(symbol, file.copied());
    }

    Ok(())
}

fn symbol_range_record(symbol: &SymbolEntry, file: Option<&FileEntry>) -> SymbolRangeRecord {
    SymbolRangeRecord {
        path: file
            .map(|entry| slash_path(&entry.path))
            .unwrap_or_else(|| "-".to_string()),
        name: symbol.name.clone(),
        kind: symbol.kind.clone(),
        owner: symbol.owner.clone(),
        line: symbol.line,
        line_end: symbol.line_end,
        body_open_line: symbol.body_open_line,
        body_close_line: symbol.body_close_line,
        signature: symbol.signature.clone(),
    }
}

fn print_symbol_range(symbol: &SymbolEntry, file: Option<&FileEntry>) {
    let path = file
        .map(|entry| slash_path(&entry.path))
        .unwrap_or_else(|| "-".to_string());
    println!("{} {}", symbol.kind, symbol.name);
    println!("  file: {path}");
    println!("  line-start: {}", symbol.line);
    println!("  line-end: {}", symbol.line_end);
    println!(
        "  body-open-line: {}",
        symbol
            .body_open_line
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string())
    );
    println!(
        "  body-close-line: {}",
        symbol
            .body_close_line
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string())
    );
    println!("  signature: {}", symbol.signature);
    println!(
        "  raw-op: ACTION: REPLACE SYMBOL | FILE: {} | SYMBOL: {}",
        path, symbol.name
    );
}

fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
