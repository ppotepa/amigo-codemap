use anyhow::Result;

mod dispatch;

pub(super) fn run(cli: crate::cli::Cli) -> Result<()> {
    dispatch::run(cli)
}
