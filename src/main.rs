mod cli;
mod git;
mod model;
mod output;
mod report;
mod scan;
mod watch;

use anyhow::Result;
use cli::{Cli, Command};

fn main() -> Result<()> {
    let mut cli = Cli::parse(std::env::args().skip(1))?;

    match cli.command {
        Command::Brief | Command::Changed | Command::Find | Command::Docs => {
            cli.options.level = 0;
            cli.options.ai = false;
        }
        Command::VerifyPlan
        | Command::Stale
        | Command::Fallout
        | Command::MovePlan
        | Command::Dup
        | Command::TauriCommands
        | Command::RegistryCheck
        | Command::OperationsSummary
        | Command::CommitSummary => {
            cli.options.level = 0;
            cli.options.ai = false;
        }
        Command::Scope | Command::Refs | Command::Impact | Command::ServiceShape if cli.options.level < 2 => {
            cli.options.level = 2;
        }
        _ => {}
    }

    match cli.command {
        Command::Scan => {
            let map = scan::scan_project(&cli.options)?;
            if output::write_codemap(&cli.options, &map)? {
                println!("wrote {}", cli.options.out.display());
            } else {
                println!("unchanged {}", cli.options.out.display());
            }
        }
        Command::Watch => watch::watch_project(cli.options)?,
        Command::Changed => {
            let map = scan::scan_project(&cli.options)?;
            report::print_changed(&map, cli.options.group.as_deref(), cli.options.limit);
        }
        Command::Symbols => {
            let map = scan::scan_project(&cli.options)?;
            for symbol in &map.symbols {
                println!(
                    "{}\t{}\t{}\t{}",
                    symbol.kind, symbol.name, symbol.file_id, symbol.line
                );
            }
        }
        Command::Compact => {
            let mut options = cli.options;
            options.pretty = false;
            options.ai = true;
            let map = scan::scan_project(&options)?;
            if output::write_codemap(&options, &map)? {
                println!("wrote {}", options.out.display());
            } else {
                println!("unchanged {}", options.out.display());
            }
        }
        Command::Brief => {
            let map = scan::scan_project(&cli.options)?;
            report::print_brief(&map);
        }
        Command::Find => {
            let map = scan::scan_project(&cli.options)?;
            let query = cli
                .options
                .query
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("find requires a query"))?;
            report::print_find(
                &cli.options.root,
                &map,
                query,
                cli.options.lines,
                cli.options.limit,
            )?;
        }
        Command::Scope => {
            let map = scan::scan_project(&cli.options)?;
            let query = cli
                .options
                .query
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("scope requires a query"))?;
            report::print_scope(&map, query, cli.options.limit);
        }
        Command::Refs => {
            let map = scan::scan_project(&cli.options)?;
            let query = cli
                .options
                .query
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("refs requires a query"))?;
            report::print_refs(
                &cli.options.root,
                &map,
                query,
                cli.options.lines,
                cli.options.limit,
            )?;
        }
        Command::Docs => {
            let map = scan::scan_project(&cli.options)?;
            report::print_docs(&cli.options.root, &map);
        }
        Command::Verify => {
            report::run_verify(
                &cli.options.root,
                &cli.options.verify_args,
                cli.options.limit,
            )?;
        }
        Command::VerifyPlan => {
            let map = scan::scan_project(&cli.options)?;
            report::verify_plan::print_verify_plan(&map, cli.options.changed_only);
        }
        Command::Stale => {
            let map = scan::scan_project(&cli.options)?;
            report::stale::print_stale(
                &cli.options.root,
                &map,
                &cli.options.patterns,
                cli.options.changed_only,
                cli.options.limit,
            )?;
        }
        Command::Impact => {
            let map = scan::scan_project(&cli.options)?;
            let query = cli
                .options
                .query
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("impact requires a query"))?;
            report::impact::print_impact(
                &cli.options.root,
                &map,
                query,
                cli.options.group.as_deref(),
                cli.options.lines,
                cli.options.limit,
            )?;
        }
        Command::Fallout => {
            report::fallout::print_fallout(cli.options.from.as_ref(), cli.options.limit)?;
        }
        Command::MovePlan => {
            let query = cli
                .options
                .query
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("move-plan requires a query"))?;
            report::move_plan::print_move_plan(
                &cli.options.root,
                query,
                cli.options.by.as_deref(),
                cli.options.limit,
            )?;
        }
        Command::Dup => {
            let map = scan::scan_project(&cli.options)?;
            report::dup::print_dup(
                &cli.options.root,
                &map,
                cli.options.query.as_deref(),
                cli.options.changed_only,
                cli.options.limit,
            )?;
        }
        Command::TauriCommands => {
            report::tauri::print_tauri_commands(&cli.options.root, cli.options.limit)?;
        }
        Command::ServiceShape => {
            let map = scan::scan_project(&cli.options)?;
            let query = cli
                .options
                .query
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("service-shape requires a query"))?;
            report::service_shape::print_service_shape(
                &cli.options.root,
                &map,
                query,
                cli.options.limit,
            )?;
        }
        Command::RegistryCheck => {
            report::registry::print_registry_check(
                &cli.options.root,
                cli.options.query.as_deref(),
                cli.options.limit,
            )?;
        }
        Command::OperationsSummary => {
            report::summary::print_operations_summary(&cli.options.root, cli.options.limit)?;
        }
        Command::CommitSummary => {
            let map = scan::scan_project(&cli.options)?;
            report::summary::print_commit_summary(&map, cli.options.limit);
        }
        Command::Explain => cli::print_help(),
    }

    Ok(())
}
