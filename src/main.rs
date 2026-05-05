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
        | Command::CommitSummary
        | Command::Slice
        | Command::DiffScope
        | Command::DeletePlan
        | Command::FileMovePlan
        | Command::RenamePlan
        | Command::ImportFixPlan
        | Command::BarrelCheck
        | Command::ShimCheck
        | Command::AssetFileCheck
        | Command::CaseCheck
        | Command::TextCheck
        | Command::CommitFiles => {
            cli.options.level = 0;
            cli.options.ai = false;
        }
        Command::Scope | Command::Refs | Command::Impact | Command::ServiceShape
            if cli.options.level < 2 =>
        {
            cli.options.level = 2;
        }
        Command::OpenSet | Command::LargeFiles | Command::PatchPreview
            if cli.options.level < 2 =>
        {
            cli.options.level = 2;
        }
        Command::Workset if (cli.options.from_impact.is_some() || cli.options.status) && cli.options.level < 2 => {
            cli.options.level = 2;
        }
        Command::OrphanFiles if cli.options.level < 3 => {
            cli.options.level = 3;
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
        Command::Slice => {
            let map = scan::scan_project(&cli.options)?;
            let query = cli
                .options
                .query
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("slice requires a file path"))?;
            report::file_ops::slice::print_slice(
                &cli.options.root,
                &map,
                query,
                cli.options.symbol.as_deref(),
                cli.options.radius,
            )?;
        }
        Command::DiffScope => {
            let map = scan::scan_project(&cli.options)?;
            report::file_ops::diff_scope::print_diff_scope(&map, cli.options.limit);
        }
        Command::DeletePlan => {
            let map = scan::scan_project(&cli.options)?;
            let query = cli
                .options
                .query
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("delete-plan requires a path"))?;
            report::file_ops::delete_plan::print_delete_plan(
                &cli.options.root,
                &map,
                query,
                cli.options.limit,
            )?;
        }
        Command::FileMovePlan => {
            let map = scan::scan_project(&cli.options)?;
            let query = cli
                .options
                .query
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("file-move-plan requires a source file"))?;
            let to = cli
                .options
                .to
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("file-move-plan requires --to"))?;
            report::file_ops::file_move_plan::print_file_move_plan(
                &cli.options.root,
                &map,
                query,
                to,
                cli.options.limit,
            )?;
        }
        Command::RenamePlan => {
            let map = scan::scan_project(&cli.options)?;
            let old = cli
                .options
                .query
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("rename-plan requires old symbol"))?;
            let new_name = cli.options.to.as_ref().map(|path| {
                if path.exists() {
                    path.to_string_lossy().to_string()
                } else {
                    path.to_string_lossy().to_string()
                }
            });
            report::file_ops::rename_plan::print_rename_plan(
                &cli.options.root,
                &map,
                old,
                new_name.as_deref(),
                cli.options.group.as_deref(),
                cli.options.limit,
            )?;
        }
        Command::ImportFixPlan => {
            let map = scan::scan_project(&cli.options)?;
            report::file_ops::import_fix_plan::print_import_fix_plan(
                &cli.options.root,
                &map,
                cli.options.changed_only,
                cli.options.limit,
            )?;
        }
        Command::OpenSet => {
            let map = scan::scan_project(&cli.options)?;
            let query = cli
                .options
                .query
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("open-set requires a query"))?;
            report::file_ops::open_set::print_open_set(
                &cli.options.root,
                &map,
                query,
                cli.options.task.as_deref(),
                cli.options.limit,
            )?;
        }
        Command::Workset => {
            let map = scan::scan_project(&cli.options)?;
            let name = cli
                .options
                .query
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("workset requires a name"))?;
            report::file_ops::workset::print_workset(
                &cli.options.root,
                &map,
                name,
                cli.options.task.as_deref(),
                cli.options.from_impact.as_deref(),
                cli.options.save,
                cli.options.status,
            )?;
        }
        Command::BarrelCheck => {
            let map = scan::scan_project(&cli.options)?;
            let query = cli
                .options
                .query
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("barrel-check requires a path"))?;
            report::file_ops::barrel_check::print_barrel_check(
                &cli.options.root,
                &map,
                query,
                cli.options.limit,
            )?;
        }
        Command::OrphanFiles => {
            let map = scan::scan_project(&cli.options)?;
            let query = cli
                .options
                .query
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("orphan-files requires a prefix path"))?;
            report::file_ops::orphan_files::print_orphan_files(
                &cli.options.root,
                &map,
                query,
                cli.options.limit,
            )?;
        }
        Command::ShimCheck => {
            let map = scan::scan_project(&cli.options)?;
            report::file_ops::shim_check::print_shim_check(
                &cli.options.root,
                &map,
                cli.options.changed_only,
                cli.options.limit,
            )?;
        }
        Command::LargeFiles => {
            let map = scan::scan_project(&cli.options)?;
            report::file_ops::large_files::print_large_files(
                &map,
                cli.options.top.max(1),
                cli.options.with_split_hints,
            );
        }
        Command::AssetFileCheck => {
            let query = cli
                .options
                .query
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("asset-file-check requires a query"))?;
            report::file_ops::asset_file_check::print_asset_file_check(
                &cli.options.root,
                query,
                cli.options.limit,
            )?;
        }
        Command::CaseCheck => {
            let map = scan::scan_project(&cli.options)?;
            report::file_ops::case_check::print_case_check(
                &cli.options.root,
                &map,
                cli.options.changed_only,
                cli.options.limit,
            )?;
        }
        Command::TextCheck => {
            let map = scan::scan_project(&cli.options)?;
            report::file_ops::text_check::print_text_check(
                &cli.options.root,
                &map,
                cli.options.changed_only,
                cli.options.limit,
            );
        }
        Command::PatchPreview => {
            let map = scan::scan_project(&cli.options)?;
            report::file_ops::patch_preview::print_patch_preview(
                &cli.options.root,
                &map,
                cli.options.from.as_deref(),
                cli.options.limit,
            )?;
        }
        Command::CommitFiles => {
            let map = scan::scan_project(&cli.options)?;
            report::file_ops::commit_files::print_commit_files(
                &cli.options.root,
                &map,
                cli.options.changed_only,
                cli.options.limit,
            )?;
        }
        Command::Explain => cli::print_help(),
    }

    Ok(())
}
