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
        related: &["open-set", "rename-plan", "slice"],
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
    fn catalog_contains_command_map() {
        assert!(COMMANDS.iter().any(|command| command.name == "command-map"));
    }
}
