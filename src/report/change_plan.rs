use std::path::Path;

use anyhow::{Result, bail};

use crate::model::CodeMap;
use crate::query::descriptive_tokens;
use crate::report::anchors::{anchor_entry_matches, cached_anchor_index};

pub fn print_change_plan(root: &Path, map: &CodeMap, query: &str, limit: usize) -> Result<()> {
    if query.trim().is_empty() {
        bail!("change-plan requires a query");
    }

    println!("change-plan: {query}");
    let tokens = descriptive_tokens(query);
    if !tokens.is_empty() {
        println!("tokens: {}", tokens.join(", "));
    }
    println!("1. Cel zmiany:");
    println!("  {query}");
    println!("2. Nawigacja codemap:");
    println!("  change-plan: amigo-codemap change-plan {query} --limit {limit}");
    println!("  trace: amigo-codemap trace <symbol-or-text> --limit 10");
    println!("  open-set: amigo-codemap open-set {query} --why --limit {limit}");
    println!("  symbols: amigo-codemap symbols {query} --limit {limit}");
    println!(
        "  slice/range: amigo-codemap signature <symbol>; amigo-codemap range-for-symbol <symbol>; amigo-codemap slice <file> --symbol <symbol>"
    );
    println!("3. Oczekiwany open-set:");
    print_scope(map, query, &tokens, limit);
    println!("4. Symbole:");
    print_symbols(map, query, &tokens, limit);
    println!("5. Anchor scope:");
    print_anchor_scope(root, map, query, &tokens, limit);
    println!("6. Text/config:");
    print_text(map, query, &tokens, limit);
    println!("7. Instrukcje per plik:");
    println!(
        "  use raw ops blocks with ACTION, FILE, SYMBOL/WITHIN_SYMBOL, FIND/REPLACE, CONTENT, END"
    );
    println!("8. Konkretne zmiany kodu:");
    println!(
        "  supported actions: CREATE FILE, REPLACE SYMBOL, INSERT BEFORE SYMBOL, INSERT AFTER SYMBOL, REPLACE TEXT, INSERT BEFORE TEXT, INSERT AFTER TEXT, REPLACE RANGE, DELETE RANGE, MODIFY ENUM, MODIFY MATCH"
    );
    println!("9. Testy i verify:");
    println!("  cargo build -p amigo-codemap");
    println!("  cargo test -p amigo-codemap");
    println!("  amigo-codemap verify-plan --changed");
    Ok(())
}

fn print_anchor_scope(root: &Path, map: &CodeMap, query: &str, tokens: &[String], limit: usize) {
    let index = cached_anchor_index(root, map, None);
    let path_tokens = path_tokens(tokens);
    let mut emitted = 0usize;
    for anchor in &index.anchors {
        let matched_query = anchor_entry_matches(anchor, query);
        let matched_token = path_tokens
            .iter()
            .any(|token| anchor_entry_matches(anchor, token));
        if !matched_query && !matched_token {
            continue;
        }
        if matched_token
            && !matched_query
            && anchor.role == "file"
            && !path_tokens
                .iter()
                .any(|token| anchor.file.to_ascii_lowercase().contains(token))
        {
            continue;
        }

        println!(
            "  {} {} domain={} role={} file={}:{}",
            anchor.priority, anchor.anchor, anchor.domain, anchor.role, anchor.file, anchor.line
        );
        emitted += 1;
        if emitted >= limit {
            break;
        }
    }
    if emitted == 0 {
        println!("  none");
    }
}

fn print_scope(map: &CodeMap, query: &str, tokens: &[String], limit: usize) {
    let query = query.to_ascii_lowercase();
    let path_tokens = path_tokens(tokens);
    let mut ranked = Vec::new();
    for file in &map.files {
        let path = file.path.to_string_lossy().replace('\\', "/");
        let tag_text = file.tags.join(",");
        let haystack = format!("{} {}", path, tag_text).to_ascii_lowercase();
        let mut score = 0i32;
        if haystack.contains(&query) {
            score += 120;
        }
        for token in &path_tokens {
            if haystack.contains(token) {
                score += 25;
            }
        }
        if path_tokens.iter().any(|token| token == "amigo")
            && path_tokens.iter().any(|token| token == "codemap")
            && haystack.starts_with("crates/tools/amigo-codemap/")
        {
            score += 420;
        }
        for term in query.split_whitespace() {
            let term = term.replace('\\', "/");
            if term.len() >= 4 && term.contains('-') && haystack.contains(&term) {
                score += 140;
            }
        }
        if score > 0 {
            ranked.push((score, path, tag_text));
        }
    }

    ranked.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    for (_, path, tag_text) in ranked.iter().take(limit) {
        println!("  {path} tags={tag_text}");
    }
    if ranked.is_empty() {
        println!("  none");
    }
}

fn path_tokens(tokens: &[String]) -> Vec<String> {
    tokens
        .iter()
        .filter(|token| {
            token.len() >= 3
                && !matches!(
                    token.as_str(),
                    "the"
                        | "and"
                        | "for"
                        | "with"
                        | "from"
                        | "source"
                        | "policy"
                        | "performance"
                        | "optimize"
                        | "text"
                        | "refs"
                        | "set"
                )
        })
        .cloned()
        .collect()
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
