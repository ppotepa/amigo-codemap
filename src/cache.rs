use anyhow::Result;

use crate::{cli::Options, output, scan};

pub fn refresh_changed_only(options: &Options) -> Result<bool> {
    let map = scan::scan_project(options)?;
    output::write_codemap(options, &map)
}
