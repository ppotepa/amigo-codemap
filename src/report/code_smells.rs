use std::path::Path;

use anyhow::Result;

use crate::model::CodeMap;

mod classify;
mod model;
mod output;
mod scan;
mod scoring;

pub use model::SmellOptions;

/// @codemap(P1): codemap-code-smells
/// Refactor Radar ranking for likely technical debt hotspots.
pub fn print_code_smells(root: &Path, map: &CodeMap, options: SmellOptions) -> Result<()> {
    let mut options = options;
    if options.report && options.report_file.is_none() {
        options.report_file = Some(root.join(".amigo").join("code-smells-report.json"));
    }
    if options.report_file.is_some() {
        options.report = true;
    }
    let findings = scan::collect_code_smells(root, map, &options)?;
    output::print_code_smells_output(root, &findings, &options)
}
