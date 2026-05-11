mod cache;
mod cli;
mod commands;
pub mod daemon;
mod daemon_client;
mod daemon_protocol;
pub use amigo_symbol_explorer::git;
pub use amigo_symbol_explorer::model;
mod output;
mod incremental;
pub use amigo_symbol_explorer::query;
mod report;
mod scan;
mod snapshot_store;
mod taxonomy;
#[cfg(test)]
mod test_support;
mod watch;

use anyhow::Result;
use cli::Cli;

pub(crate) fn ops_input_format(raw: bool) -> report::file_ops::ops_plan::OpsInputFormat {
    if raw {
        report::file_ops::ops_plan::OpsInputFormat::Raw
    } else {
        report::file_ops::ops_plan::OpsInputFormat::Yaml
    }
}

pub(crate) fn load_report_map(options: &cli::Options) -> Result<model::CodeMap> {
    if let Some(map) = daemon_client::try_load_map(options)? {
        return Ok(map);
    }

    let loaded = snapshot_store::load_or_scan(options)?;
    let _source = loaded.source;
    Ok(loaded.map)
}

pub fn run_cli() -> Result<()> {
    let cli = Cli::parse(std::env::args().skip(1))?;
    commands::run(cli)
}
