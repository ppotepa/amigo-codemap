use anyhow::{Result, bail};

use crate::report::file_ops::model::{FileOpReport, NextAction, Risk, RiskLevel, print_report};

#[derive(Debug, Clone, Copy)]
struct CommandDescriptor {
    name: &'static str,
    category: &'static str,
    cli_paths: &'static [&'static str],
    dispatch_paths: &'static [&'static str],
    implementation_paths: &'static [&'static str],
    docs_paths: &'static [&'static str],
    test_paths: &'static [&'static str],
    related: &'static [&'static str],
}

const DOC_PATHS: &[&str] = &["crates/tools/amigo-codemap/README.md", "AMIGO_WORKFLOW.md"];

const MAIN_PATH: &[&str] = &["crates/tools/amigo-codemap/src/main.rs"];
const CLI_PATH: &[&str] = &["crates/tools/amigo-codemap/src/cli.rs"];

const COMMANDS: &[CommandDescriptor] = &[
    CommandDescriptor {
        name: "command-map",
        category: "meta",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &["crates/tools/amigo-codemap/src/report/command_map.rs"],
        docs_paths: DOC_PATHS,
        test_paths: &[
            "crates/tools/amigo-codemap/src/cli.rs",
            "crates/tools/amigo-codemap/src/report/command_map.rs",
        ],
        related: &["append-plan", "operations-summary", "docs"],
    },
    CommandDescriptor {
        name: "refresh",
        category: "snapshot",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &[
            "crates/tools/amigo-codemap/src/cache.rs",
            "crates/tools/amigo-codemap/src/snapshot_store.rs",
        ],
        docs_paths: DOC_PATHS,
        test_paths: &[
            "crates/tools/amigo-codemap/src/cli.rs",
            "crates/tools/amigo-codemap/src/snapshot_store.rs",
        ],
        related: &["scan", "watch", "status"],
    },
    CommandDescriptor {
        name: "status",
        category: "snapshot",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &["crates/tools/amigo-codemap/src/snapshot_store.rs"],
        docs_paths: DOC_PATHS,
        test_paths: &[
            "crates/tools/amigo-codemap/src/cli.rs",
            "crates/tools/amigo-codemap/src/snapshot_store.rs",
        ],
        related: &["refresh", "watch", "scan"],
    },
    CommandDescriptor {
        name: "changes",
        category: "git",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &["crates/tools/amigo-codemap/src/report/live_changes.rs"],
        docs_paths: DOC_PATHS,
        test_paths: &[
            "crates/tools/amigo-codemap/src/cli.rs",
            "crates/tools/amigo-codemap/src/report/live_changes.rs",
        ],
        related: &["changed", "commit-plan", "commit-summary", "status"],
    },
    CommandDescriptor {
        name: "commit-plan",
        category: "git",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &["crates/tools/amigo-codemap/src/report/live_changes.rs"],
        docs_paths: DOC_PATHS,
        test_paths: &[
            "crates/tools/amigo-codemap/src/cli.rs",
            "crates/tools/amigo-codemap/src/report/live_changes.rs",
        ],
        related: &["changes", "commit-files", "commit-summary", "verify-plan"],
    },
    CommandDescriptor {
        name: "taxonomy",
        category: "meta",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &[
            "crates/tools/amigo-codemap/src/taxonomy.rs",
            "crates/tools/amigo-codemap/src/report/taxonomy_report.rs",
        ],
        docs_paths: DOC_PATHS,
        test_paths: &["crates/tools/amigo-codemap/src/cli.rs"],
        related: &["anchors", "anchor-check", "command-map"],
    },
    CommandDescriptor {
        name: "anchors",
        category: "meta",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &[
            "crates/tools/amigo-codemap/src/report/anchors.rs",
            "crates/tools/amigo-codemap/src/scan/codemap_tags.rs",
        ],
        docs_paths: DOC_PATHS,
        test_paths: &["crates/tools/amigo-codemap/src/cli.rs"],
        related: &["taxonomy", "anchor-check", "trace", "open-set"],
    },
    CommandDescriptor {
        name: "anchor-check",
        category: "meta",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &[
            "crates/tools/amigo-codemap/src/report/anchor_check.rs",
            "crates/tools/amigo-codemap/src/taxonomy.rs",
        ],
        docs_paths: DOC_PATHS,
        test_paths: &["crates/tools/amigo-codemap/src/cli.rs"],
        related: &["anchors", "taxonomy", "trace"],
    },
    CommandDescriptor {
        name: "symbols",
        category: "navigation",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &["crates/tools/amigo-codemap/src/report/symbols.rs"],
        docs_paths: DOC_PATHS,
        test_paths: &[
            "crates/tools/amigo-codemap/src/cli.rs",
            "crates/tools/amigo-codemap/src/report/symbols.rs",
        ],
        related: &["where", "signature", "trace"],
    },
    CommandDescriptor {
        name: "where",
        category: "navigation",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &["crates/tools/amigo-codemap/src/report/where_symbol.rs"],
        docs_paths: DOC_PATHS,
        test_paths: &[
            "crates/tools/amigo-codemap/src/cli.rs",
            "crates/tools/amigo-codemap/src/report/where_symbol.rs",
        ],
        related: &["symbols", "signature", "trace", "open-set"],
    },
    CommandDescriptor {
        name: "signature",
        category: "navigation",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &["crates/tools/amigo-codemap/src/report/signature.rs"],
        docs_paths: DOC_PATHS,
        test_paths: &[
            "crates/tools/amigo-codemap/src/cli.rs",
            "crates/tools/amigo-codemap/src/report/signature.rs",
        ],
        related: &["symbols", "where"],
    },
    CommandDescriptor {
        name: "trace",
        category: "navigation",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &["crates/tools/amigo-codemap/src/report/trace.rs"],
        docs_paths: DOC_PATHS,
        test_paths: &[
            "crates/tools/amigo-codemap/src/cli.rs",
            "crates/tools/amigo-codemap/src/report/trace.rs",
        ],
        related: &["where", "signature", "open-set", "impact"],
    },
    CommandDescriptor {
        name: "append-plan",
        category: "file-ops",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &[
            "crates/tools/amigo-codemap/src/report/file_ops/append_plan.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/mod.rs",
        ],
        docs_paths: DOC_PATHS,
        test_paths: &[
            "crates/tools/amigo-codemap/src/cli.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/append_plan.rs",
        ],
        related: &["copy-plan", "open-set", "rename-plan", "slice"],
    },
    CommandDescriptor {
        name: "copy-plan",
        category: "file-ops",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &[
            "crates/tools/amigo-codemap/src/report/file_ops/copy_plan.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/mod.rs",
        ],
        docs_paths: DOC_PATHS,
        test_paths: &[
            "crates/tools/amigo-codemap/src/cli.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/copy_plan.rs",
        ],
        related: &["append-plan", "open-set", "slice"],
    },
    CommandDescriptor {
        name: "patch-check",
        category: "file-ops",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &[
            "crates/tools/amigo-codemap/src/report/file_ops/patch_apply.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/mod.rs",
        ],
        docs_paths: DOC_PATHS,
        test_paths: &[
            "crates/tools/amigo-codemap/src/cli.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/patch_apply.rs",
        ],
        related: &["patch-apply", "patch-preview", "append-plan"],
    },
    CommandDescriptor {
        name: "patch-apply",
        category: "file-ops",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &[
            "crates/tools/amigo-codemap/src/report/file_ops/patch_apply.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/mod.rs",
        ],
        docs_paths: DOC_PATHS,
        test_paths: &[
            "crates/tools/amigo-codemap/src/cli.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/patch_apply.rs",
        ],
        related: &["patch-check", "patch-preview", "append-plan"],
    },
    CommandDescriptor {
        name: "open-set",
        category: "file-ops",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &[
            "crates/tools/amigo-codemap/src/report/file_ops/open_set.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/mod.rs",
        ],
        docs_paths: DOC_PATHS,
        test_paths: &[
            "crates/tools/amigo-codemap/src/cli.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/open_set.rs",
        ],
        related: &["append-plan", "slice", "impact"],
    },
    CommandDescriptor {
        name: "slice",
        category: "file-ops",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &[
            "crates/tools/amigo-codemap/src/report/file_ops/slice.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/mod.rs",
        ],
        docs_paths: DOC_PATHS,
        test_paths: &[
            "crates/tools/amigo-codemap/src/cli.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/slice.rs",
        ],
        related: &["open-set", "append-plan", "impact"],
    },
    CommandDescriptor {
        name: "delete-plan",
        category: "file-ops",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &[
            "crates/tools/amigo-codemap/src/report/file_ops/delete_plan.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/mod.rs",
        ],
        docs_paths: DOC_PATHS,
        test_paths: &[
            "crates/tools/amigo-codemap/src/cli.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/delete_plan.rs",
        ],
        related: &["file-move-plan", "orphan-files", "shim-check"],
    },
    CommandDescriptor {
        name: "file-move-plan",
        category: "file-ops",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &[
            "crates/tools/amigo-codemap/src/report/file_ops/file_move_plan.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/mod.rs",
        ],
        docs_paths: DOC_PATHS,
        test_paths: &[
            "crates/tools/amigo-codemap/src/cli.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/file_move_plan.rs",
        ],
        related: &["import-fix-plan", "delete-plan", "rename-plan"],
    },
    CommandDescriptor {
        name: "rename-plan",
        category: "file-ops",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &[
            "crates/tools/amigo-codemap/src/report/file_ops/rename_plan.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/mod.rs",
        ],
        docs_paths: DOC_PATHS,
        test_paths: &[
            "crates/tools/amigo-codemap/src/cli.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/rename_plan.rs",
        ],
        related: &["append-plan", "import-fix-plan", "file-move-plan"],
    },
    CommandDescriptor {
        name: "import-fix-plan",
        category: "file-ops",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &[
            "crates/tools/amigo-codemap/src/report/file_ops/import_fix_plan.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/mod.rs",
        ],
        docs_paths: DOC_PATHS,
        test_paths: &[
            "crates/tools/amigo-codemap/src/cli.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/import_fix_plan.rs",
        ],
        related: &["file-move-plan", "delete-plan", "fallout"],
    },
    CommandDescriptor {
        name: "ops-preview",
        category: "file-ops",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &[
            "crates/tools/amigo-codemap/src/report/file_ops/ops_plan.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/mod.rs",
        ],
        docs_paths: DOC_PATHS,
        test_paths: &[
            "crates/tools/amigo-codemap/src/cli.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/ops_plan.rs",
        ],
        related: &["ops-check", "ops-apply", "patch-check"],
    },
    CommandDescriptor {
        name: "ops-check",
        category: "file-ops",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &[
            "crates/tools/amigo-codemap/src/report/file_ops/ops_plan.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/mod.rs",
        ],
        docs_paths: DOC_PATHS,
        test_paths: &[
            "crates/tools/amigo-codemap/src/cli.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/ops_plan.rs",
        ],
        related: &["ops-preview", "ops-apply", "patch-check"],
    },
    CommandDescriptor {
        name: "ops-apply",
        category: "file-ops",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &[
            "crates/tools/amigo-codemap/src/report/file_ops/ops_plan.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/mod.rs",
        ],
        docs_paths: DOC_PATHS,
        test_paths: &[
            "crates/tools/amigo-codemap/src/cli.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/ops_plan.rs",
        ],
        related: &["ops-check", "ops-skeleton", "patch-apply"],
    },
    CommandDescriptor {
        name: "ops-skeleton",
        category: "file-ops",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &[
            "crates/tools/amigo-codemap/src/report/file_ops/ops_skeleton.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/mod.rs",
        ],
        docs_paths: DOC_PATHS,
        test_paths: &[
            "crates/tools/amigo-codemap/src/cli.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/ops_skeleton.rs",
        ],
        related: &[
            "ops-schema",
            "ops-preview",
            "ops-check",
            "ops-apply",
            "range-for-symbol",
        ],
    },
    CommandDescriptor {
        name: "ops-schema",
        category: "file-ops",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &[
            "crates/tools/amigo-codemap/src/report/file_ops/ops_schema.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/mod.rs",
        ],
        docs_paths: DOC_PATHS,
        test_paths: &[
            "crates/tools/amigo-codemap/src/cli.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/ops_schema.rs",
        ],
        related: &["ops-skeleton", "ops-check", "ops-preview"],
    },
    CommandDescriptor {
        name: "ops-split",
        category: "file-ops",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &[
            "crates/tools/amigo-codemap/src/report/file_ops/ops_reports.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/mod.rs",
        ],
        docs_paths: DOC_PATHS,
        test_paths: &["crates/tools/amigo-codemap/src/cli.rs"],
        related: &["ops-preview", "ops-summary", "ops-apply"],
    },
    CommandDescriptor {
        name: "ops-verify",
        category: "file-ops",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &[
            "crates/tools/amigo-codemap/src/report/file_ops/ops_reports.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/mod.rs",
        ],
        docs_paths: DOC_PATHS,
        test_paths: &["crates/tools/amigo-codemap/src/cli.rs"],
        related: &["ops-check", "ops-apply", "verify-plan"],
    },
    CommandDescriptor {
        name: "ops-summary",
        category: "file-ops",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &[
            "crates/tools/amigo-codemap/src/report/file_ops/ops_reports.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/mod.rs",
        ],
        docs_paths: DOC_PATHS,
        test_paths: &["crates/tools/amigo-codemap/src/cli.rs"],
        related: &["ops-preview", "ops-verify", "operations-summary"],
    },
    CommandDescriptor {
        name: "range-for-symbol",
        category: "file-ops",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &[
            "crates/tools/amigo-codemap/src/report/file_ops/range_for_symbol.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/mod.rs",
        ],
        docs_paths: DOC_PATHS,
        test_paths: &[
            "crates/tools/amigo-codemap/src/cli.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/range_for_symbol.rs",
        ],
        related: &["signature", "slice", "ops-skeleton", "ops-check"],
    },
    CommandDescriptor {
        name: "anchor-range",
        category: "file-ops",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &[
            "crates/tools/amigo-codemap/src/report/file_ops/anchor_range.rs",
            "crates/tools/amigo-codemap/src/report/file_ops/mod.rs",
        ],
        docs_paths: DOC_PATHS,
        test_paths: &["crates/tools/amigo-codemap/src/cli.rs"],
        related: &["anchors", "anchor-check", "ops-check"],
    },
    CommandDescriptor {
        name: "change-plan",
        category: "navigation",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &["crates/tools/amigo-codemap/src/report/change_plan.rs"],
        docs_paths: DOC_PATHS,
        test_paths: &["crates/tools/amigo-codemap/src/cli.rs"],
        related: &["trace", "open-set", "impact", "verify-plan"],
    },
    CommandDescriptor {
        name: "explain-file",
        category: "navigation",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &["crates/tools/amigo-codemap/src/report/explain_file.rs"],
        docs_paths: DOC_PATHS,
        test_paths: &["crates/tools/amigo-codemap/src/cli.rs"],
        related: &["neighbors", "slice"],
    },
    CommandDescriptor {
        name: "neighbors",
        category: "navigation",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &["crates/tools/amigo-codemap/src/report/neighbors.rs"],
        docs_paths: DOC_PATHS,
        test_paths: &["crates/tools/amigo-codemap/src/cli.rs"],
        related: &["explain-file", "impact", "open-set"],
    },
    CommandDescriptor {
        name: "api-surface",
        category: "navigation",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &["crates/tools/amigo-codemap/src/report/api_surface.rs"],
        docs_paths: DOC_PATHS,
        test_paths: &["crates/tools/amigo-codemap/src/cli.rs"],
        related: &["signature", "where"],
    },
    CommandDescriptor {
        name: "component-graph",
        category: "navigation",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &["crates/tools/amigo-codemap/src/report/component_graph.rs"],
        docs_paths: DOC_PATHS,
        test_paths: &["crates/tools/amigo-codemap/src/cli.rs"],
        related: &["trace", "open-set", "neighbors"],
    },
    CommandDescriptor {
        name: "tauri-graph",
        category: "navigation",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &["crates/tools/amigo-codemap/src/report/tauri_graph.rs"],
        docs_paths: DOC_PATHS,
        test_paths: &["crates/tools/amigo-codemap/src/cli.rs"],
        related: &["trace", "impact", "api-surface"],
    },
    CommandDescriptor {
        name: "callsite-candidates",
        category: "navigation",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &["crates/tools/amigo-codemap/src/report/callsite_candidates.rs"],
        docs_paths: DOC_PATHS,
        test_paths: &["crates/tools/amigo-codemap/src/cli.rs"],
        related: &["impact", "where", "trace"],
    },
    CommandDescriptor {
        name: "todo-index",
        category: "quality",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &["crates/tools/amigo-codemap/src/report/todo_index.rs"],
        docs_paths: DOC_PATHS,
        test_paths: &["crates/tools/amigo-codemap/src/cli.rs"],
        related: &["risk-index", "stale"],
    },
    CommandDescriptor {
        name: "risk-index",
        category: "quality",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &["crates/tools/amigo-codemap/src/report/risk_index.rs"],
        docs_paths: DOC_PATHS,
        test_paths: &["crates/tools/amigo-codemap/src/cli.rs"],
        related: &["todo-index", "large-files", "commit-files"],
    },
    CommandDescriptor {
        name: "operations-summary",
        category: "summary",
        cli_paths: CLI_PATH,
        dispatch_paths: MAIN_PATH,
        implementation_paths: &["crates/tools/amigo-codemap/src/report/summary.rs"],
        docs_paths: DOC_PATHS,
        test_paths: &[
            "crates/tools/amigo-codemap/src/cli.rs",
            "crates/tools/amigo-codemap/src/report/summary.rs",
        ],
        related: &["commit-summary", "command-map", "docs"],
    },
];

pub fn print_command_map(query: &str) -> Result<()> {
    if query.trim().is_empty() {
        bail!("command-map requires a command query");
    }

    let query = query.trim();
    let exact = COMMANDS.iter().find(|command| command.name == query);
    let mut matches = COMMANDS
        .iter()
        .filter(|command| command.name.contains(query))
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| left.name.cmp(right.name));

    let Some(command) = exact.or_else(|| matches.first().copied()) else {
        print_report(&FileOpReport {
            task: format!("command-map {query}"),
            scope: vec![format!("query: {query}")],
            findings: vec!["matches: none".to_string()],
            risks: vec![Risk {
                level: RiskLevel::Medium,
                message: "command is not in the static command catalog yet".to_string(),
            }],
            verify: vec!["cargo test -p amigo-codemap".to_string()],
            next: vec![
                NextAction {
                    label: "inspect cli.rs command enum manually".to_string(),
                },
                NextAction {
                    label: "add the missing command descriptor".to_string(),
                },
            ],
        });
        return Ok(());
    };

    let mut findings = Vec::new();
    findings.push(format!("command: {}", command.name));
    findings.push(format!("category: {}", command.category));
    findings.push("cli:".to_string());
    findings.extend(command.cli_paths.iter().map(|path| format!("  {path}")));
    findings.push("dispatch:".to_string());
    findings.extend(
        command
            .dispatch_paths
            .iter()
            .map(|path| format!("  {path}")),
    );
    findings.push("implementation:".to_string());
    findings.extend(
        command
            .implementation_paths
            .iter()
            .map(|path| format!("  {path}")),
    );
    findings.push("docs:".to_string());
    findings.extend(command.docs_paths.iter().map(|path| format!("  {path}")));
    findings.push("tests:".to_string());
    findings.extend(command.test_paths.iter().map(|path| format!("  {path}")));
    findings.push(format!("related: {}", command.related.join(", ")));

    if matches.len() > 1 {
        findings.push("similar commands:".to_string());
        for similar in matches.iter().take(6) {
            findings.push(format!("  {}", similar.name));
        }
    }

    print_report(&FileOpReport {
        task: format!("command-map {query}"),
        scope: vec![
            format!("query: {query}"),
            "catalog: static command descriptors".to_string(),
        ],
        findings,
        risks: vec![Risk {
            level: RiskLevel::Low,
            message: "catalog can drift if cli wiring changes without updating descriptors"
                .to_string(),
        }],
        verify: vec![
            "cargo test -p amigo-codemap".to_string(),
            "cargo build -p amigo-codemap".to_string(),
        ],
        next: vec![
            NextAction {
                label: "read cli, dispatch, then implementation in that order".to_string(),
            },
            NextAction {
                label: "update docs after code changes".to_string(),
            },
            NextAction {
                label: "refresh command-map descriptors if files moved".to_string(),
            },
        ],
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::COMMANDS;

    #[test]
    fn catalog_contains_append_plan() {
        assert!(COMMANDS.iter().any(|command| command.name == "append-plan"));
    }

    #[test]
    fn catalog_contains_copy_plan() {
        assert!(COMMANDS.iter().any(|command| command.name == "copy-plan"));
    }

    #[test]
    fn catalog_contains_command_map() {
        assert!(COMMANDS.iter().any(|command| command.name == "command-map"));
    }

    #[test]
    fn catalog_contains_anchor_commands() {
        assert!(COMMANDS.iter().any(|command| command.name == "anchors"));
        assert!(
            COMMANDS
                .iter()
                .any(|command| command.name == "anchor-check")
        );
        assert!(COMMANDS.iter().any(|command| command.name == "taxonomy"));
    }

    #[test]
    fn catalog_contains_snapshot_commands() {
        assert!(COMMANDS.iter().any(|command| command.name == "refresh"));
        assert!(COMMANDS.iter().any(|command| command.name == "status"));
    }

    #[test]
    fn catalog_contains_live_git_commands() {
        assert!(COMMANDS.iter().any(|command| command.name == "changes"));
        assert!(COMMANDS.iter().any(|command| command.name == "commit-plan"));
    }

    #[test]
    fn catalog_contains_patch_apply() {
        assert!(COMMANDS.iter().any(|command| command.name == "patch-apply"));
    }

    #[test]
    fn catalog_contains_trace() {
        assert!(COMMANDS.iter().any(|command| command.name == "trace"));
    }

    #[test]
    fn catalog_contains_ops_apply() {
        assert!(COMMANDS.iter().any(|command| command.name == "ops-apply"));
    }

    #[test]
    fn catalog_contains_ops_skeleton() {
        assert!(
            COMMANDS
                .iter()
                .any(|command| command.name == "ops-skeleton")
        );
    }

    #[test]
    fn catalog_contains_range_for_symbol() {
        assert!(
            COMMANDS
                .iter()
                .any(|command| command.name == "range-for-symbol")
        );
    }

    #[test]
    fn catalog_contains_yaml_ops_helpers() {
        for name in [
            "ops-schema",
            "ops-split",
            "ops-verify",
            "ops-summary",
            "anchor-range",
        ] {
            assert!(COMMANDS.iter().any(|command| command.name == name));
        }
    }

    #[test]
    fn catalog_contains_change_plan() {
        assert!(COMMANDS.iter().any(|command| command.name == "change-plan"));
    }

    #[test]
    fn catalog_contains_risk_index() {
        assert!(COMMANDS.iter().any(|command| command.name == "risk-index"));
    }
}
