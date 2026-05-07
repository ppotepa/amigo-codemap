use std::path::Path;

use anyhow::Result;

use crate::model::CodeMap;
use crate::report::anchors::build_anchor_index;
use crate::taxonomy::CodemapTaxonomy;

pub fn print_anchor_check(root: &Path, map: &CodeMap) -> Result<()> {
    let taxonomy = CodemapTaxonomy::try_load(root);
    let index = build_anchor_index(map, taxonomy.as_ref());

    println!("anchor-check:");
    println!("  anchors: {}", index.counts.anchors);
    println!("  errors: {}", index.counts.errors);
    println!("  warnings: {}", index.counts.warnings);

    for diagnostic in &index.diagnostics {
        println!(
            "{} {} anchor={} file={}:{} {}",
            diagnostic.severity,
            diagnostic.kind,
            diagnostic.anchor.as_deref().unwrap_or("-"),
            diagnostic.file.as_deref().unwrap_or("-"),
            diagnostic
                .line
                .map(|line| line.to_string())
                .unwrap_or_else(|| "-".to_string()),
            diagnostic.message
        );
    }

    if index.counts.errors == 0 {
        println!("anchor-check: ok");
    }

    Ok(())
}
