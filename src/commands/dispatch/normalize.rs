use crate::cli::{Cli, ScanPolicy, command_scan_policy};

pub(super) fn apply(cli: &mut Cli) {
    match command_scan_policy(cli.command) {
        ScanPolicy::Preserve => {}
        ScanPolicy::Level0NoAi => {
            cli.options.level = 0;
            cli.options.ai = false;
        }
        ScanPolicy::MinLevel(level) => {
            if cli.options.level < level {
                cli.options.level = level;
            }
        }
        ScanPolicy::ForceLevel(level) => {
            cli.options.level = level;
        }
        ScanPolicy::Slice => apply_slice_policy(cli),
        ScanPolicy::OpsCheck => apply_ops_check_policy(cli),
        ScanPolicy::Workset => apply_workset_policy(cli),
        ScanPolicy::OrphanFiles => {
            if cli.options.level < 3 {
                cli.options.level = 3;
            }
        }
    }
}

fn apply_slice_policy(cli: &mut Cli) {
    if cli.options.symbol.is_some() {
        if cli.options.level < 2 {
            cli.options.level = 2;
        }
    } else {
        cli.options.level = 0;
        cli.options.ai = false;
    }
}

fn apply_ops_check_policy(cli: &mut Cli) {
    if cli.options.strict {
        if cli.options.level < 2 {
            cli.options.level = 2;
        }
    } else {
        cli.options.level = 0;
        cli.options.ai = false;
    }
}

fn apply_workset_policy(cli: &mut Cli) {
    if (cli.options.from_impact.is_some() || cli.options.status) && cli.options.level < 2 {
        cli.options.level = 2;
    }
}
