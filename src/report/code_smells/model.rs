use std::path::PathBuf;

use serde::Serialize;

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
    pub report_file: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct CodeSmellFinding {
    pub(super) path: String,
    pub(super) line: usize,
    pub(super) symbol: Option<String>,
    pub(super) score: usize,
    pub(super) severity: &'static str,
    pub(super) confidence: &'static str,
    pub(super) smells: Vec<String>,
    pub(super) metrics: SmellMetrics,
    pub(super) reasons: Vec<String>,
    pub(super) next_actions: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub(super) struct SmellMetrics {
    pub(super) lines: usize,
    pub(super) symbols: usize,
    pub(super) exports: usize,
    pub(super) largest_block_line: Option<usize>,
    pub(super) branch_tokens: usize,
    pub(super) branch_density: f32,
    pub(super) largest_block: Option<String>,
    pub(super) largest_block_lines: usize,
    pub(super) outgoing_deps: usize,
    pub(super) incoming_deps: usize,
    pub(super) todo_markers: usize,
    pub(super) fixme_markers: usize,
    pub(super) hack_markers: usize,
    pub(super) unwrap_markers: usize,
    pub(super) expect_markers: usize,
    pub(super) unwrap_expect_markers: usize,
    pub(super) panic_markers: usize,
    pub(super) changed: bool,
}
