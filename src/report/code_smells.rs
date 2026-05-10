use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::Serialize;

use crate::model::{CodeMap, FileEntry, SymbolEntry};

#[derive(Debug, Clone)]
pub struct SmellOptions {
    pub top: usize,
    pub changed_only: bool,
    pub group: Option<String>,
    pub min_score: usize,
    pub report: bool,
    pub file_lines: usize,
    pub json: bool,
    pub why: bool,
    pub include_tests: bool,
    pub include_generated: bool,
    pub file: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
struct CodeSmellFinding {
    path: String,
    line: usize,
    symbol: Option<String>,
    score: usize,
    severity: &'static str,
    confidence: &'static str,
    smells: Vec<String>,
    metrics: SmellMetrics,
    reasons: Vec<String>,
    next_actions: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
struct SmellMetrics {
    lines: usize,
    symbols: usize,
    exports: usize,
    branch_tokens: usize,
    branch_density: f32,
    largest_block: Option<String>,
    largest_block_lines: usize,
    outgoing_deps: usize,
    incoming_deps: usize,
    todo_markers: usize,
    unwrap_expect_markers: usize,
    panic_markers: usize,
    changed: bool,
}

/// @codemap(P1): codemap-code-smells
/// Refactor Radar ranking for likely technical debt hotspots.
pub fn print_code_smells(root: &Path, map: &CodeMap, options: SmellOptions) -> Result<()> {
    let findings = collect_code_smells(root, map, &options)?;

    if options.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "code_smells": findings,
            }))?
        );
        return Ok(());
    }

    if let Some(group) = options.group.as_deref() {
        print_grouped_smells(&findings, group);
        return Ok(());
    }

    if options.report {
        print_report_summary(&findings);
    }

    println!("code-smells:");
    let limit = if options.report {
        findings.len()
    } else {
        options.top
    };
    for (index, finding) in findings.iter().take(limit).enumerate() {
        println!("  {}. {}", index + 1, finding.path);
        println!("     score: {}", finding.score);
        println!("     severity: {}", finding.severity);
        println!("     confidence: {}", finding.confidence);
        if let Some(symbol) = &finding.symbol {
            println!("     symbol: {symbol}");
        }
        println!("     smells:");
        for smell in &finding.smells {
            println!("       - {smell}");
        }
        println!("     metrics:");
        println!("       lines: {}", finding.metrics.lines);
        println!("       symbols: {}", finding.metrics.symbols);
        println!("       branch_tokens: {}", finding.metrics.branch_tokens);
        println!(
            "       branch_density: {:.1} per 100 LOC",
            finding.metrics.branch_density
        );
        if let Some(block) = &finding.metrics.largest_block {
            println!(
                "       largest_block: {}, {} lines",
                block, finding.metrics.largest_block_lines
            );
        }
        println!("       outgoing_deps: {}", finding.metrics.outgoing_deps);
        println!("       incoming_deps: {}", finding.metrics.incoming_deps);
        if options.why {
            println!("     reasons:");
            for reason in &finding.reasons {
                println!("       - {reason}");
            }
            println!("     next:");
            for next in &finding.next_actions {
                println!("       - {next}");
            }
        }
    }
    Ok(())
}

fn collect_code_smells(
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

fn rank_file(
    file: &FileEntry,
    path: &str,
    metrics: &SmellMetrics,
    file_lines_threshold: usize,
) -> CodeSmellFinding {
    let mut smells = Vec::new();
    let mut reasons = Vec::new();
    let too_large_threshold = file_lines_threshold.saturating_add(250);
    let god_file_threshold = file_lines_threshold.saturating_add(550);

    if metrics.lines > god_file_threshold {
        push(&mut smells, "file.god_file");
        reasons.push(format!("file has more than {god_file_threshold} lines"));
    } else if metrics.lines > too_large_threshold {
        push(&mut smells, "file.too_large");
        reasons.push(format!("file has more than {too_large_threshold} lines"));
    } else if metrics.lines > file_lines_threshold {
        push(&mut smells, "file.large");
        reasons.push(format!("file has more than {file_lines_threshold} lines"));
    }

    if metrics.largest_block_lines > 250 {
        push(&mut smells, component_or_function_smell(path, "critical"));
        reasons.push("largest symbol body has more than 250 lines".to_string());
    } else if metrics.largest_block_lines > 150 {
        push(&mut smells, component_or_function_smell(path, "high"));
        reasons.push("largest symbol body has more than 150 lines".to_string());
    } else if metrics.largest_block_lines > 80 {
        push(&mut smells, "symbol.long_body");
        reasons.push("largest symbol body has more than 80 lines".to_string());
    }

    if metrics.branch_tokens > 120 {
        push(&mut smells, "complexity.high_branch_count");
        reasons.push("file has a high branch-token count".to_string());
    }
    if metrics.branch_density >= 8.0 {
        push(&mut smells, "complexity.high_branch_density");
        reasons.push("branch-token density is high for the file size".to_string());
    }
    if metrics.symbols > 120 {
        push(&mut smells, "surface.too_many_symbols");
        reasons.push("file exposes or contains more than 120 symbols".to_string());
    } else if metrics.symbols > 70 {
        push(&mut smells, "module.mixed_responsibility");
        reasons.push("file contains many symbols in one surface".to_string());
    }
    if metrics.outgoing_deps > 30 || metrics.incoming_deps > 40 {
        push(&mut smells, "coupling.hub_file");
        reasons.push("file has high incoming or outgoing dependency count".to_string());
    } else if metrics.outgoing_deps > 18 {
        push(&mut smells, "coupling.high_fanout");
        reasons.push("file imports or depends on many other files".to_string());
    }
    if metrics.todo_markers >= 10 {
        push(&mut smells, "marker.todo_cluster");
        reasons.push("file contains a TODO/FIXME/HACK marker cluster".to_string());
    }
    if metrics.unwrap_expect_markers >= 8 {
        push(&mut smells, "rust.unwrap_expect_cluster");
        reasons.push("file contains an unwrap/expect cluster".to_string());
    }
    if metrics.panic_markers > 0 && !is_test_file(path) {
        push(&mut smells, "error_handling.panic_in_runtime_path");
        reasons.push("runtime path contains panic markers".to_string());
    }
    if metrics.changed && (metrics.lines > 450 || metrics.branch_tokens > 80) {
        push(&mut smells, "change_risk.hotspot");
        reasons.push("changed file also has size or complexity risk".to_string());
    }
    if is_catalog_file(path) && metrics.lines > 700 {
        push(&mut smells, "descriptor_catalog.large");
        reasons.push("large catalog-like file; refactor confidence is lower".to_string());
    }

    let score = score(metrics, path, file_lines_threshold);
    let symbol = metrics.largest_block.clone();
    CodeSmellFinding {
        path: path.to_string(),
        line: largest_symbol_line(file, symbol.as_deref()).unwrap_or(1),
        symbol,
        score,
        severity: severity(score),
        confidence: confidence(file, path, metrics),
        smells,
        metrics: metrics.clone(),
        reasons,
        next_actions: next_actions(path, metrics),
    }
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
    let largest_block = largest.as_ref().map(|(name, _)| name.clone());
    let largest_block_lines = largest.map(|(_, lines)| lines).unwrap_or_default();
    let exports = symbols
        .iter()
        .filter(|symbol| symbol.visibility == "pub" || symbol.visibility == "export")
        .count();
    let lines = file.lines.max(text.lines().count());
    SmellMetrics {
        lines,
        symbols: symbols.len(),
        exports,
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
        todo_markers: count_any(text, &["TODO", "FIXME", "HACK"]),
        unwrap_expect_markers: count_any(text, &[".unwrap(", ".expect("]),
        panic_markers: count_any(text, &["panic!", "todo!", "unimplemented!"]),
        changed,
    }
}

fn largest_body(text: &str, symbols: &[&SymbolEntry]) -> Option<(String, usize)> {
    symbols
        .iter()
        .filter(|symbol| {
            matches!(
                symbol.kind.as_str(),
                "fn" | "method" | "component" | "hook" | "impl"
            )
        })
        .filter_map(|symbol| {
            let body_lines = brace_body_line_count(text, symbol.line).unwrap_or(symbol.line_count);
            (body_lines > 0).then(|| (symbol.name.clone(), body_lines))
        })
        .max_by_key(|(_, lines)| *lines)
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

fn score(metrics: &SmellMetrics, path: &str, file_lines_threshold: usize) -> usize {
    let file_size = scale(
        metrics.lines,
        &[
            (file_lines_threshold, 8),
            (file_lines_threshold.saturating_add(250), 16),
            (file_lines_threshold.saturating_add(550), 25),
        ],
    );
    let symbol_size = scale(
        metrics.largest_block_lines,
        &[(80, 8), (150, 16), (250, 25)],
    );
    let complexity = scale(metrics.branch_tokens, &[(40, 6), (80, 12), (120, 20)]);
    let coupling = scale(
        metrics.outgoing_deps.max(metrics.incoming_deps),
        &[(12, 5), (24, 10), (40, 15)],
    );
    let change = if metrics.changed && (metrics.lines > 450 || metrics.branch_tokens > 60) {
        10
    } else if metrics.changed {
        4
    } else {
        0
    };
    let markers = scale(
        metrics.todo_markers + metrics.unwrap_expect_markers + metrics.panic_markers,
        &[(3, 2), (8, 4), (15, 5)],
    );
    let mut total = file_size + symbol_size + complexity + coupling + change + markers;
    if is_catalog_file(path) {
        total = total.saturating_sub(10);
    }
    total.min(100)
}

fn scale(value: usize, thresholds: &[(usize, usize)]) -> usize {
    thresholds
        .iter()
        .filter_map(|(threshold, score)| (value >= *threshold).then_some(*score))
        .max()
        .unwrap_or_default()
}

fn print_grouped_smells(findings: &[CodeSmellFinding], group: &str) {
    let mut groups = BTreeMap::<String, Vec<&CodeSmellFinding>>::new();
    for finding in findings {
        for key in group_keys(finding, group) {
            groups.entry(key).or_default().push(finding);
        }
    }
    println!("code-smells:{group}");
    for (name, mut items) in groups {
        items.sort_by(|left, right| right.score.cmp(&left.score));
        let max_score = items
            .iter()
            .map(|item| item.score)
            .max()
            .unwrap_or_default();
        println!("  {name}: count={} max_score={}", items.len(), max_score);
        for item in items.into_iter().take(3) {
            println!("    - {} score={}", item.path, item.score);
        }
    }
}

fn print_report_summary(findings: &[CodeSmellFinding]) {
    println!("code-smells-report:");
    println!("  total: {}", findings.len());
    for group in ["severity", "domain", "smell"] {
        let mut counts = BTreeMap::<String, usize>::new();
        for finding in findings {
            for key in group_keys(finding, group) {
                *counts.entry(key).or_default() += 1;
            }
        }
        println!("  by_{group}:");
        for (key, count) in counts {
            println!("    {key}: {count}");
        }
    }
}

fn group_keys(finding: &CodeSmellFinding, group: &str) -> Vec<String> {
    match group {
        "severity" => vec![finding.severity.to_string()],
        "language" => vec![extension(&finding.path).to_string()],
        "smell" => finding.smells.clone(),
        "path" => vec![
            finding
                .path
                .split('/')
                .take(3)
                .collect::<Vec<_>>()
                .join("/"),
        ],
        "domain" => vec![domain_for_path(&finding.path).to_string()],
        "package" => vec![package_for_path(&finding.path).to_string()],
        _ => vec!["all".to_string()],
    }
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

fn is_rankable_file(path: &str) -> bool {
    !path.ends_with("Cargo.lock")
        && !path.ends_with("package-lock.json")
        && !path.ends_with(".png")
        && !path.ends_with(".jpg")
        && !path.ends_with(".webp")
}

fn is_generated_file(file: &FileEntry, path: &str) -> bool {
    file.tags.iter().any(|tag| tag == "kind:generated")
        || path.contains("/gen/")
        || path.contains(".generated.")
        || path.ends_with("-schema.json")
}

fn is_test_file(path: &str) -> bool {
    path.contains("/tests/")
        || path.ends_with("_test.rs")
        || path.ends_with(".test.ts")
        || path.ends_with(".test.tsx")
        || path.ends_with(".spec.ts")
        || path.ends_with(".spec.tsx")
}

fn is_catalog_file(path: &str) -> bool {
    path.contains("descriptor")
        || path.contains("registry")
        || path.ends_with("dto.rs")
        || path.ends_with("dto.ts")
        || path.ends_with("-schema.json")
}

fn component_or_function_smell(path: &str, level: &str) -> &'static str {
    if path.ends_with(".tsx") && level == "critical" {
        "component.god_component"
    } else if path.ends_with(".tsx") {
        "component.too_large"
    } else if path.ends_with(".rs") {
        "method.too_long"
    } else {
        "function.too_long"
    }
}

fn severity(score: usize) -> &'static str {
    match score {
        0..=29 => "info",
        30..=49 => "low",
        50..=69 => "medium",
        70..=84 => "high",
        _ => "critical",
    }
}

fn confidence(file: &FileEntry, path: &str, metrics: &SmellMetrics) -> &'static str {
    if is_catalog_file(path) {
        "medium"
    } else if matches!(file.language.as_str(), "rs" | "ts" | "tsx")
        && metrics.largest_block_lines > 0
    {
        "high"
    } else {
        "medium"
    }
}

fn next_actions(path: &str, metrics: &SmellMetrics) -> Vec<String> {
    let mut next = vec![format!(
        "run: amigo-codemap open-set \"{path}\" --why --limit 10"
    )];
    if let Some(symbol) = &metrics.largest_block {
        next.push(format!("run: amigo-codemap slice {path} --symbol {symbol}"));
    }
    if path.ends_with(".tsx") {
        next.push("split shell/layout/actions/hooks/services".to_string());
    } else if path.ends_with(".rs") {
        next.push("split validation/apply/model helpers by responsibility".to_string());
    } else {
        next.push("split catalog/data and behavior into separate modules".to_string());
    }
    next
}

fn largest_symbol_line(_file: &FileEntry, _symbol: Option<&str>) -> Option<usize> {
    None
}

fn push(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|item| item == value) {
        values.push(value.to_string());
    }
}

fn domain_for_path(path: &str) -> &str {
    if path.contains("amigo-codemap") {
        "codemap"
    } else if path.contains("amigo-editor") {
        "editor"
    } else if path.contains("/engine/") {
        "engine"
    } else if path.contains("/apps/app/") {
        "runtime-app"
    } else {
        "unknown"
    }
}

fn package_for_path(path: &str) -> &str {
    path.split("/src/").next().unwrap_or(path)
}

fn extension(path: &str) -> &str {
    path.rsplit('.').next().unwrap_or("unknown")
}

fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::{SmellMetrics, score, severity};

    #[test]
    fn severity_maps_score_ranges() {
        assert_eq!(severity(29), "info");
        assert_eq!(severity(30), "low");
        assert_eq!(severity(50), "medium");
        assert_eq!(severity(70), "high");
        assert_eq!(severity(85), "critical");
    }

    #[test]
    fn score_prioritizes_large_complex_changed_files() {
        let metrics = SmellMetrics {
            lines: 1200,
            largest_block_lines: 280,
            branch_tokens: 140,
            outgoing_deps: 35,
            changed: true,
            todo_markers: 5,
            ..Default::default()
        };

        assert!(score(&metrics, "src/main.rs", 450) >= 85);
    }
}
