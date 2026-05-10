use std::path::Path;

use anyhow::Result;

use crate::model::CodeMap;

use super::ops_plan::{self, OpsInputFormat};

/// @codemap(P0): codemap-raw-ops-input
/// Chat-friendly raw input adapter for file operations.
pub fn print_raw_ops_preview(
    root: &Path,
    from: Option<&Path>,
    raw: Option<&str>,
    limit: usize,
) -> Result<()> {
    ops_plan::print_ops_preview(root, from, raw, OpsInputFormat::Raw, limit)
}

pub fn print_raw_ops_check(
    root: &Path,
    map: Option<&CodeMap>,
    from: Option<&Path>,
    raw: Option<&str>,
    strict: bool,
    limit: usize,
) -> Result<()> {
    ops_plan::print_ops_check(root, map, from, raw, OpsInputFormat::Raw, strict, limit)
}

pub fn print_raw_ops_apply(
    root: &Path,
    map: Option<&CodeMap>,
    from: Option<&Path>,
    raw: Option<&str>,
    write: bool,
    backup: bool,
    stop_on_error: bool,
    strict: bool,
    limit: usize,
    verbose: bool,
) -> Result<()> {
    ops_plan::print_ops_apply(
        root,
        map,
        from,
        raw,
        OpsInputFormat::Raw,
        write,
        backup,
        stop_on_error,
        strict,
        limit,
        verbose,
    )
}

pub fn raw_plan_requires_codemap(
    from: Option<&Path>,
    raw: Option<&str>,
    strict: bool,
) -> Result<bool> {
    ops_plan::plan_requires_codemap(from, raw, OpsInputFormat::Raw, strict)
}
