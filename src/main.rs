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
        Command::Scope | Command::Refs if cli.options.level < 2 => {
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
        Command::Explain => cli::print_help(),
    }

    Ok(())
}
