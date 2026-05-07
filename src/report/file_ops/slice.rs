use std::path::Path;

use anyhow::Result;

use crate::model::{CodeMap, SymbolEntry};

use super::common::{
    find_file_by_path, import_block, is_changed, line_window, read_text_at_root, slash_path,
    symbols_in_file,
};
use super::model::{FileOpReport, NextAction, Risk, RiskLevel};

pub fn print_slice(
    root: &Path,
    map: &CodeMap,
    query: &str,
    symbol: Option<&str>,
    radius: usize,
) -> Result<()> {
    let file = find_file_by_path(map, query)
        .ok_or_else(|| anyhow::anyhow!("slice requires an existing file path: {query}"))?;
    let path = file.path.clone();
    let text = read_text_at_root(root, &path)?;
    let symbols = symbols_in_file(map, &file.id);
    if let Some(symbol_name) = symbol {
        let Some(target) = find_symbol_match(&symbols, symbol_name) else {
            let suggestions = symbol_suggestions(&symbols, symbol_name, 8);
            let mut message = format!("symbol not found in {}: {}", slash_path(&path), symbol_name);
            if !suggestions.is_empty() {
                message.push_str("\nnearby symbols:");
                for suggestion in suggestions {
                    message.push_str(&format!(
                        "\n  {} {}:{} score={}",
                        suggestion.symbol.kind,
                        suggestion.symbol.name,
                        suggestion.symbol.line,
                        suggestion.score
                    ));
                }
                message.push_str("\nnext: run `symbols --file <path> --metadata` or retry `slice --symbol <suggested-name>`");
            }
            anyhow::bail!(message);
        };
        for (index, line_text) in text.lines().enumerate() {
            let line_no = index + 1;
            if line_no >= target.line && line_no <= target.line_end {
                println!("{line_no}: {line_text}");
            }
        }
        return Ok(());
    }
    let target = symbol
        .and_then(|name| symbols.iter().find(|entry| entry.name == name))
        .or_else(|| symbols.first());

    let mut scope = vec![
        format!("file: {}", slash_path(&path)),
        format!("lines: {}", file.lines),
        format!("language: {}", file.language),
    ];
    if let Some(target) = target {
        scope.push(format!(
            "symbol: {}:{}:{}",
            target.kind, target.name, target.line
        ));
    }

    let mut findings = Vec::new();
    findings.push("imports:".to_string());
    for (line, text) in import_block(&text).into_iter().take(20) {
        findings.push(format!("  {line}: {text}"));
    }

    findings.push("symbol window:".to_string());
    if let Some(target) = target {
        for (line_no, line_text) in line_window(&text, target.line, radius) {
            findings.push(format!("  {line_no}: {}", line_text));
        }
    } else {
        for (line_no, line_text) in line_window(&text, 1, radius) {
            findings.push(format!("  {line_no}: {}", line_text));
        }
    }

    if !symbols.is_empty() {
        let mut local_deps = std::collections::BTreeSet::new();
        for dependency in &map.dependencies {
            if dependency.from == file.id {
                local_deps.insert(dependency.to.clone());
            }
        }
        findings.push("local deps:".to_string());
        if local_deps.is_empty() {
            findings.push("  none".to_string());
        } else {
            for item in local_deps.iter() {
                findings.push(format!("  {item}"));
            }
        }
    }

    let mut risks = Vec::new();
    if is_changed(map, &path) {
        risks.push(Risk {
            level: RiskLevel::Medium,
            message: "local context changed; include verify-plan before edit".to_string(),
        });
    }
    if target.is_none() {
        risks.push(Risk {
            level: RiskLevel::Low,
            message: "symbol not found; showing file window only".to_string(),
        });
    }

    let mut next = Vec::new();
    if target.is_some() {
        next.push(NextAction {
            label: "inspect this slice".to_string(),
        });
    } else {
        next.push(NextAction {
            label: "find a precise symbol name with --symbol".to_string(),
        });
    }
    next.push(NextAction {
        label: "run impact for changed exported symbol".to_string(),
    });

    super::model::print_report(&FileOpReport {
        task: format!("slice {query}"),
        scope,
        findings,
        risks,
        verify: vec![
            "npm run build".to_string(),
            "npm test".to_string(),
            "cargo test -p amigo-editor --lib".to_string(),
        ],
        next,
    });

    Ok(())
}

fn find_symbol_match<'a>(symbols: &'a [&'a SymbolEntry], query: &str) -> Option<&'a SymbolEntry> {
    symbols
        .iter()
        .copied()
        .find(|entry| symbol_name_matches(&entry.name, query))
        .or_else(|| {
            let suggestions = symbol_suggestions(symbols, query, 1);
            suggestions
                .first()
                .and_then(|suggestion| (suggestion.score >= 88).then_some(suggestion.symbol))
        })
}

fn symbol_name_matches(name: &str, query: &str) -> bool {
    name == query || normalize_symbol_name(name) == normalize_symbol_name(query)
}

#[derive(Debug, Clone, Copy)]
struct SymbolSuggestion<'a> {
    symbol: &'a SymbolEntry,
    score: usize,
}

fn symbol_suggestions<'a>(
    symbols: &'a [&'a SymbolEntry],
    query: &str,
    limit: usize,
) -> Vec<SymbolSuggestion<'a>> {
    let mut suggestions = symbols
        .iter()
        .copied()
        .filter_map(|symbol| {
            let score = symbol_similarity_score(&symbol.name, query);
            (score > 0).then_some(SymbolSuggestion { symbol, score })
        })
        .collect::<Vec<_>>();

    suggestions.sort_by(|left, right| {
        right.score.cmp(&left.score).then_with(|| {
            left.symbol
                .name
                .to_ascii_lowercase()
                .cmp(&right.symbol.name.to_ascii_lowercase())
        })
    });
    suggestions.truncate(limit);
    suggestions
}

fn symbol_similarity_score(name: &str, query: &str) -> usize {
    let name_norm = normalize_symbol_name(name);
    let query_norm = normalize_symbol_name(query);

    if name_norm.is_empty() || query_norm.is_empty() {
        return 0;
    }

    if name_norm == query_norm {
        return 100;
    }

    if name_norm.contains(&query_norm) || query_norm.contains(&name_norm) {
        let shorter = name_norm.len().min(query_norm.len());
        let longer = name_norm.len().max(query_norm.len());
        return 80 + (shorter * 20 / longer.max(1));
    }

    let token_score = symbol_token_overlap_score(name, query);
    let subsequence = common_subsequence_len(&name_norm, &query_norm);
    let common_prefix = common_prefix_len(&name_norm, &query_norm);
    let longer = name_norm.len().max(query_norm.len()).max(1);
    let char_score = ((subsequence * 70) + (common_prefix * 30)) / longer;
    token_score.max(char_score)
}

fn normalize_symbol_name(value: &str) -> String {
    value
        .chars()
        .filter(|char| char.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn common_prefix_len(left: &str, right: &str) -> usize {
    left.chars()
        .zip(right.chars())
        .take_while(|(left, right)| left == right)
        .count()
}

fn common_subsequence_len(left: &str, right: &str) -> usize {
    let right_chars = right.chars().collect::<Vec<_>>();
    let mut cursor = 0usize;
    let mut count = 0usize;

    for left_char in left.chars() {
        if let Some(offset) = right_chars[cursor..]
            .iter()
            .position(|right_char| *right_char == left_char)
        {
            cursor += offset + 1;
            count += 1;
            if cursor >= right_chars.len() {
                break;
            }
        }
    }

    count
}

fn symbol_token_overlap_score(name: &str, query: &str) -> usize {
    let name_tokens = symbol_tokens(name);
    let query_tokens = symbol_tokens(query);

    if name_tokens.is_empty() || query_tokens.is_empty() {
        return 0;
    }

    let matches = name_tokens
        .iter()
        .filter(|name_token| {
            query_tokens
                .iter()
                .any(|query_token| name_token == &query_token)
        })
        .count();

    if matches == 0 {
        return 0;
    }

    let max_tokens = name_tokens.len().max(query_tokens.len());
    30 + (matches * 70 / max_tokens.max(1))
}

fn symbol_tokens(value: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut previous_was_lowercase = false;

    for char in value.chars() {
        if char == '_' || char == '-' || char == ' ' || char == ':' {
            push_symbol_token(&mut tokens, &mut current);
            previous_was_lowercase = false;
            continue;
        }

        if char.is_ascii_uppercase() && previous_was_lowercase {
            push_symbol_token(&mut tokens, &mut current);
        }

        if char.is_ascii_alphanumeric() {
            previous_was_lowercase = char.is_ascii_lowercase() || char.is_ascii_digit();
            current.extend(char.to_lowercase());
        }
    }

    push_symbol_token(&mut tokens, &mut current);
    tokens
}

fn push_symbol_token(tokens: &mut Vec<String>, current: &mut String) {
    if !current.is_empty() {
        tokens.push(std::mem::take(current));
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::{test_file, test_map, test_symbol_with_range};

    use super::{find_symbol_match, symbol_suggestions};

    #[test]
    fn fuzzy_symbol_match_handles_case_and_separator_drift() {
        let file = test_file("src/lib.rs", "src/lib.rs");
        let symbol = test_symbol_with_range("fallback_editor_snapshot", "fn", &file.id, 3, 6);
        let map = test_map(vec![file.clone()], vec![symbol]);
        let symbols = super::symbols_in_file(&map, &file.id);

        let matched = find_symbol_match(&symbols, "fallbackEditorSnapshot")
            .expect("symbol should fuzzy match");

        assert_eq!(matched.name, "fallback_editor_snapshot");
    }

    #[test]
    fn symbol_suggestions_rank_nearby_symbols() {
        let file = test_file("src/input.rs", "src/input.rs");
        let map = test_map(
            vec![file.clone()],
            vec![
                test_symbol_with_range("handle_pointer_move", "fn", &file.id, 3, 6),
                test_symbol_with_range("handle_pointer_down", "fn", &file.id, 8, 11),
                test_symbol_with_range("clear_hover", "fn", &file.id, 13, 14),
            ],
        );
        let symbols = super::symbols_in_file(&map, &file.id);
        let suggestions = symbol_suggestions(&symbols, "handle_pointer_event", 3);

        assert_eq!(suggestions[0].symbol.name, "handle_pointer_move");
        assert!(suggestions[0].score > suggestions[2].score);
    }

    #[test]
    fn symbol_suggestions_use_token_overlap_for_stale_names() {
        let file = test_file("src/snapshot.rs", "src/snapshot.rs");
        let map = test_map(
            vec![file.clone()],
            vec![test_symbol_with_range(
                "fallback_editor_snapshot",
                "fn",
                &file.id,
                3,
                6,
            )],
        );
        let symbols = super::symbols_in_file(&map, &file.id);
        let suggestions = symbol_suggestions(&symbols, "build_editor_scene_snapshot", 1);

        assert_eq!(suggestions[0].symbol.name, "fallback_editor_snapshot");
        assert!(suggestions[0].score >= 60);
    }
}
