use std::path::Path;

use anyhow::{Result, bail};

use crate::model::{CodeMap, FileEntry, SymbolEntry};
use crate::query::Query;

pub fn print_symbols(
    map: &CodeMap,
    query: Option<&str>,
    file_filter: Option<&Path>,
    changed_only: bool,
    metadata: bool,
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
    let file_id_filter = match file_filter {
        Some(path) => Some(resolve_file_id(map, path)?),
        None => None,
    };

    if let Some(file_id) = file_id_filter.as_deref() {
        if let Some(file) = files.get(file_id) {
            println!("file: {}", path_of(file));
        }
    }

    let mut emitted = 0usize;
    for symbol in &map.symbols {
        if changed_only && !changed_ids.contains(&symbol.file_id) {
            continue;
        }
        if file_id_filter
            .as_deref()
            .is_some_and(|id| id != symbol.file_id)
        {
            continue;
        }
        if !query.matches_symbol(
            &symbol.name,
            &symbol.kind,
            &symbol.visibility,
            symbol.owner.as_deref(),
            &symbol.tags,
        ) {
            continue;
        }
        if emitted >= limit {
            break;
        }

        let file = files.get(symbol.file_id.as_str());
        if metadata {
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

    if emitted == 0 {
        println!("no symbols matched");
    }

    Ok(())
}

fn path_of(file: &&FileEntry) -> String {
    file.path.to_string_lossy().replace('\\', "/")
}

fn resolve_file_id(map: &CodeMap, path: &Path) -> Result<String> {
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
