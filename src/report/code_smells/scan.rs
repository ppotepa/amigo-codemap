use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::Result;

use crate::model::{CodeMap, FileEntry, SymbolEntry};

use super::classify::{is_generated_file, is_rankable_file, is_test_file, slash_path};
use super::model::{CodeSmellFinding, SmellMetrics, SmellOptions};
use super::scoring::rank_file;

pub(super) fn collect_code_smells(
    root: &Path,
    map: &CodeMap,
    options: &SmellOptions,
) -> Result<Vec<CodeSmellFinding>> {
    let changed = map
        .git
        .changed
        .iter()
        .filter_map(|change| change.file_id.as_deref())
        .collect::<BTreeSet<_>>();
    let symbols_by_file = symbols_by_file(map);
    let incoming = incoming_deps(map);
    let outgoing = outgoing_deps(map);
    let file_filter = options.file.as_ref().map(|path| slash_path(path));

    let mut findings = Vec::new();
    for file in &map.files {
        let path = slash_path(&file.path);
        if file_filter.as_deref().is_some_and(|filter| filter != path) {
            continue;
        }
        if options.changed_only && !changed.contains(file.id.as_str()) {
            continue;
        }
        if !options.include_tests && is_test_file(&path) {
            continue;
        }
        if !options.include_generated && is_generated_file(file, &path) {
            continue;
        }
        if !is_rankable_file(&path) {
            continue;
        }

        let text = fs::read_to_string(root.join(&file.path)).unwrap_or_default();
        let file_symbols = symbols_by_file
            .get(file.id.as_str())
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let metrics = measure_file(
            &text,
            file,
            file_symbols,
            changed.contains(file.id.as_str()),
            outgoing.get(file.id.as_str()).copied().unwrap_or_default(),
            incoming.get(file.id.as_str()).copied().unwrap_or_default(),
        );
        let finding = rank_file(file, &path, &metrics, options.file_lines);
        if finding.score >= options.min_score {
            findings.push(finding);
        }
    }

    findings.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| right.metrics.lines.cmp(&left.metrics.lines))
    });
    Ok(findings)
}

fn measure_file(
    text: &str,
    file: &FileEntry,
    symbols: &[&SymbolEntry],
    changed: bool,
    outgoing_deps: usize,
    incoming_deps: usize,
) -> SmellMetrics {
    let branch_tokens = count_branch_tokens(text);
    let largest = largest_body(text, symbols);
    let largest_block = largest.as_ref().map(|(name, _, _)| name.clone());
    let largest_block_line = largest.as_ref().map(|(_, line, _)| *line);
    let largest_block_lines = largest.map(|(_, _, lines)| lines).unwrap_or_default();
    let exports = symbols
        .iter()
        .filter(|symbol| symbol.visibility == "pub" || symbol.visibility == "export")
        .count();
    let lines = file.lines.max(text.lines().count());
    SmellMetrics {
        lines,
        symbols: symbols.len(),
        exports,
        largest_block_line,
        branch_tokens,
        branch_density: if lines == 0 {
            0.0
        } else {
            branch_tokens as f32 * 100.0 / lines as f32
        },
        largest_block,
        largest_block_lines,
        outgoing_deps,
        incoming_deps,
        todo_markers: count_any(text, &["TODO"]),
        fixme_markers: count_any(text, &["FIXME"]),
        hack_markers: count_any(text, &["HACK"]),
        unwrap_markers: count_any(text, &[".unwrap("]),
        expect_markers: count_any(text, &[".expect("]),
        unwrap_expect_markers: count_any(text, &[".unwrap(", ".expect("]),
        panic_markers: count_any(text, &["panic!", "todo!", "unimplemented!"]),
        changed,
    }
}

fn largest_body(text: &str, symbols: &[&SymbolEntry]) -> Option<(String, usize, usize)> {
    symbols
        .iter()
        .filter(|symbol| {
            matches!(symbol.kind.as_str(), "fn" | "method" | "component" | "hook")
                && !symbol
                    .owner
                    .as_deref()
                    .is_some_and(|owner| owner.starts_with("impl "))
        })
        .filter_map(|symbol| {
            let body_lines = brace_body_line_count(text, symbol.line).unwrap_or(symbol.line_count);
            (body_lines > 0).then(|| (symbol.name.clone(), symbol.line, body_lines))
        })
        .max_by_key(|(_, _, lines)| *lines)
}

fn brace_body_line_count(text: &str, start_line: usize) -> Option<usize> {
    let lines = text.lines().collect::<Vec<_>>();
    let start_index = start_line.saturating_sub(1);
    let mut depth = 0isize;
    let mut saw_open = false;
    for (offset, line) in lines.iter().enumerate().skip(start_index) {
        for ch in line.chars() {
            match ch {
                '{' => {
                    depth += 1;
                    saw_open = true;
                }
                '}' if saw_open => {
                    depth -= 1;
                    if depth <= 0 {
                        return Some(offset.saturating_sub(start_index) + 1);
                    }
                }
                _ => {}
            }
        }
    }
    None
}

fn symbols_by_file(map: &CodeMap) -> BTreeMap<&str, Vec<&SymbolEntry>> {
    let mut by_file = BTreeMap::<&str, Vec<&SymbolEntry>>::new();
    for symbol in &map.symbols {
        by_file
            .entry(symbol.file_id.as_str())
            .or_default()
            .push(symbol);
    }
    by_file
}

fn outgoing_deps(map: &CodeMap) -> BTreeMap<&str, usize> {
    let mut counts = BTreeMap::new();
    for dep in &map.dependencies {
        *counts.entry(dep.from.as_str()).or_insert(0) += 1;
    }
    counts
}

fn incoming_deps(map: &CodeMap) -> BTreeMap<&str, usize> {
    let mut counts = BTreeMap::new();
    for dep in &map.dependencies {
        *counts.entry(dep.to.as_str()).or_insert(0) += 1;
    }
    counts
}

fn count_branch_tokens(text: &str) -> usize {
    count_any(
        text,
        &[
            "if ", "else if", "match ", "for ", "while ", "case ", "catch ", "switch ", "&&", "||",
            "?", "await ", ".unwrap(", ".expect(", "panic!",
        ],
    )
}

fn count_any(text: &str, needles: &[&str]) -> usize {
    needles
        .iter()
        .map(|needle| text.matches(needle).count())
        .sum()
}
