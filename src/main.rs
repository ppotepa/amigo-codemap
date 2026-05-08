mod cache;
mod cli;
pub use amigo_symbol_explorer::git;
pub use amigo_symbol_explorer::model;
mod output;
pub use amigo_symbol_explorer::query;
mod report;
mod scan;
mod snapshot_store;
mod taxonomy;
#[cfg(test)]
mod test_support;
mod watch;

use anyhow::Result;
use cli::{Cli, Command};

fn load_report_map(options: &cli::Options) -> Result<model::CodeMap> {
    let loaded = snapshot_store::load_or_scan(options)?;
    let _source = loaded.source;
    Ok(loaded.map)
}

fn main() -> Result<()> {
    let mut cli = Cli::parse(std::env::args().skip(1))?;

    match cli.command {
        Command::Brief
        | Command::Changed
        | Command::Changes
        | Command::Find
        | Command::Docs
        | Command::CommandMap
        | Command::Files => {
            cli.options.level = 0;
            cli.options.ai = false;
        }
        Command::VerifyPlan
        | Command::Taxonomy
        | Command::Stale
        | Command::Fallout
        | Command::MovePlan
        | Command::Dup
        | Command::TauriCommands
        | Command::RegistryCheck
        | Command::MetadataAudit
        | Command::DescriptorSkeleton
        | Command::OperationsSummary
        | Command::CommitPlan
        | Command::CommitSummary
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
        | Command::PatchCheck
        | Command::PatchApply
        | Command::OpsPreview
        | Command::RiskIndex
        | Command::CommitFiles => {
            cli.options.level = 0;
            cli.options.ai = false;
        }
        Command::Scope
        | Command::Refs
        | Command::Anchors
        | Command::AnchorCheck
        | Command::Impact
        | Command::ServiceShape
            if cli.options.level < 2 =>
        {
            cli.options.level = 2;
        }
        Command::Where
        | Command::Symbols
        | Command::Signature
        | Command::Trace
        | Command::ChangePlan
        | Command::ExplainFile
        | Command::Neighbors
        | Command::ApiSurface
        | Command::ComponentGraph
        | Command::TauriGraph
        | Command::CallsiteCandidates
        | Command::TodoIndex
        | Command::OpsApply
        | Command::OpsSkeleton
        | Command::OpsSchema
        | Command::OpsSplit
        | Command::OpsVerify
        | Command::OpsSummary
        | Command::RangeForSymbol
        | Command::RangeForLines
        | Command::AnchorRange
            if cli.options.level < 2 =>
        {
            cli.options.level = 2;
        }
        Command::OpsCheck if cli.options.strict && cli.options.level < 2 => {
            cli.options.level = 2;
        }
        Command::OpsCheck => {
            cli.options.level = 0;
            cli.options.ai = false;
        }
        Command::Slice if cli.options.symbol.is_some() && cli.options.level < 2 => {
            cli.options.level = 2;
        }
        Command::Slice => {
            cli.options.level = 0;
            cli.options.ai = false;
        }
        Command::OpenSet
        | Command::LargeFiles
        | Command::PatchPreview
        | Command::AppendPlan
        | Command::CopyPlan
            if cli.options.level < 2 =>
        {
            cli.options.level = 2;
        }
        Command::Workset
            if (cli.options.from_impact.is_some() || cli.options.status)
                && cli.options.level < 2 =>
        {
            cli.options.level = 2;
        }
        Command::OrphanFiles if cli.options.level < 3 => {
            cli.options.level = 3;
        }
        Command::Refresh => {
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
        Command::Refresh => {
            let wrote = cache::refresh_changed_only(&cli.options)?;
            if wrote {
                println!("refreshed {}", cli.options.out.display());
            } else {
                println!("unchanged {}", cli.options.out.display());
            }
        }
        Command::Watch => watch::watch_project(cli.options)?,
        Command::Status => {
            snapshot_store::print_status(&cli.options)?;
        }
        Command::Changes => {
            report::live_changes::print_changes(
                &cli.options.root,
                cli.options.group.as_deref(),
                cli.options.limit,
                cli.options.compact,
                cli.options.hide_generated,
                cli.options.warnings,
            )?;
        }
        Command::Changed => {
            let map = load_report_map(&cli.options)?;
            report::print_changed(&map, cli.options.group.as_deref(), cli.options.limit);
        }
        Command::Files => {
            let map = load_report_map(&cli.options)?;
            report::print_files(
                &map,
                cli.options.query.as_deref(),
                cli.options.group.as_deref(),
                cli.options.changed_only,
                cli.options.limit,
            );
        }
        Command::Symbols => {
            let map = load_report_map(&cli.options)?;
            report::symbols::print_symbols(
                &map,
                cli.options.query.as_deref(),
                cli.options.file.as_deref(),
                cli.options.changed_only,
                cli.options.metadata,
                cli.options.limit,
            )?;
        }
        Command::Where => {
            let map = load_report_map(&cli.options)?;
            let query = cli
                .options
                .query
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("where requires a query"))?;
            report::where_symbol::print_where(&cli.options.root, &map, query, cli.options.limit)?;
        }
        Command::Signature => {
            let map = load_report_map(&cli.options)?;
            let query = cli
                .options
                .query
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("signature requires a query"))?;
            report::signature::print_signature(&map, query, cli.options.limit)?;
        }
        Command::Trace => {
            let map = load_report_map(&cli.options)?;
            let query = cli
                .options
                .query
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("trace requires a query"))?;
            report::trace::print_trace(&map, query, cli.options.limit)?;
        }
        Command::ChangePlan => {
            let map = load_report_map(&cli.options)?;
            let query = cli
                .options
                .query
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("change-plan requires a query"))?;
            report::change_plan::print_change_plan(&map, query, cli.options.limit)?;
        }
        Command::ExplainFile => {
            let map = load_report_map(&cli.options)?;
            let query = cli
                .options
                .query
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("explain-file requires a path"))?;
            report::explain_file::print_explain_file(&map, query)?;
        }
        Command::Neighbors => {
            let map = load_report_map(&cli.options)?;
            let query = cli
                .options
                .query
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("neighbors requires a path"))?;
            report::neighbors::print_neighbors(&map, query, cli.options.limit)?;
        }
        Command::ApiSurface => {
            let map = load_report_map(&cli.options)?;
            report::api_surface::print_api_surface(&map, cli.options.limit);
        }
        Command::ComponentGraph => {
            let map = load_report_map(&cli.options)?;
            report::component_graph::print_component_graph(&map, cli.options.limit);
        }
        Command::TauriGraph => {
            let map = load_report_map(&cli.options)?;
            report::tauri_graph::print_tauri_graph(&map, cli.options.limit);
        }
        Command::CallsiteCandidates => {
            let map = load_report_map(&cli.options)?;
            let query = cli
                .options
                .query
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("callsite-candidates requires a query"))?;
            report::callsite_candidates::print_callsite_candidates(&map, query, cli.options.limit)?;
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
            let map = load_report_map(&cli.options)?;
            report::print_brief(&map);
        }
        Command::Find => {
            let map = load_report_map(&cli.options)?;
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
            let map = load_report_map(&cli.options)?;
            let query = cli
                .options
                .query
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("scope requires a query"))?;
            report::print_scope(&map, query, cli.options.limit);
        }
        Command::Refs => {
            let map = load_report_map(&cli.options)?;
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
            let map = load_report_map(&cli.options)?;
            report::print_docs(&cli.options.root, &map);
        }
        Command::CommandMap => {
            let query = cli
                .options
                .query
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("command-map requires a query"))?;
            report::command_map::print_command_map(query)?;
        }
        Command::Anchors => {
            let map = if cli.options.write {
                scan::scan_project(&cli.options)?
            } else {
                load_report_map(&cli.options)?
            };
            report::anchors::print_anchors(
                &cli.options.root,
                &map,
                cli.options.query.as_deref(),
                cli.options.write,
                cli.options.limit,
            )?;
        }
        Command::AnchorCheck => {
            let map = load_report_map(&cli.options)?;
            report::anchor_check::print_anchor_check(&cli.options.root, &map)?;
        }
        Command::Taxonomy => {
            report::taxonomy_report::print_taxonomy(&cli.options.root)?;
        }
        Command::Verify => {
            report::run_verify(
                &cli.options.root,
                &cli.options.verify_args,
                cli.options.limit,
            )?;
        }
        Command::VerifyPlan => {
            let map = load_report_map(&cli.options)?;
            report::verify_plan::print_verify_plan(&map, cli.options.changed_only);
        }
        Command::Stale => {
            let map = load_report_map(&cli.options)?;
            report::stale::print_stale(
                &cli.options.root,
                &map,
                &cli.options.patterns,
                cli.options.changed_only,
                cli.options.limit,
            )?;
        }
        Command::Impact => {
            let map = load_report_map(&cli.options)?;
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
            let map = load_report_map(&cli.options)?;
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
            let map = load_report_map(&cli.options)?;
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
        Command::MetadataAudit => {
            let report =
                amigo_symbol_explorer::metadata::component_audit::audit_component_metadata(
                    &cli.options.root,
                )?;
            report.print_text();
        }
        Command::DescriptorSkeleton => {
            let component = cli.options.query.as_deref().unwrap_or("ExampleComponent");
            amigo_symbol_explorer::metadata::descriptor_skeleton::print_descriptor_skeleton(
                component,
            )?;
        }
        Command::OperationsSummary => {
            report::summary::print_operations_summary(&cli.options.root, cli.options.limit)?;
        }
        Command::CommitPlan => {
            report::live_changes::print_commit_plan(
                &cli.options.root,
                cli.options.limit,
                cli.options.compact,
            )?;
        }
        Command::CommitSummary => {
            let map = load_report_map(&cli.options)?;
            report::summary::print_commit_summary(&map, cli.options.limit);
        }
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
                cli.options.limit,
            )?;
        }
        Command::OpsCheck => {
            let map = if cli.options.strict {
                Some(load_report_map(&cli.options)?)
            } else {
                None
            };
            report::file_ops::ops_plan::print_ops_check(
                &cli.options.root,
                map.as_ref(),
                cli.options.from.as_deref(),
                cli.options.yaml.as_deref(),
                cli.options.strict,
                cli.options.limit,
            )?;
        }
        Command::OpsApply => {
            let map = if cli.options.write {
                scan::scan_project(&cli.options)?
            } else {
                load_report_map(&cli.options)?
            };
            report::file_ops::ops_plan::print_ops_apply(
                &cli.options.root,
                &map,
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
                cli.options.by.as_deref(),
            )?;
        }
        Command::OpsVerify => {
            report::file_ops::ops_reports::print_ops_verify(
                cli.options.from.as_deref(),
                cli.options.yaml.as_deref(),
                cli.options.run,
            )?;
        }
        Command::OpsSummary => {
            report::file_ops::ops_reports::print_ops_summary(
                cli.options.from.as_deref(),
                cli.options.yaml.as_deref(),
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
        Command::CommitFiles => {
            let map = load_report_map(&cli.options)?;
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
