use std::collections::HashSet;

use super::command_names::parse_command_name;
use super::command_spec::COMMAND_SPECS;
use super::{Cli, Command};

#[test]
fn parses_find_query_and_limit() {
    let cli = Cli::parse([
        "find".to_string(),
        "AssetTreePanel".to_string(),
        "--limit".to_string(),
        "12".to_string(),
    ])
    .expect("cli should parse");

    assert_eq!(cli.command, Command::Find);
    assert_eq!(cli.options.query.as_deref(), Some("AssetTreePanel"));
    assert_eq!(cli.options.limit, 12);
}

#[test]
fn parses_changed_group() {
    let cli = Cli::parse([
        "changed".to_string(),
        "--group".to_string(),
        "package".to_string(),
    ])
    .expect("cli should parse");

    assert_eq!(cli.command, Command::Changed);
    assert_eq!(cli.options.group.as_deref(), Some("package"));
}

#[test]
fn parses_files_query_and_group() {
    let cli = Cli::parse([
        "files".to_string(),
        "--query".to_string(),
        "layer:frontend,kind:test".to_string(),
        "--group".to_string(),
        "tag".to_string(),
        "--changed".to_string(),
    ])
    .expect("cli should parse");

    assert_eq!(cli.command, Command::Files);
    assert_eq!(
        cli.options.query.as_deref(),
        Some("layer:frontend,kind:test")
    );
    assert_eq!(cli.options.group.as_deref(), Some("tag"));
    assert!(cli.options.changed_only);
}

#[test]
fn parses_stale_patterns() {
    let cli = Cli::parse([
        "stale".to_string(),
        "--patterns".to_string(),
        "one,two".to_string(),
        "--changed".to_string(),
    ])
    .expect("cli should parse");

    assert_eq!(cli.command, Command::Stale);
    assert_eq!(cli.options.patterns, vec!["one", "two"]);
    assert!(cli.options.changed_only);
}

#[test]
fn parses_verify_plan_changed() {
    let cli =
        Cli::parse(["verify-plan".to_string(), "--changed".to_string()]).expect("cli should parse");

    assert_eq!(cli.command, Command::VerifyPlan);
    assert!(cli.options.changed_only);
}

#[test]
fn parses_impact_group() {
    let cli = Cli::parse([
        "impact".to_string(),
        "EditorSelectionRef".to_string(),
        "--group".to_string(),
        "feature".to_string(),
    ])
    .expect("cli should parse");

    assert_eq!(cli.command, Command::Impact);
    assert_eq!(cli.options.query.as_deref(), Some("EditorSelectionRef"));
    assert_eq!(cli.options.group.as_deref(), Some("feature"));
}

#[test]
fn parses_fallout_from() {
    let cli = Cli::parse([
        "fallout".to_string(),
        "--from".to_string(),
        "npm-build.log".to_string(),
    ])
    .expect("cli should parse");

    assert_eq!(cli.command, Command::Fallout);
    assert_eq!(
        cli.options
            .from
            .as_ref()
            .map(|path| path.display().to_string()),
        Some("npm-build.log".to_string())
    );
}

#[test]
fn parses_command_map_query() {
    let cli = Cli::parse(["command-map".to_string(), "append-plan".to_string()])
        .expect("cli should parse");

    assert_eq!(cli.command, Command::CommandMap);
    assert_eq!(cli.options.query.as_deref(), Some("append-plan"));
}

#[test]
fn parses_anchors_query_and_write() {
    let cli = Cli::parse([
        "anchors".to_string(),
        "domain:codemap".to_string(),
        "--write".to_string(),
    ])
    .expect("cli should parse");

    assert_eq!(cli.command, Command::Anchors);
    assert_eq!(cli.options.query.as_deref(), Some("domain:codemap"));
    assert!(cli.options.write);
}

#[test]
fn parses_file_move_plan_options() {
    let cli = Cli::parse([
        "file-move-plan".to_string(),
        "crates/apps/amigo-editor/src/main.tsx".to_string(),
        "--to".to_string(),
        "crates/apps/amigo-editor/src/app/main.tsx".to_string(),
        "--changed".to_string(),
        "--radius".to_string(),
        "42".to_string(),
        "--top".to_string(),
        "12".to_string(),
        "--save".to_string(),
    ])
    .expect("cli should parse");

    assert_eq!(cli.command, Command::FileMovePlan);
    assert_eq!(
        cli.options.query.as_deref(),
        Some("crates/apps/amigo-editor/src/main.tsx")
    );
    assert_eq!(cli.options.radius, 42);
    assert_eq!(cli.options.top, 12);
    assert!(cli.options.changed_only);
    assert_eq!(
        cli.options
            .to
            .as_ref()
            .map(|path| path.display().to_string()),
        Some("crates/apps/amigo-editor/src/app/main.tsx".to_string())
    );
    assert!(cli.options.save);
}

#[test]
fn parses_smells_options() {
    let cli = Cli::parse([
        "smells".to_string(),
        "--top".to_string(),
        "30".to_string(),
        "--why".to_string(),
        "--min-score".to_string(),
        "50".to_string(),
        "--report".to_string(),
        "--file-lines".to_string(),
        "500".to_string(),
        "--include-tests".to_string(),
        "--include-generated".to_string(),
    ])
    .expect("cli should parse");

    assert_eq!(cli.command, Command::Smells);
    assert_eq!(cli.options.top, 30);
    assert_eq!(cli.options.min_score, 50);
    assert!(cli.options.report);
    assert!(cli.options.report_file.is_none());
    assert_eq!(cli.options.file_lines, 500);
    assert!(cli.options.why);
    assert!(cli.options.include_tests);
    assert!(cli.options.include_generated);
}

#[test]
fn parses_smells_report_file() {
    let cli = Cli::parse([
        "smells".to_string(),
        "--report-file".to_string(),
        "artifacts/smells.json".to_string(),
        "--top".to_string(),
        "10".to_string(),
    ])
    .expect("cli should parse");

    assert_eq!(cli.command, Command::Smells);
    assert_eq!(
        cli.options
            .report_file
            .as_ref()
            .map(|path| path.display().to_string()),
        Some("artifacts/smells.json".to_string())
    );
    assert!(!cli.options.report);
}

#[test]
fn parses_refactor_candidates_alias() {
    let cli = Cli::parse([
        "refactor-candidates".to_string(),
        "--changed".to_string(),
        "--group".to_string(),
        "domain".to_string(),
    ])
    .expect("cli should parse");

    assert_eq!(cli.command, Command::Smells);
    assert!(cli.options.changed_only);
    assert_eq!(cli.options.group.as_deref(), Some("domain"));
}

#[test]
fn every_command_variant_has_command_spec() {
    let expected = [
        Command::Scan,
        Command::Refresh,
        Command::Watch,
        Command::Status,
        Command::Changes,
        Command::Files,
        Command::Changed,
        Command::Symbols,
        Command::Where,
        Command::Signature,
        Command::Trace,
        Command::TraceField,
        Command::ChangePlan,
        Command::ExplainFile,
        Command::Neighbors,
        Command::ApiSurface,
        Command::ComponentGraph,
        Command::TauriGraph,
        Command::CallsiteCandidates,
        Command::TodoIndex,
        Command::RiskIndex,
        Command::Smells,
        Command::Compact,
        Command::Explain,
        Command::Brief,
        Command::Find,
        Command::Scope,
        Command::Refs,
        Command::Docs,
        Command::CommandMap,
        Command::Anchors,
        Command::AnchorCheck,
        Command::Taxonomy,
        Command::Verify,
        Command::VerifyPlan,
        Command::Stale,
        Command::Impact,
        Command::Fallout,
        Command::MovePlan,
        Command::Dup,
        Command::TauriCommands,
        Command::ServiceShape,
        Command::RegistryCheck,
        Command::MetadataAudit,
        Command::DescriptorSkeleton,
        Command::OperationsSummary,
        Command::CommitPlan,
        Command::CommitSummary,
        Command::AppendPlan,
        Command::CopyPlan,
        Command::Slice,
        Command::DiffScope,
        Command::DeletePlan,
        Command::FileMovePlan,
        Command::RenamePlan,
        Command::ImportFixPlan,
        Command::OpenSet,
        Command::Workset,
        Command::BarrelCheck,
        Command::OrphanFiles,
        Command::ShimCheck,
        Command::LargeFiles,
        Command::AssetFileCheck,
        Command::CaseCheck,
        Command::TextCheck,
        Command::PatchPreview,
        Command::PatchCheck,
        Command::PatchApply,
        Command::OpsPreview,
        Command::OpsCheck,
        Command::OpsApply,
        Command::OpsRawPreview,
        Command::OpsRawCheck,
        Command::OpsRawApply,
        Command::OpsSkeleton,
        Command::OpsSchema,
        Command::OpsSplit,
        Command::OpsVerify,
        Command::OpsSummary,
        Command::RangeForSymbol,
        Command::RangeForLines,
        Command::AnchorRange,
        Command::CommitFiles,
        Command::ResolveSymbol,
        Command::PreviewEdit,
        Command::CompileEdit,
        Command::ReplaceSymbol,
        Command::ReplaceMethodBody,
        Command::ReplaceRange,
        Command::InsertBeforeSymbol,
        Command::InsertAfterSymbol,
        Command::VerifyScope,
    ];
    let spec_commands = COMMAND_SPECS
        .iter()
        .map(|spec| spec.command)
        .collect::<HashSet<_>>();

    for command in expected {
        assert!(
            spec_commands.contains(&command),
            "missing CommandSpec for {command:?}"
        );
    }
    assert_eq!(spec_commands.len(), expected.len());
}

#[test]
fn every_command_spec_name_and_alias_parses_back() {
    let mut names = HashSet::new();
    for spec in COMMAND_SPECS {
        assert!(
            names.insert(spec.name),
            "duplicate command name {}",
            spec.name
        );
        assert_eq!(parse_command_name(spec.name), Some(spec.command));
        for alias in spec.aliases {
            assert!(names.insert(*alias), "duplicate command alias {alias}");
            assert_eq!(parse_command_name(alias), Some(spec.command));
        }
    }
}

#[test]
fn parses_workset_from_impact_and_split_hints() {
    let cli = Cli::parse([
        "workset".to_string(),
        "selection-migration".to_string(),
        "--from-impact".to_string(),
        "EditorSelectionRef".to_string(),
        "--status".to_string(),
        "--with-split-hints".to_string(),
    ])
    .expect("cli should parse");

    assert_eq!(cli.command, Command::Workset);
    assert_eq!(
        cli.options.from_impact.as_deref(),
        Some("EditorSelectionRef")
    );
    assert!(cli.options.status);
    assert!(cli.options.with_split_hints);
}

#[test]
fn parses_patch_apply_write() {
    let cli = Cli::parse([
        "patch-apply".to_string(),
        "--from".to_string(),
        "patch.diff".to_string(),
        "--write".to_string(),
    ])
    .expect("cli should parse");

    assert_eq!(cli.command, Command::PatchApply);
    assert_eq!(
        cli.options.from.as_deref(),
        Some(std::path::Path::new("patch.diff"))
    );
    assert!(cli.options.write);
}

#[test]
fn parses_where_query() {
    let cli = Cli::parse(["where".to_string(), "CodeMap".to_string()]).expect("cli should parse");
    assert_eq!(cli.command, Command::Where);
    assert_eq!(cli.options.query.as_deref(), Some("CodeMap"));
}

#[test]
fn parses_signature_query() {
    let cli =
        Cli::parse(["signature".to_string(), "CodeMap".to_string()]).expect("cli should parse");
    assert_eq!(cli.command, Command::Signature);
    assert_eq!(cli.options.query.as_deref(), Some("CodeMap"));
}

#[test]
fn parses_trace_query() {
    let cli = Cli::parse(["trace".to_string(), "entity.inspector".to_string()])
        .expect("cli should parse");
    assert_eq!(cli.command, Command::Trace);
    assert_eq!(cli.options.query.as_deref(), Some("entity.inspector"));
}

#[test]
fn parses_ops_raw_check_from_stdin() {
    let cli = Cli::parse([
        "ops-raw-check".to_string(),
        "--from".to_string(),
        "-".to_string(),
    ])
    .expect("parse cli");

    assert_eq!(cli.command, Command::OpsRawCheck);
    assert_eq!(cli.options.from, Some(std::path::PathBuf::from("-")));
}

#[test]
fn parses_ops_raw_apply_with_write() {
    let cli = Cli::parse([
        "ops-raw-apply".to_string(),
        "--from".to_string(),
        "ops.raw".to_string(),
        "--write".to_string(),
    ])
    .expect("parse cli");

    assert_eq!(cli.command, Command::OpsRawApply);
    assert_eq!(cli.options.from, Some(std::path::PathBuf::from("ops.raw")));
    assert!(cli.options.write);
}

#[test]
fn parses_resolve_symbol_filters() {
    let cli = Cli::parse([
        "resolve-symbol".to_string(),
        "--path".to_string(),
        "crates/tools/amigo-codemap/src/cli/options/parser.rs".to_string(),
        "--symbol".to_string(),
        "parse".to_string(),
        "--kind".to_string(),
        "fn".to_string(),
        "--owner".to_string(),
        "impl Parser".to_string(),
        "--visibility".to_string(),
        "pub".to_string(),
        "--json".to_string(),
    ])
    .expect("parse cli");

    assert_eq!(cli.command, Command::ResolveSymbol);
    assert_eq!(
        cli.options.file,
        Some(std::path::PathBuf::from(
            "crates/tools/amigo-codemap/src/cli/options/parser.rs"
        ))
    );
    assert_eq!(cli.options.symbol.as_deref(), Some("parse"));
    assert_eq!(cli.options.kind.as_deref(), Some("fn"));
    assert_eq!(cli.options.owner.as_deref(), Some("impl Parser"));
    assert_eq!(cli.options.visibility.as_deref(), Some("pub"));
    assert!(cli.options.json);
}

#[test]
fn parses_replace_range_content_flags() {
    let cli = Cli::parse([
        "replace-range".to_string(),
        "--path".to_string(),
        "test/main.rs".to_string(),
        "--start-line".to_string(),
        "11".to_string(),
        "--end-line".to_string(),
        "22".to_string(),
        "--with-text".to_string(),
        "this is new code".to_string(),
        "--write".to_string(),
    ])
    .expect("parse cli");

    assert_eq!(cli.command, Command::ReplaceRange);
    assert_eq!(
        cli.options.file,
        Some(std::path::PathBuf::from("test/main.rs"))
    );
    assert_eq!(cli.options.start_line, Some(11));
    assert_eq!(cli.options.end_line, Some(22));
    assert_eq!(cli.options.with_text.as_deref(), Some("this is new code"));
    assert!(cli.options.write);
}

#[test]
fn parses_compile_edit_with_by() {
    let cli = Cli::parse([
        "compile-edit".to_string(),
        "--by".to_string(),
        "replace-symbol".to_string(),
        "--path".to_string(),
        "crates/tools/amigo-codemap/src/cli/help.rs".to_string(),
        "--symbol".to_string(),
        "print_help".to_string(),
        "--with-text".to_string(),
        "replacement".to_string(),
    ])
    .expect("cli should parse");

    assert_eq!(cli.command, Command::CompileEdit);
    assert_eq!(cli.options.by.as_deref(), Some("replace-symbol"));
    assert_eq!(cli.options.symbol.as_deref(), Some("print_help"));
}

#[test]
fn parses_verify_scope_symbol_target() {
    let cli = Cli::parse([
        "verify-scope".to_string(),
        "--path".to_string(),
        "crates/tools/amigo-codemap/src/cli/help.rs".to_string(),
        "--symbol".to_string(),
        "print_help".to_string(),
        "--json".to_string(),
    ])
    .expect("cli should parse");

    assert_eq!(cli.command, Command::VerifyScope);
    assert!(cli.options.json);
}
