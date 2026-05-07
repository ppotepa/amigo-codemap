use anyhow::{Result, bail};

use crate::model::CodeMap;
use crate::query::descriptive_tokens;
use crate::report::anchors::{anchor_entry_matches, build_anchor_index};

pub fn print_change_plan(map: &CodeMap, query: &str, limit: usize) -> Result<()> {
    if query.trim().is_empty() {
        bail!("change-plan requires a query");
    }

    println!("change-plan: {query}");
    let tokens = descriptive_tokens(query);
    if !tokens.is_empty() {
        println!("tokens: {}", tokens.join(", "));
    }
    println!("1. scope:");
    print_scope(map, query, &tokens, limit);
    println!("2. symbols:");
    print_symbols(map, query, &tokens, limit);
    println!("3. anchor scope:");
    print_anchor_scope(map, query, &tokens, limit);
    println!("4. text/config:");
    print_text(map, query, &tokens, limit);
    println!("5. suggested commands:");
    println!("  amigo-codemap trace {query} --limit {limit}");
    println!("  amigo-codemap anchors {query} --limit {limit}");
    println!("  amigo-codemap open-set {query} --why --limit {limit}");
    println!("  amigo-codemap impact {query} --limit {limit}");
    println!("  amigo-codemap verify-plan --changed");
    println!("6. verify:");
    println!("  cargo build -p amigo-codemap");
    println!("  cargo test -p amigo-codemap");
    Ok(())
}

fn print_anchor_scope(map: &CodeMap, query: &str, tokens: &[String], limit: usize) {
    let index = build_anchor_index(map, None);
    let mut emitted = 0usize;
    for anchor in &index.anchors {
        if anchor_entry_matches(anchor, query)
            || tokens
                .iter()
                .any(|token| anchor_entry_matches(anchor, token))
        {
            println!(
                "  {} {} domain={} role={} file={}:{}",
                anchor.priority,
                anchor.anchor,
                anchor.domain,
                anchor.role,
                anchor.file,
                anchor.line
            );
            emitted += 1;
            if emitted >= limit {
                break;
            }
        }
    }
    if emitted == 0 {
        println!("  none");
    }
}

fn print_scope(map: &CodeMap, query: &str, tokens: &[String], limit: usize) {
    let query = query.to_ascii_lowercase();
    let mut emitted = 0usize;
    for file in &map.files {
        let path = file.path.to_string_lossy().replace('\\', "/");
        let tag_text = file.tags.join(",");
        let haystack = format!("{} {}", path, tag_text).to_ascii_lowercase();
        if haystack.contains(&query) || tokens.iter().any(|token| haystack.contains(token)) {
            println!("  {path} tags={tag_text}");
            emitted += 1;
            if emitted >= limit {
                break;
            }
        }
    }
    if emitted == 0 {
        println!("  none");
    }
}

fn print_symbols(map: &CodeMap, query: &str, tokens: &[String], limit: usize) {
    let query = query.to_ascii_lowercase();
    let mut emitted = 0usize;
    for symbol in &map.symbols {
        let haystack = format!(
            "{} {} {} {}",
            symbol.name,
            symbol.kind,
            symbol.signature,
            symbol.tags.join(",")
        )
        .to_ascii_lowercase();
        if haystack.contains(&query) || tokens.iter().any(|token| haystack.contains(token)) {
            println!(
                "  {} {} {}:{}-{}",
                symbol.kind, symbol.name, symbol.file_id, symbol.line, symbol.line_end
            );
            emitted += 1;
            if emitted >= limit {
                break;
            }
        }
    }
    if emitted == 0 {
        println!("  none");
    }
}

fn print_text(map: &CodeMap, query: &str, tokens: &[String], limit: usize) {
    let query = query.to_ascii_lowercase();
    let mut emitted = 0usize;
    for occurrence in &map.text_occurrences {
        let haystack = format!(
            "{} {} {}",
            occurrence.normalized_value,
            occurrence.kind,
            occurrence.tags.join(",")
        )
        .to_ascii_lowercase();
        if haystack.contains(&query) || tokens.iter().any(|token| haystack.contains(token)) {
            println!(
                "  {} {}:{} {}",
                occurrence.kind, occurrence.file_id, occurrence.line, occurrence.context
            );
            emitted += 1;
            if emitted >= limit {
                break;
            }
        }
    }
    if emitted == 0 {
        println!("  none");
    }
}
