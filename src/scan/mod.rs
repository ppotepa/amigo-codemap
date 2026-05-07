//! Thin CLI adapter around amigo-symbol-explorer scanning.

use amigo_symbol_explorer::scan::SymbolExplorerScanOptions;
pub use amigo_symbol_explorer::scan::language_for;

use anyhow::Result;

pub fn scan_project(options: &crate::cli::Options) -> Result<crate::model::CodeMap> {
    let scan_options = SymbolExplorerScanOptions {
        root: options.root.clone(),
        level: options.level,
        ai: options.ai,
    };
    amigo_symbol_explorer::scan::scan_project(&scan_options)
}
