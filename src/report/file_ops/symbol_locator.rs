use std::path::Path;

use anyhow::{Result, bail};

use crate::model::{CodeMap, FileEntry, SymbolEntry};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolMatchKind {
    Exact,
    Normalized,
    OwnerQualified,
    Fuzzy,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct ResolvedSymbol<'a> {
    pub file: &'a FileEntry,
    pub symbol: &'a SymbolEntry,
    pub score: usize,
    pub match_kind: SymbolMatchKind,
}

#[derive(Debug)]
pub struct SymbolSuggestion<'a> {
    pub symbol: &'a SymbolEntry,
    pub score: usize,
}

pub fn resolve_file<'a>(map: &'a CodeMap, path: &Path) -> Result<&'a FileEntry> {
    map.files
        .iter()
        .find(|file| file.path == path)
        .ok_or_else(|| anyhow::anyhow!("file not found in codemap: {}", path.display()))
}

pub fn symbols_in_file<'a>(map: &'a CodeMap, file: &'a FileEntry) -> Vec<&'a SymbolEntry> {
    map.symbols
        .iter()
        .filter(|symbol| symbol.file_id == file.id)
        .collect()
}

pub fn symbol_suggestions<'a>(
    symbols: &'a [&'a SymbolEntry],
    query: &str,
    limit: usize,
) -> Vec<SymbolSuggestion<'a>> {
    let query_norm = normalize_symbol_name(query);
    let mut out = symbols
        .iter()
        .map(|symbol| SymbolSuggestion {
            score: symbol_similarity_score(symbol, query, &query_norm),
            symbol,
        })
        .collect::<Vec<_>>();
    out.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.symbol.name.cmp(&b.symbol.name))
    });
    out.truncate(limit);
    out
}

pub fn resolve_symbol_in_file<'a>(
    map: &'a CodeMap,
    path: &Path,
    query: &str,
) -> Result<ResolvedSymbol<'a>> {
    let file = resolve_file(map, path)?;
    let symbols = symbols_in_file(map, file);
    let query_norm = normalize_symbol_name(query);

    let mut candidates = symbols
        .iter()
        .filter_map(|symbol| {
            let (score, kind) = symbol_match_score(symbol, query, &query_norm)?;
            Some((score, kind, symbol))
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.2.name.cmp(&b.2.name)));
    let Some((score, kind, symbol)) = candidates.into_iter().next() else {
        let suggestions = symbol_suggestions(&symbols, query, 5);
        bail!("{}", format_symbol_not_found(path, query, &suggestions));
    };

    Ok(ResolvedSymbol {
        file,
        symbol,
        score,
        match_kind: kind,
    })
}

pub fn format_symbol_not_found(
    path: &Path,
    query: &str,
    suggestions: &[SymbolSuggestion<'_>],
) -> String {
    let mut out = format!(
        "symbol not found in {}: {}\nnearby symbols:\n",
        path.display(),
        query
    );
    if suggestions.is_empty() {
        out.push_str("  none\n");
    } else {
        for item in suggestions {
            out.push_str(&format!(
                "  {} {} score={}\n",
                item.symbol.kind, item.symbol.name, item.score
            ));
        }
    }
    out
}

fn symbol_match_score(
    symbol: &SymbolEntry,
    query: &str,
    query_norm: &str,
) -> Option<(usize, SymbolMatchKind)> {
    if symbol.name == query {
        return Some((1000, SymbolMatchKind::Exact));
    }
    if normalize_symbol_name(&symbol.name) == query_norm {
        return Some((900, SymbolMatchKind::Normalized));
    }
    if let Some(owner) = &symbol.owner {
        let owner_query = format!("{}::{}", owner, query);
        if owner_query.eq_ignore_ascii_case(&format!("{}::{}", owner, symbol.name)) {
            return Some((850, SymbolMatchKind::OwnerQualified));
        }
    }
    let score = symbol_similarity_score(symbol, query, query_norm);
    (score > 0).then_some((score, SymbolMatchKind::Fuzzy))
}

fn symbol_similarity_score(symbol: &SymbolEntry, query: &str, query_norm: &str) -> usize {
    let name_norm = normalize_symbol_name(&symbol.name);
    let mut score = 0;
    if name_norm.starts_with(query_norm) || query_norm.starts_with(&name_norm) {
        score += 50;
    }
    score += common_prefix_len(&symbol.name, query).min(20);
    score += common_subsequence_len(&symbol.name, query).min(20);
    score += symbol_token_overlap_score(&symbol.name, query);
    score
}

fn normalize_symbol_name(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn common_prefix_len(left: &str, right: &str) -> usize {
    left.chars()
        .zip(right.chars())
        .take_while(|(a, b)| a.eq_ignore_ascii_case(b))
        .count()
}

fn common_subsequence_len(left: &str, right: &str) -> usize {
    let right_norm = normalize_symbol_name(right);
    normalize_symbol_name(left)
        .chars()
        .filter(|ch| right_norm.contains(*ch))
        .count()
}

fn symbol_token_overlap_score(left: &str, right: &str) -> usize {
    let left_tokens = symbol_tokens(left);
    let right_tokens = symbol_tokens(right);
    left_tokens.intersection(&right_tokens).count() * 10
}

fn symbol_tokens(value: &str) -> std::collections::BTreeSet<String> {
    let mut out = std::collections::BTreeSet::new();
    for token in value.split(|ch: char| !ch.is_ascii_alphanumeric()) {
        if !token.is_empty() {
            out.insert(token.to_ascii_lowercase());
        }
    }
    out
}
