use super::command_spec::command_spec_by_name;
use crate::cli::Command;

pub(super) fn parse_command_name(value: &str) -> Option<Command> {
    command_spec_by_name(value).map(|spec| spec.command)
}
