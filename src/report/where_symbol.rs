use anyhow::{Result, bail};

use crate::model::{CodeMap, FileEntry};
use crate::report::common::text_refs;

pub fn print_where(root: &std::path::Path, map: &CodeMap, query: &str, limit: usize) -> Result<()> {
    if query.trim().is_empty() {
        bail!("where requires a symbol query");
    }

    let files = map
        .files
        .iter()
        .map(|file| (file.id.as_str(), file))
        .collect::<std::collections::BTreeMap<_, _>>();
    let query_lower = query.to_ascii_lowercase();
    let definitions = map
        .symbols
        .iter()
        .filter(|symbol| {
            symbol.name == query || symbol.name.to_ascii_lowercase().contains(&query_lower)
        })
        .collect::<Vec<_>>();

    println!("where: {query}");
    println!("definitions:");
    if definitions.is_empty() {
        println!("  none");
    } else {
        for symbol in definitions.iter().take(limit) {
            let file = files.get(symbol.file_id.as_str());
            println!(
                "  {} {} {}:{}-{} owner={} visibility={} confidence={}",
                symbol.kind,
                symbol.name,
                file.map(path_of).unwrap_or_else(|| "-".to_string()),
                symbol.line,
                symbol.line_end,
                symbol.owner.as_deref().unwrap_or("-"),
                symbol.visibility,
                symbol.confidence,
            );
            println!("    {}", symbol.signature);
        }
    }

    println!("references:");
    let refs = text_refs(root, map, query, limit)?;
    if refs.is_empty() {
        println!("  none");
    } else {
        for reference in refs.iter().take(limit) {
            println!(
                "  {}{}",
                reference.path,
                if reference.changed { " changed" } else { "" }
            );
            for (line, text) in reference.lines.iter().take(3) {
                println!("    {line}: {text}");
            }
        }
    }

    println!("next:");
    if let Some(symbol) = definitions.first()
        && let Some(file) = files.get(symbol.file_id.as_str())
    {
        println!("  1. slice {} --symbol {}", path_of(file), symbol.name);
    }
    println!("  2. trace {query}");
    println!("  3. impact {query} --group feature");
    Ok(())
}

fn path_of(file: &&FileEntry) -> String {
    file.path.to_string_lossy().replace('\\', "/")
}
