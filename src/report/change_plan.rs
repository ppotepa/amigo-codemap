use anyhow::{Result, bail};

use crate::model::CodeMap;
use crate::report::anchors::{anchor_entry_matches, build_anchor_index};

pub fn print_change_plan(map: &CodeMap, query: &str, limit: usize) -> Result<()> {
    if query.trim().is_empty() {
        bail!("change-plan requires a query");
    }

    println!("change-plan: {query}");
    println!("1. scope:");
    print_scope(map, query, limit);
    println!("2. symbols:");
    print_symbols(map, query, limit);
    println!("3. anchor scope:");
    print_anchor_scope(map, query, limit);
    println!("4. text/config:");
    print_text(map, query, limit);
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

fn print_anchor_scope(map: &CodeMap, query: &str, limit: usize) {
    let index = build_anchor_index(map, None);
    let mut emitted = 0usize;
    for anchor in &index.anchors {
        if anchor_entry_matches(anchor, query) {
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

fn print_scope(map: &CodeMap, query: &str, limit: usize) {
    let query = query.to_ascii_lowercase();
    let mut emitted = 0usize;
    for file in &map.files {
        let path = file.path.to_string_lossy().replace('\\', "/");
        let tag_text = file.tags.join(",");
        if path.to_ascii_lowercase().contains(&query)
            || tag_text.to_ascii_lowercase().contains(&query)
        {
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

fn print_symbols(map: &CodeMap, query: &str, limit: usize) {
    let query = query.to_ascii_lowercase();
    let mut emitted = 0usize;
    for symbol in &map.symbols {
        if symbol.name.to_ascii_lowercase().contains(&query)
            || symbol.signature.to_ascii_lowercase().contains(&query)
        {
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

fn print_text(map: &CodeMap, query: &str, limit: usize) {
    let query = query.to_ascii_lowercase();
    let mut emitted = 0usize;
    for occurrence in &map.text_occurrences {
        if occurrence.normalized_value.contains(&query) {
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
