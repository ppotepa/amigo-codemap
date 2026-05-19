use std::path::PathBuf;

use anyhow::Result;

use super::command::Command;

mod command_names;
mod command_spec;
mod parser;
#[cfg(test)]
mod tests;

pub(crate) use command_spec::{
    COMMAND_SPECS, CommandFamily, PositionalMode, ScanPolicy, command_family,
    command_positional_mode, command_scan_policy, command_spec_by_name,
};

#[derive(Debug, Clone)]
pub struct Options {
    pub root: PathBuf,
    pub out: PathBuf,
    pub level: u8,
    pub pretty: bool,
    pub ai: bool,
    pub query: Option<String>,
    pub start_line: Option<usize>,
    pub end_line: Option<usize>,
    pub group: Option<String>,
    pub lines: bool,
    pub line_range: Option<String>,
    pub limit: usize,
    pub min_score: usize,
    pub report: bool,
    pub report_file: Option<PathBuf>,
    pub file_lines: usize,
    pub verify_args: Vec<String>,
    pub changed_only: bool,
    pub patterns: Vec<String>,
    pub file: Option<PathBuf>,
    pub name: Option<String>,
    pub kind: Option<String>,
    pub owner: Option<String>,
    pub visibility: Option<String>,
    pub from: Option<PathBuf>,
    pub yaml: Option<String>,
    pub yaml_op: String,
    pub by: Option<String>,
    pub to: Option<PathBuf>,
    pub symbol: Option<String>,
    pub with_file: Option<PathBuf>,
    pub with_text: Option<String>,
    pub task: Option<String>,
    pub from_impact: Option<String>,
    pub radius: usize,
    pub context_radius: usize,
    pub top: usize,
    pub with_split_hints: bool,
    pub timings: bool,
    pub progress: bool,
    pub diagnostics: bool,
    pub slow_file_threshold_ms: u64,
    pub max_file_size_bytes: u64,
    pub max_files: usize,
    pub save: bool,
    pub status: bool,
    pub write: bool,
    pub strict: bool,
    pub backup: bool,
    pub stop_on_error: bool,
    pub run: bool,
    pub why: bool,
    pub metadata: bool,
    pub json: bool,
    pub raw: bool,
    pub no_verbose: bool,
    pub quiet: bool,
    pub no_cache: bool,
    pub compact: bool,
    pub hide_generated: bool,
    pub include_tests: bool,
    pub include_generated: bool,
    pub warnings: bool,
    pub expect_present: Vec<String>,
    pub expect_absent: Vec<String>,
    pub daemon_mode: DaemonMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonMode {
    Auto,
    Require,
    Disabled,
}

#[derive(Debug, Clone)]
pub struct Cli {
    pub command: Command,
    pub options: Options,
}

impl Cli {
    pub fn parse<I>(args: I) -> Result<Self>
    where
        I: IntoIterator<Item = String>,
    {
        parser::parse(args)
    }
}
