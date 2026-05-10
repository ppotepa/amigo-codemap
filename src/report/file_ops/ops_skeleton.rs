use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use crate::model::{CodeMap, FileEntry, SymbolEntry};

pub fn print_ops_skeleton(
    map: &CodeMap,
    query: &str,
    out: &Path,
    write: bool,
    raw: bool,
    limit: usize,
) -> Result<()> {
    if query.trim().is_empty() {
        bail!("ops-skeleton requires a query");
    }

    let text = if raw {
        render_raw_ops_skeleton(map, query, limit)
    } else {
        render_ops_skeleton(map, query, limit)
    };
    if write {
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(out, text)?;
        println!("wrote {}", out.display());
    } else {
        print!("{text}");
    }

    Ok(())
}

fn render_raw_ops_skeleton(map: &CodeMap, query: &str, limit: usize) -> String {
    let files = map
        .files
        .iter()
        .map(|file| (file.id.as_str(), file))
        .collect::<std::collections::BTreeMap<_, _>>();
    let symbols = matching_symbols(map, query);

    let mut output = String::new();
    if symbols.is_empty() {
        for file in matching_files(map, query).into_iter().take(limit.max(1)) {
            output.push_str("ACTION: REPLACE RANGE\n");
            output.push_str(&format!("FILE: {}\n", slash_path(&file.path)));
            output.push_str("START_LINE: 1\n");
            output.push_str("END_LINE: 1\n");
            output.push_str(&format!("EXPECTED_HASH: {}\n", file.hash));
            output.push_str("CONTENT:\n# replacement content\nEND\n\n");
        }
    } else {
        for symbol in symbols.into_iter().take(limit.max(1)) {
            if let Some(file) = files.get(symbol.file_id.as_str()) {
                output.push_str("ACTION: REPLACE SYMBOL\n");
                output.push_str(&format!("FILE: {}\n", slash_path(&file.path)));
                output.push_str(&format!("SYMBOL: {}\n", symbol.name));
                output.push_str(&format!("EXPECTED_HASH: {}\n", file.hash));
                output.push_str("CONTENT:\n# replacement content\nEND\n\n");
            }
        }
    }
    output
}

fn render_ops_skeleton(map: &CodeMap, query: &str, limit: usize) -> String {
    let mut output = String::new();
    let task = slug(query);
    output.push_str(&format!("task: {task}\n"));
    output.push_str("ops:\n");

    let files = map
        .files
        .iter()
        .map(|file| (file.id.as_str(), file))
        .collect::<std::collections::BTreeMap<_, _>>();
    let symbols = matching_symbols(map, query);

    if symbols.is_empty() {
        for file in matching_files(map, query).into_iter().take(limit.max(1)) {
            push_replace_range_stub(&mut output, file);
        }
    } else {
        for symbol in symbols.into_iter().take(limit.max(1)) {
            if let Some(file) = files.get(symbol.file_id.as_str()) {
                push_replace_symbol_stub(&mut output, file, symbol);
            }
        }
    }

    output
}

fn matching_symbols<'a>(map: &'a CodeMap, query: &str) -> Vec<&'a SymbolEntry> {
    let query = query.to_ascii_lowercase();
    map.symbols
        .iter()
        .filter(|symbol| {
            symbol.name.to_ascii_lowercase().contains(&query)
                || symbol.signature.to_ascii_lowercase().contains(&query)
        })
        .collect()
}

fn matching_files<'a>(map: &'a CodeMap, query: &str) -> Vec<&'a FileEntry> {
    let query = query.to_ascii_lowercase();
    map.files
        .iter()
        .filter(|file| {
            slash_path(&file.path).to_ascii_lowercase().contains(&query)
                || file.tags.join(",").to_ascii_lowercase().contains(&query)
        })
        .collect()
}

fn push_replace_symbol_stub(output: &mut String, file: &FileEntry, symbol: &SymbolEntry) {
    output.push_str("  - kind: replace_symbol\n");
    output.push_str(&format!("    path: {}\n", slash_path(&file.path)));
    output.push_str(&format!("    symbol: {}\n", symbol.name));
    output.push_str(&format!("    expected_hash: \"{}\"\n", file.hash));
    output.push_str("    content: |\n");
    output.push_str("      # replacement content\n");
}

fn push_replace_range_stub(output: &mut String, file: &FileEntry) {
    output.push_str("  - kind: replace_range\n");
    output.push_str(&format!("    path: {}\n", slash_path(&file.path)));
    output.push_str("    start_line: 1\n");
    output.push_str("    end_line: 1\n");
    output.push_str(&format!("    expected_hash: \"{}\"\n", file.hash));
    output.push_str("    context_before: null\n");
    output.push_str("    context_after: null\n");
    output.push_str("    content: |\n");
    output.push_str("      # replacement content\n");
}

fn slug(value: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            previous_dash = false;
        } else if !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
    }
    slug.trim_matches('-').to_string()
}

fn slash_path(path: &PathBuf) -> String {
    path.to_string_lossy().replace('\\', "/")
}
