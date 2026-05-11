//! Thin CLI adapter around amigo-symbol-explorer scanning.

use amigo_symbol_explorer::scan::SymbolExplorerScanOptions;
pub use amigo_symbol_explorer::scan::language_for;

use anyhow::Result;

pub fn scan_project(options: &crate::cli::Options) -> Result<crate::model::CodeMap> {
    let scan_options = SymbolExplorerScanOptions {
        root: options.root.clone(),
        level: options.level,
        ai: options.ai,
        diagnostics: amigo_symbol_explorer::scan::ScanDiagnostics {
            timings: options.timings,
            progress: options.progress,
            diagnostics: options.diagnostics,
            slow_file_threshold_ms: options.slow_file_threshold_ms,
            max_file_size_bytes: options.max_file_size_bytes,
            max_files: options.max_files,
        },
    };
    amigo_symbol_explorer::scan::scan_project(&scan_options)
}
