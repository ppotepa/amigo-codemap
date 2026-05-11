use anyhow::Result;

use crate::cli::{Cli, Command};
use crate::load_report_map;
use crate::{cache, output, report, scan, snapshot_store, watch};

pub(super) fn run(cli: Cli) -> Result<()> {
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
        Command::TraceField => {
            let map = load_report_map(&cli.options)?;
            let query = cli
                .options
                .query
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("trace-field requires a query"))?;
            report::trace_field::print_trace_field(&cli.options.root, &map, query, cli.options.limit)?;
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
            let mut plan = report::verify_plan::plan_for_map(&map, cli.options.changed_only);
            report::verify_plan::apply_expectations(
                &mut plan,
                &cli.options.expect_present,
                &cli.options.expect_absent,
            );
            print!("{}", report::verify_plan::render_verify_plan(&plan));
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
        Command::Explain => unreachable!("handled by parent dispatcher"),
        _ => unreachable!("non-report command routed to reports dispatcher"),
    }

    Ok(())
}
