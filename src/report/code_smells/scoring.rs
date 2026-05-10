use crate::model::FileEntry;

use super::classify::{
    component_or_function_smell, file_role, is_catalog_file, is_data_table_file,
    is_stylesheet_file, is_test_file, next_actions, push, role_confidence,
};
use super::model::{CodeSmellFinding, SmellMetrics};

pub(super) fn rank_file(
    file: &FileEntry,
    path: &str,
    metrics: &SmellMetrics,
    file_lines_threshold: usize,
) -> CodeSmellFinding {
    let mut smells = Vec::new();
    let mut reasons = Vec::new();
    let role = file_role(path);
    let too_large_threshold = file_lines_threshold.saturating_add(250);
    let god_file_threshold = file_lines_threshold.saturating_add(550);

    if metrics.lines > god_file_threshold {
        push(&mut smells, "file.god_file");
        reasons.push(format!("file has more than {god_file_threshold} lines"));
    } else if metrics.lines > too_large_threshold {
        push(&mut smells, "file.too_large");
        reasons.push(format!("file has more than {too_large_threshold} lines"));
    } else if is_stylesheet_file(path) && metrics.lines > file_lines_threshold.saturating_sub(120) {
        push(&mut smells, "stylesheet.too_large");
        reasons.push("stylesheet is large enough to benefit from tokenization".to_string());
    } else if is_data_table_file(path) && metrics.lines > file_lines_threshold {
        push(&mut smells, "data_table.large");
        reasons.push("data table-like file is large and hard to review".to_string());
    } else if metrics.lines > file_lines_threshold {
        push(&mut smells, "file.large");
        reasons.push(format!("file has more than {file_lines_threshold} lines"));
    }

    if metrics.largest_block_lines > 250 {
        if role == "catalog" || is_data_table_file(path) {
            push(&mut smells, "descriptor_catalog.large");
            reasons.push("large catalog-like file; refactor confidence is lower".to_string());
        } else {
            push(&mut smells, component_or_function_smell(path, "critical"));
        }
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
    if role == "command-handler" && metrics.symbols > 90 {
        push(&mut smells, "surface.too_many_symbols");
        reasons.push("command surface has many symbols".to_string());
    } else if metrics.symbols > 120 {
        push(&mut smells, "surface.too_many_symbols");
        reasons.push("file exposes or contains more than 120 symbols".to_string());
    } else if metrics.symbols > 70 {
        push(&mut smells, "module.mixed_responsibility");
        reasons.push("file contains many symbols in one surface".to_string());
    }
    if metrics.outgoing_deps > 30 || metrics.incoming_deps > 40 {
        push(&mut smells, "coupling.hub_file");
        reasons.push("file has high incoming or outgoing dependency count".to_string());
    } else {
        if metrics.outgoing_deps > 18 {
            push(&mut smells, "coupling.high_fanout");
            reasons.push("file imports or depends on many other files".to_string());
        }
        if metrics.incoming_deps > 25 {
            push(&mut smells, "coupling.high_fanin");
            reasons.push("file is a high-fanin hotspot for many dependents".to_string());
        }
    }
    if metrics.todo_markers >= 10 {
        push(&mut smells, "marker.todo_cluster");
        reasons.push("file contains a TODO marker cluster".to_string());
    }
    if metrics.fixme_markers >= 4 {
        push(&mut smells, "marker.fixme_cluster");
        reasons.push("file contains multiple FIXME markers".to_string());
    }
    if metrics.hack_markers >= 3 {
        push(&mut smells, "marker.hack_cluster");
        reasons.push("file contains multiple HACK markers".to_string());
    }
    if metrics.unwrap_markers >= 6 {
        push(&mut smells, "rust.unwrap_expect_cluster");
        reasons.push("file contains frequent unwrap usage".to_string());
    }
    if metrics.expect_markers >= 4 {
        push(&mut smells, "rust.expect_cluster");
        reasons.push("file contains frequent expect usage".to_string());
    }
    if metrics.unwrap_expect_markers >= 12 {
        push(&mut smells, "rust.unwrap_expect_cluster");
        reasons.push("file contains unwrap/expect clusters".to_string());
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
    if is_stylesheet_file(path) && metrics.lines > 500 {
        push(&mut smells, "stylesheet.too_large");
    }
    if is_data_table_file(path) && metrics.lines > 800 {
        push(&mut smells, "data_table.large");
    }

    let score = score(metrics, path, file_lines_threshold, role);
    let symbol = metrics.largest_block.clone();
    CodeSmellFinding {
        path: path.to_string(),
        line: metrics
            .largest_block_line
            .or_else(|| (file.lines > 0).then_some(file.lines))
            .unwrap_or(1),
        symbol,
        score,
        severity: severity(score),
        confidence: confidence(file, path, metrics, role),
        smells,
        metrics: metrics.clone(),
        reasons,
        next_actions: next_actions(path, metrics),
    }
}

pub(super) fn score(
    metrics: &SmellMetrics,
    path: &str,
    file_lines_threshold: usize,
    role: &str,
) -> usize {
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
        metrics.todo_markers
            + metrics.fixme_markers
            + metrics.hack_markers
            + metrics.unwrap_markers
            + metrics.expect_markers
            + metrics.panic_markers,
        &[(3, 2), (8, 4), (15, 5)],
    );
    let mut total = file_size + symbol_size + complexity + coupling + change + markers;
    if is_catalog_file(path) {
        total = total.saturating_sub(10);
    }
    if role == "command-handler" {
        total = total.saturating_add(2);
    }
    total.min(100)
}

pub(super) fn severity(score: usize) -> &'static str {
    match score {
        0..=29 => "info",
        30..=49 => "low",
        50..=69 => "medium",
        70..=84 => "high",
        _ => "critical",
    }
}

fn confidence(file: &FileEntry, path: &str, metrics: &SmellMetrics, role: &str) -> &'static str {
    if matches!(role, "catalog" | "service") {
        "medium"
    } else if matches!(file.language.as_str(), "rs" | "ts" | "tsx")
        && metrics.largest_block_lines > 0
    {
        role_confidence(path, role)
    } else {
        "medium"
    }
}

fn scale(value: usize, thresholds: &[(usize, usize)]) -> usize {
    thresholds
        .iter()
        .filter_map(|(threshold, score)| (value >= *threshold).then_some(*score))
        .max()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{score, severity};
    use crate::report::code_smells::model::SmellMetrics;

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

        assert!(score(&metrics, "src/main.rs", 450, "service") >= 85);
    }
}
