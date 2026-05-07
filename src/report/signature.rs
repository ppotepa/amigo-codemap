use anyhow::{Result, bail};

use crate::model::{CodeMap, FileEntry};

pub fn print_signature(map: &CodeMap, query: &str, limit: usize) -> Result<()> {
    if query.trim().is_empty() {
        bail!("signature requires a symbol query");
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

    println!("signature: {query}");
    if matches.is_empty() {
        println!("no symbols matched");
        return Ok(());
    }

    for symbol in matches.into_iter().take(limit) {
        let file = files.get(symbol.file_id.as_str());
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
        println!("  tags: {}", symbol.tags.join(","));
        println!("  signature: {}", symbol.signature);
        if !symbol.generics.is_empty() {
            println!("  generics: {}", symbol.generics.join(", "));
        }
        if !symbol.params.is_empty() {
            println!("  params: {}", symbol.params.join(", "));
        }
        println!(
            "  returns: {}",
            symbol.return_type.as_deref().unwrap_or("-")
        );
    }
    Ok(())
}

fn path_of(file: &&FileEntry) -> String {
    file.path.to_string_lossy().replace('\\', "/")
}
