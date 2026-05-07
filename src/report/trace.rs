use anyhow::{Result, bail};

use crate::model::{CodeMap, FileEntry};
use crate::report::anchors::tag_matches_query;

pub fn print_trace(map: &CodeMap, query: &str, limit: usize) -> Result<()> {
    if query.trim().is_empty() {
        bail!("trace requires a query");
    }
    let files = map
        .files
        .iter()
        .map(|file| (file.id.as_str(), file))
        .collect::<std::collections::BTreeMap<_, _>>();
    let query_lower = query.to_ascii_lowercase();

    println!("trace: {query}");
    println!("likely meaning: {}", classify_trace_query(query));
    println!("matched symbols:");
    let mut symbol_count = 0usize;
    for symbol in &map.symbols {
        if symbol.name == query || symbol.name.to_ascii_lowercase().contains(&query_lower) {
            symbol_count += 1;
            if symbol_count <= limit {
                let file = files.get(symbol.file_id.as_str());
                println!(
                    "  {} {} {}:{} owner={} visibility={}",
                    symbol.kind,
                    symbol.name,
                    file.map(path_of).unwrap_or_else(|| "-".to_string()),
                    symbol.line,
                    symbol.owner.as_deref().unwrap_or("-"),
                    symbol.visibility,
                );
            }
        }
    }
    if symbol_count == 0 {
        println!("  none");
    }

    println!("matched text occurrences:");
    let mut by_kind = std::collections::BTreeMap::<String, Vec<_>>::new();
    for occurrence in &map.text_occurrences {
        if occurrence.normalized_value.contains(&query_lower)
            || occurrence.value.to_ascii_lowercase().contains(&query_lower)
        {
            by_kind
                .entry(occurrence.kind.clone())
                .or_default()
                .push(occurrence);
        }
    }
    if by_kind.is_empty() {
        println!("  none");
    } else {
        for (kind, occurrences) in by_kind {
            println!("  {kind}: {}", occurrences.len());
            for occurrence in occurrences.into_iter().take(5) {
                let file = files.get(occurrence.file_id.as_str());
                println!(
                    "    {}:{} {}",
                    file.map(path_of).unwrap_or_else(|| "-".to_string()),
                    occurrence.line,
                    occurrence.context,
                );
            }
        }
    }

    println!("matched anchors:");
    let mut tag_count = 0usize;
    for tag in &map.tags {
        if tag_matches_query(tag, query) {
            tag_count += 1;
            if tag_count <= limit {
                let file = files.get(tag.file_id.as_str());
                println!(
                    "  {} {} domain={} role={} file={}:{}",
                    tag.priority.as_deref().unwrap_or("P2"),
                    tag.anchor,
                    tag.domain.as_deref().unwrap_or("-"),
                    tag.role.as_deref().unwrap_or("-"),
                    file.map(path_of).unwrap_or_else(|| "-".to_string()),
                    tag.line
                );
            }
        }
    }
    if tag_count == 0 {
        println!("  none");
    }

    println!("related files:");
    let mut related = std::collections::BTreeSet::<String>::new();
    for symbol in &map.symbols {
        if (symbol.name == query || symbol.name.to_ascii_lowercase().contains(&query_lower))
            && let Some(file) = files.get(symbol.file_id.as_str())
        {
            related.insert(path_of(file));
        }
    }
    for occurrence in &map.text_occurrences {
        if (occurrence.normalized_value.contains(&query_lower)
            || occurrence.value.to_ascii_lowercase().contains(&query_lower))
            && let Some(file) = files.get(occurrence.file_id.as_str())
        {
            related.insert(path_of(file));
        }
    }
    for path in related.iter().take(limit) {
        println!("  {path}");
    }

    println!("next:");
    println!("  1. where {query}");
    println!("  2. signature {query}");
    println!("  3. open-set {query} --task trace");
    println!("  4. impact {query} --group feature");
    Ok(())
}

fn classify_trace_query(query: &str) -> &'static str {
    if query.starts_with('.') {
        "css-class"
    } else if query.contains('/') && query.contains("scene") {
        "scene-path"
    } else if query.contains('.') && query.contains('-') {
        "id-like-string"
    } else if query.ends_with("_command") || query.contains("invoke") {
        "command"
    } else {
        "symbol-or-text"
    }
}

fn path_of(file: &&FileEntry) -> String {
    file.path.to_string_lossy().replace('\\', "/")
}
