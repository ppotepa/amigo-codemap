use anyhow::Result;

use crate::cli::Options;
use crate::snapshot_store;

pub fn refresh_changed_only(options: &Options) -> Result<bool> {
    snapshot_store::refresh_snapshot(options)
}
