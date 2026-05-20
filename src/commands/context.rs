use anyhow::{Result, anyhow};

use crate::cli::{Cli, Command, Options};
use crate::model::CodeMap;
pub(super) struct AppContext {
    pub cli: Cli,
    map: Option<CodeMap>,
}
impl AppContext {
    pub(super) fn new(cli: Cli) -> Self {
        Self { cli, map: None }
    }

    pub(super) fn command(&self) -> Command {
        self.cli.command
    }

    pub(super) fn options(&self) -> &Options {
        &self.cli.options
    }

    pub(super) fn map(&mut self) -> Result<&CodeMap> {
        if self.map.is_none() {
            self.map = Some(crate::load_report_map(&self.cli.options)?);
        }

        Ok(self.map.as_ref().expect("map was just loaded"))
    }

    pub(super) fn query(&self, message: &'static str) -> Result<&str> {
        self.cli
            .options
            .query
            .as_deref()
            .ok_or_else(|| anyhow!(message))
    }
}
