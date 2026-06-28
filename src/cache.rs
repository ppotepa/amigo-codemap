use anyhow::Result;

use crate::cli::Options;
use crate::incremental::{WorkspaceIndex, current_changed_paths, write_outputs};
use crate::snapshot_store;

pub fn refresh_changed_only(options: &Options) -> Result<bool> {
    if let Some(envelope) = snapshot_store::read_snapshot(options)?
        && snapshot_store::snapshot_is_usable(options, &envelope)
    {
        let touched = current_changed_paths(&options.root)?;
        if !touched.is_empty() {
            let mut index = WorkspaceIndex::from_map(envelope.map);
            index.refresh_touched(options, &touched)?;
            return write_outputs(options, &index.map);
        }
    }

    snapshot_store::refresh_snapshot(options)
}
