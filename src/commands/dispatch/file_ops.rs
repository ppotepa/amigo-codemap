use anyhow::Result;

use crate::cli::{Cli, Command};
use crate::load_report_map;
use crate::ops_input_format;
use crate::report;

pub(super) fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Slice => {
            let map = load_report_map(&cli.options)?;
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
                cli.options.line_range.as_deref(),
                cli.options.radius,
            )?;
        }
        Command::AppendPlan => {
            let map = load_report_map(&cli.options)?;
            let query = cli
                .options
                .query
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("append-plan requires a file path"))?;
            report::file_ops::append_plan::print_append_plan(
                &cli.options.root,
                &map,
                query,
                cli.options.task.as_deref(),
                cli.options.limit,
            )?;
        }
        Command::CopyPlan => {
            let map = load_report_map(&cli.options)?;
            let query = cli
                .options
                .query
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("copy-plan requires a target file path"))?;
            report::file_ops::copy_plan::print_copy_plan(
                &cli.options.root,
                &map,
                query,
                cli.options.from.as_deref(),
                cli.options.task.as_deref(),
                cli.options.limit,
            )?;
        }
        Command::DiffScope => {
            let map = load_report_map(&cli.options)?;
            report::file_ops::diff_scope::print_diff_scope(&map, cli.options.limit);
        }
        Command::DeletePlan => {
            let map = load_report_map(&cli.options)?;
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
            let map = load_report_map(&cli.options)?;
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
            let map = load_report_map(&cli.options)?;
            let old = cli
                .options
                .query
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("rename-plan requires old symbol"))?;
            let new_name = cli
                .options
                .to
                .as_ref()
                .map(|path| path.to_string_lossy().to_string());
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
            let map = load_report_map(&cli.options)?;
            report::file_ops::import_fix_plan::print_import_fix_plan(
                &cli.options.root,
                &map,
                cli.options.changed_only,
                cli.options.limit,
            )?;
        }
        Command::OpenSet => {
            let map = load_report_map(&cli.options)?;
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
                cli.options.why,
            )?;
        }
        Command::Workset => {
            let map = load_report_map(&cli.options)?;
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
            let map = load_report_map(&cli.options)?;
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
            let map = load_report_map(&cli.options)?;
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
            let map = load_report_map(&cli.options)?;
            report::file_ops::shim_check::print_shim_check(
                &cli.options.root,
                &map,
                cli.options.changed_only,
                cli.options.limit,
            )?;
        }
        Command::LargeFiles => {
            let map = load_report_map(&cli.options)?;
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
            let map = load_report_map(&cli.options)?;
            report::file_ops::case_check::print_case_check(
                &cli.options.root,
                &map,
                cli.options.changed_only,
                cli.options.limit,
            )?;
        }
        Command::TextCheck => {
            let map = load_report_map(&cli.options)?;
            report::file_ops::text_check::print_text_check(
                &cli.options.root,
                &map,
                cli.options.changed_only,
                cli.options.limit,
            );
        }
        Command::PatchPreview => {
            let map = load_report_map(&cli.options)?;
            report::file_ops::patch_preview::print_patch_preview(
                &cli.options.root,
                &map,
                cli.options.from.as_deref(),
                cli.options.limit,
            )?;
        }
        Command::PatchCheck => {
            report::file_ops::patch_apply::print_patch_check(
                &cli.options.root,
                cli.options.from.as_deref(),
                cli.options.limit,
            )?;
        }
        Command::PatchApply => {
            report::file_ops::patch_apply::print_patch_apply(
                &cli.options.root,
                cli.options.from.as_deref(),
                cli.options.write,
                cli.options.limit,
            )?;
        }
        Command::OpsPreview => {
            report::file_ops::ops_plan::print_ops_preview(
                &cli.options.root,
                cli.options.from.as_deref(),
                cli.options.yaml.as_deref(),
                ops_input_format(cli.options.raw),
                cli.options.limit,
            )?;
        }
        Command::OpsCheck => {
            let needs_map = report::file_ops::ops_plan::plan_requires_codemap(
                cli.options.from.as_deref(),
                cli.options.yaml.as_deref(),
                ops_input_format(cli.options.raw),
                cli.options.strict,
            )?;
            let map = if needs_map {
                Some(load_report_map(&cli.options)?)
            } else {
                None
            };
            report::file_ops::ops_plan::print_ops_check(
                &cli.options.root,
                map.as_ref(),
                cli.options.from.as_deref(),
                cli.options.yaml.as_deref(),
                ops_input_format(cli.options.raw),
                cli.options.strict,
                cli.options.limit,
            )?;
        }
        Command::OpsApply => {
            let needs_map = report::file_ops::ops_plan::plan_requires_codemap(
                cli.options.from.as_deref(),
                cli.options.yaml.as_deref(),
                ops_input_format(cli.options.raw),
                cli.options.strict,
            )?;
            let map = if needs_map {
                Some(load_report_map(&cli.options)?)
            } else {
                None
            };
            report::file_ops::ops_plan::print_ops_apply(
                &cli.options.root,
                map.as_ref(),
                cli.options.from.as_deref(),
                cli.options.yaml.as_deref(),
                ops_input_format(cli.options.raw),
                cli.options.write,
                cli.options.backup,
                cli.options.stop_on_error,
                cli.options.strict,
                cli.options.limit,
                !cli.options.no_verbose,
            )?;
        }
        Command::OpsRawPreview => {
            report::file_ops::raw_ops::print_raw_ops_preview(
                &cli.options.root,
                cli.options.from.as_deref(),
                cli.options.yaml.as_deref(),
                cli.options.limit,
            )?;
        }
        Command::OpsRawCheck => {
            let needs_map = report::file_ops::raw_ops::raw_plan_requires_codemap(
                cli.options.from.as_deref(),
                cli.options.yaml.as_deref(),
                cli.options.strict,
            )?;
            let map = if needs_map {
                Some(load_report_map(&cli.options)?)
            } else {
                None
            };
            report::file_ops::raw_ops::print_raw_ops_check(
                &cli.options.root,
                map.as_ref(),
                cli.options.from.as_deref(),
                cli.options.yaml.as_deref(),
                cli.options.strict,
                cli.options.limit,
            )?;
        }
        Command::OpsRawApply => {
            let needs_map = report::file_ops::raw_ops::raw_plan_requires_codemap(
                cli.options.from.as_deref(),
                cli.options.yaml.as_deref(),
                cli.options.strict,
            )?;
            let map = if needs_map {
                Some(load_report_map(&cli.options)?)
            } else {
                None
            };
            report::file_ops::raw_ops::print_raw_ops_apply(
                &cli.options.root,
                map.as_ref(),
                cli.options.from.as_deref(),
                cli.options.yaml.as_deref(),
                cli.options.write,
                cli.options.backup,
                cli.options.stop_on_error,
                cli.options.strict,
                cli.options.limit,
                !cli.options.no_verbose,
            )?;
        }
        Command::OpsSchema => {
            report::file_ops::ops_schema::print_ops_schema(
                cli.options.query.as_deref(),
                cli.options.json,
            )?;
        }
        Command::OpsSplit => {
            report::file_ops::ops_reports::print_ops_split(
                cli.options.from.as_deref(),
                cli.options.yaml.as_deref(),
                ops_input_format(cli.options.raw),
                cli.options.by.as_deref(),
            )?;
        }
        Command::OpsVerify => {
            report::file_ops::ops_reports::print_ops_verify(
                cli.options.from.as_deref(),
                cli.options.yaml.as_deref(),
                ops_input_format(cli.options.raw),
                cli.options.run,
            )?;
        }
        Command::OpsSummary => {
            report::file_ops::ops_reports::print_ops_summary(
                cli.options.from.as_deref(),
                cli.options.yaml.as_deref(),
                ops_input_format(cli.options.raw),
                cli.options.changed_only,
            )?;
        }
        Command::OpsSkeleton => {
            let map = load_report_map(&cli.options)?;
            let query = cli
                .options
                .query
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("ops-skeleton requires a query"))?;
            report::file_ops::ops_skeleton::print_ops_skeleton(
                &map,
                query,
                &cli.options.out,
                cli.options.write,
                cli.options.raw,
                cli.options.limit,
            )?;
        }
        Command::RangeForSymbol => {
            let map = load_report_map(&cli.options)?;
            let query = cli
                .options
                .query
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("range-for-symbol requires a query"))?;
            report::file_ops::range_for_symbol::print_range_for_symbol(
                &map,
                query,
                cli.options.limit,
            )?;
        }
        Command::RangeForLines => {
            let map = load_report_map(&cli.options)?;
            let query = cli
                .options
                .query
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("range-for-lines requires a file path"))?;
            let start_line = cli
                .options
                .start_line
                .ok_or_else(|| anyhow::anyhow!("range-for-lines requires start line"))?;
            let end_line = cli
                .options
                .end_line
                .ok_or_else(|| anyhow::anyhow!("range-for-lines requires end line"))?;
            report::file_ops::range_for_lines::print_range_for_lines(
                &cli.options.root,
                &map,
                std::path::Path::new(query),
                start_line,
                end_line,
                &cli.options.yaml_op,
                cli.options.context_radius,
            )?;
        }
        Command::AnchorRange => {
            let map = load_report_map(&cli.options)?;
            let query = cli
                .options
                .query
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("anchor-range requires a query"))?;
            let to = cli
                .options
                .to
                .as_ref()
                .map(|path| path.to_string_lossy().to_string());
            report::file_ops::anchor_range::print_anchor_range(
                &cli.options.root,
                &map,
                query,
                to.as_deref(),
                cli.options.limit,
            )?;
        }
        Command::TodoIndex => {
            let map = load_report_map(&cli.options)?;
            report::todo_index::print_todo_index(&map, cli.options.limit);
        }
        Command::RiskIndex => {
            let map = load_report_map(&cli.options)?;
            report::risk_index::print_risk_index(&map, cli.options.limit);
        }
        Command::Smells => {
            let map = load_report_map(&cli.options)?;
            report::code_smells::print_code_smells(
                &cli.options.root,
                &map,
                report::code_smells::SmellOptions {
                    top: cli.options.top.max(1),
                    changed_only: cli.options.changed_only,
                    group: cli.options.group.clone(),
                    min_score: cli.options.min_score,
                    report: cli.options.report,
                    report_file: cli.options.report_file.clone(),
                    file_lines: cli.options.file_lines,
                    json: cli.options.json,
                    why: cli.options.why,
                    include_tests: cli.options.include_tests,
                    include_generated: cli.options.include_generated,
                    file: cli.options.file.clone(),
                },
            )?;
        }
        Command::CommitFiles => {
            let map = load_report_map(&cli.options)?;
            report::file_ops::commit_files::print_commit_files(
                &cli.options.root,
                &map,
                cli.options.changed_only,
                cli.options.limit,
            )?;
        }
        _ => unreachable!("non-file-op command routed to file-ops dispatcher"),
    }

    Ok(())
}
