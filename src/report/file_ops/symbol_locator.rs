use std::path::Path;

use anyhow::{Result, bail};
use serde::Serialize;

use crate::model::{CodeMap, FileEntry, SymbolEntry};
use crate::query::Query;

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

#[derive(Debug, Clone, Copy, Default)]
pub struct SymbolFilters<'a> {
    pub name: Option<&'a str>,
    pub kind: Option<&'a str>,
    pub owner: Option<&'a str>,
    pub visibility: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResolvedSymbolJson {
    pub path: String,
    pub symbol: String,
    pub kind: String,
    pub visibility: String,
    pub owner: Option<String>,
    pub signature: String,
    pub line: usize,
    pub line_end: usize,
    pub body_open_line: Option<usize>,
    pub body_close_line: Option<usize>,
    pub match_kind: String,
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
    resolve_symbol_in_file_with_filters(map, path, query, SymbolFilters::default(), false)
}

pub fn resolve_symbol_in_file_with_filters<'a>(
    map: &'a CodeMap,
    path: &Path,
    query: &str,
    filters: SymbolFilters<'_>,
    strict: bool,
) -> Result<ResolvedSymbol<'a>> {
    let file = resolve_file(map, path)?;
    let symbols = symbols_in_file(map, file);
    let filtered = symbols
        .into_iter()
        .filter(|symbol| symbol_matches_filters(symbol, query, filters))
        .collect::<Vec<_>>();
    let query_norm = normalize_symbol_name(query);

    let mut candidates = filtered
        .iter()
        .filter_map(|symbol| {
            let (score, kind) = symbol_match_score(symbol, query, &query_norm)?;
            Some((score, kind, symbol))
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.2.name.cmp(&b.2.name)));
    if strict {
        let exact = candidates
            .iter()
            .filter(|(_, kind, _)| {
                matches!(kind, SymbolMatchKind::Exact | SymbolMatchKind::Normalized)
            })
            .collect::<Vec<_>>();
        match exact.as_slice() {
            [] => {
                let suggestions = symbol_suggestions(&filtered, query, 5);
                bail!("{}", format_symbol_not_found(path, query, &suggestions));
            }
            [item] => {
                let (score, kind, symbol) = **item;
                return Ok(ResolvedSymbol {
                    file,
                    symbol,
                    score,
                    match_kind: kind,
                });
            }
            _ => {
                bail!("{}", format_symbol_ambiguous(path, query, &exact));
            }
        }
    }

    let Some((score, kind, symbol)) = candidates.into_iter().next() else {
        let suggestions = symbol_suggestions(&filtered, query, 5);
        bail!("{}", format_symbol_not_found(path, query, &suggestions));
    };

    Ok(ResolvedSymbol {
        file,
        symbol,
        score,
        match_kind: kind,
    })
}

pub fn format_symbol_ambiguous(
    path: &Path,
    query: &str,
    matches: &[&(usize, SymbolMatchKind, &&SymbolEntry)],
) -> String {
    let mut out = format!(
        "symbol is ambiguous in {}: {}\nmatches:\n",
        path.display(),
        query
    );
    for (_, _, symbol) in matches {
        out.push_str(&format!(
            "  {} {} visibility={} owner={} line={} body={}-{}\n",
            symbol.kind,
            symbol.name,
            symbol.visibility,
            symbol.owner.as_deref().unwrap_or("-"),
            symbol.line,
            symbol
                .body_open_line
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
            symbol
                .body_close_line
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string())
        ));
    }
    let kinds = matches
        .iter()
        .map(|(_, _, symbol)| symbol.kind.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let visibilities = matches
        .iter()
        .map(|(_, _, symbol)| symbol.visibility.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let owners = matches
        .iter()
        .filter_map(|(_, _, symbol)| symbol.owner.as_deref())
        .collect::<std::collections::BTreeSet<_>>();
    out.push_str("try:\n");
    if kinds.len() > 1 {
        for kind in kinds {
            out.push_str(&format!("  --kind {kind}\n"));
        }
    }
    if visibilities.len() > 1 {
        for visibility in visibilities {
            out.push_str(&format!("  --visibility {visibility}\n"));
        }
    }
    for owner in owners.into_iter().take(5) {
        out.push_str(&format!("  --owner \"{owner}\"\n"));
    }
    out
}

pub fn resolved_symbol_json(resolved: &ResolvedSymbol<'_>) -> ResolvedSymbolJson {
    ResolvedSymbolJson {
        path: slash_path(&resolved.file.path),
        symbol: resolved.symbol.name.clone(),
        kind: resolved.symbol.kind.clone(),
        visibility: resolved.symbol.visibility.clone(),
        owner: resolved.symbol.owner.clone(),
        signature: resolved.symbol.signature.clone(),
        line: resolved.symbol.line,
        line_end: resolved.symbol.line_end,
        body_open_line: resolved.symbol.body_open_line,
        body_close_line: resolved.symbol.body_close_line,
        match_kind: format!("{:?}", resolved.match_kind).to_ascii_lowercase(),
        score: resolved.score,
    }
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

fn symbol_matches_filters(symbol: &SymbolEntry, query: &str, filters: SymbolFilters<'_>) -> bool {
    let selector = selector_query(query, filters);
    if !selector.matches_symbol(
        &symbol.name,
        &symbol.kind,
        &symbol.visibility,
        symbol.owner.as_deref(),
        &symbol.tags,
    ) {
        return false;
    }
    if let Some(name) = filters.name
        && normalize_symbol_name(&symbol.name) != normalize_symbol_name(name)
    {
        return false;
    }
    if let Some(kind) = filters.kind
        && !symbol.kind.eq_ignore_ascii_case(kind)
    {
        return false;
    }
    if let Some(owner) = filters.owner
        && !symbol.owner.as_deref().is_some_and(|value| {
            value
                .to_ascii_lowercase()
                .contains(&owner.to_ascii_lowercase())
        })
    {
        return false;
    }
    if let Some(visibility) = filters.visibility
        && !symbol.visibility.eq_ignore_ascii_case(visibility)
    {
        return false;
    }
    if let Some(name) = filters.name {
        return normalize_symbol_name(&symbol.name) == normalize_symbol_name(name);
    }
    if !query.is_empty() {
        return symbol_match_score(symbol, query, &normalize_symbol_name(query)).is_some();
    }
    true
}

fn selector_query(query: &str, filters: SymbolFilters<'_>) -> Query {
    let mut terms = Vec::new();
    if !query.trim().is_empty() {
        terms.push(query.trim().to_string());
    }
    if let Some(name) = filters.name {
        terms.push(format!("name:{name}"));
    }
    if let Some(kind) = filters.kind {
        terms.push(format!("kind:{kind}"));
    }
    if let Some(owner) = filters.owner {
        terms.push(format!("owner:{owner}"));
    }
    if let Some(visibility) = filters.visibility {
        terms.push(format!("visibility:{visibility}"));
    }
    let joined = terms.join(",");
    Query::parse((!joined.is_empty()).then_some(joined.as_str()))
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

fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
