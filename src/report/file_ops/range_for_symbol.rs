use std::path::Path;

use anyhow::{Result, bail};

use crate::model::{CodeMap, FileEntry, SymbolEntry};

pub fn print_range_for_symbol(map: &CodeMap, query: &str, limit: usize) -> Result<()> {
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

fn print_symbol_range(symbol: &SymbolEntry, file: Option<&FileEntry>) {
    println!("symbol: {}", symbol.name);
    println!("kind: {}", symbol.kind);
    println!(
        "path: {}",
        file.map(|file| slash_path(&file.path))
            .unwrap_or_else(|| "-".to_string())
    );
    println!("start_line: {}", symbol.line);
    println!("end_line: {}", symbol.line_end);
    println!("line_count: {}", symbol.line_count);
    println!(
        "hash: {}",
        file.map(|file| file.hash.as_str()).unwrap_or("-")
    );
    println!(
        "expected_hash: {}",
        file.map(|file| file.hash.as_str()).unwrap_or("-")
    );
    println!("visibility: {}", symbol.visibility);
    println!("owner: {}", symbol.owner.as_deref().unwrap_or("-"));
    println!("signature: {}", symbol.signature);
    println!("ops hint:");
    if let Some(file) = file {
        println!("  - kind: replace_symbol");
        println!("    path: {}", slash_path(&file.path));
        println!("    symbol: {}", symbol.name);
        println!("    content: |");
        println!("      # replacement content");
    }
}

fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
