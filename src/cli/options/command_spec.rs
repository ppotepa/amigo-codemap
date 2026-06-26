use crate::cli::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScanPolicy {
    Preserve,
    Level0NoAi,
    MinLevel(u8),
    ForceLevel(u8),
    Slice,
    OpsCheck,
    Workset,
    OrphanFiles,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PositionalMode {
    None,
    Query,
    VerifyArgs,
    RangeLines,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CommandFamily {
    Snapshot,
    Navigation,
    Analysis,
    Planning,
    Patch,
    Ops,
    Verify,
    Git,
    Meta,
    Summary,
}

impl CommandFamily {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Snapshot => "snapshot",
            Self::Navigation => "navigation",
            Self::Analysis => "analysis",
            Self::Planning => "planning",
            Self::Patch => "patch",
            Self::Ops => "ops",
            Self::Verify => "verify",
            Self::Git => "git",
            Self::Meta => "meta",
            Self::Summary => "summary",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandRoute {
    Snapshot,
    Report,
    FileOps,
    Ops,
    Edit,
    Help,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CommandSpec {
    pub command: Command,
    pub name: &'static str,
    pub aliases: &'static [&'static str],
}

pub(crate) const COMMAND_SPECS: &[CommandSpec] = &[
    CommandSpec {
        command: Command::Scan,
        name: "scan",
        aliases: &[],
    },
    CommandSpec {
        command: Command::Refresh,
        name: "refresh",
        aliases: &[],
    },
    CommandSpec {
        command: Command::Watch,
        name: "watch",
        aliases: &[],
    },
    CommandSpec {
        command: Command::Status,
        name: "status",
        aliases: &[],
    },
    CommandSpec {
        command: Command::Changes,
        name: "changes",
        aliases: &[],
    },
    CommandSpec {
        command: Command::Files,
        name: "files",
        aliases: &[],
    },
    CommandSpec {
        command: Command::Changed,
        name: "changed",
        aliases: &[],
    },
    CommandSpec {
        command: Command::Symbols,
        name: "symbols",
        aliases: &[],
    },
    CommandSpec {
        command: Command::Where,
        name: "where",
        aliases: &[],
    },
    CommandSpec {
        command: Command::Signature,
        name: "signature",
        aliases: &[],
    },
    CommandSpec {
        command: Command::Trace,
        name: "trace",
        aliases: &[],
    },
    CommandSpec {
        command: Command::TraceField,
        name: "trace-field",
        aliases: &[],
    },
    CommandSpec {
        command: Command::ChangePlan,
        name: "change-plan",
        aliases: &[],
    },
    CommandSpec {
        command: Command::ExplainFile,
        name: "explain-file",
        aliases: &[],
    },
    CommandSpec {
        command: Command::Neighbors,
        name: "neighbors",
        aliases: &[],
    },
    CommandSpec {
        command: Command::ApiSurface,
        name: "api-surface",
        aliases: &[],
    },
    CommandSpec {
        command: Command::ComponentGraph,
        name: "component-graph",
        aliases: &[],
    },
    CommandSpec {
        command: Command::Plugins,
        name: "plugins",
        aliases: &["plugin-graph"],
    },
    CommandSpec {
        command: Command::TauriGraph,
        name: "tauri-graph",
        aliases: &[],
    },
    CommandSpec {
        command: Command::CallsiteCandidates,
        name: "callsite-candidates",
        aliases: &[],
    },
    CommandSpec {
        command: Command::TodoIndex,
        name: "todo-index",
        aliases: &[],
    },
    CommandSpec {
        command: Command::RiskIndex,
        name: "risk-index",
        aliases: &[],
    },
    CommandSpec {
        command: Command::Smells,
        name: "smells",
        aliases: &["refactor-candidates"],
    },
    CommandSpec {
        command: Command::Compact,
        name: "compact",
        aliases: &[],
    },
    CommandSpec {
        command: Command::Explain,
        name: "explain",
        aliases: &["--help", "-h"],
    },
    CommandSpec {
        command: Command::Brief,
        name: "brief",
        aliases: &[],
    },
    CommandSpec {
        command: Command::Find,
        name: "find",
        aliases: &[],
    },
    CommandSpec {
        command: Command::Scope,
        name: "scope",
        aliases: &[],
    },
    CommandSpec {
        command: Command::Refs,
        name: "refs",
        aliases: &[],
    },
    CommandSpec {
        command: Command::Docs,
        name: "docs",
        aliases: &["readme-coverage"],
    },
    CommandSpec {
        command: Command::CommandMap,
        name: "command-map",
        aliases: &[],
    },
    CommandSpec {
        command: Command::Anchors,
        name: "anchors",
        aliases: &[],
    },
    CommandSpec {
        command: Command::AnchorCheck,
        name: "anchor-check",
        aliases: &[],
    },
    CommandSpec {
        command: Command::Taxonomy,
        name: "taxonomy",
        aliases: &[],
    },
    CommandSpec {
        command: Command::Verify,
        name: "verify",
        aliases: &[],
    },
    CommandSpec {
        command: Command::VerifyPlan,
        name: "verify-plan",
        aliases: &[],
    },
    CommandSpec {
        command: Command::ArchGuard,
        name: "arch-guard",
        aliases: &[],
    },
    CommandSpec {
        command: Command::Stale,
        name: "stale",
        aliases: &[],
    },
    CommandSpec {
        command: Command::Impact,
        name: "impact",
        aliases: &[],
    },
    CommandSpec {
        command: Command::Fallout,
        name: "fallout",
        aliases: &[],
    },
    CommandSpec {
        command: Command::MovePlan,
        name: "move-plan",
        aliases: &[],
    },
    CommandSpec {
        command: Command::Dup,
        name: "dup",
        aliases: &[],
    },
    CommandSpec {
        command: Command::TauriCommands,
        name: "tauri-commands",
        aliases: &[],
    },
    CommandSpec {
        command: Command::ServiceShape,
        name: "service-shape",
        aliases: &[],
    },
    CommandSpec {
        command: Command::RegistryCheck,
        name: "registry-check",
        aliases: &[],
    },
    CommandSpec {
        command: Command::MetadataAudit,
        name: "metadata-audit",
        aliases: &[],
    },
    CommandSpec {
        command: Command::DescriptorSkeleton,
        name: "descriptor-skeleton",
        aliases: &[],
    },
    CommandSpec {
        command: Command::OperationsSummary,
        name: "operations-summary",
        aliases: &[],
    },
    CommandSpec {
        command: Command::CommitPlan,
        name: "commit-plan",
        aliases: &[],
    },
    CommandSpec {
        command: Command::CommitSummary,
        name: "commit-summary",
        aliases: &[],
    },
    CommandSpec {
        command: Command::AppendPlan,
        name: "append-plan",
        aliases: &[],
    },
    CommandSpec {
        command: Command::CopyPlan,
        name: "copy-plan",
        aliases: &[],
    },
    CommandSpec {
        command: Command::Slice,
        name: "slice",
        aliases: &[],
    },
    CommandSpec {
        command: Command::DiffScope,
        name: "diff-scope",
        aliases: &[],
    },
    CommandSpec {
        command: Command::DeletePlan,
        name: "delete-plan",
        aliases: &[],
    },
    CommandSpec {
        command: Command::FileMovePlan,
        name: "file-move-plan",
        aliases: &[],
    },
    CommandSpec {
        command: Command::RenamePlan,
        name: "rename-plan",
        aliases: &[],
    },
    CommandSpec {
        command: Command::ImportFixPlan,
        name: "import-fix-plan",
        aliases: &[],
    },
    CommandSpec {
        command: Command::OpenSet,
        name: "open-set",
        aliases: &[],
    },
    CommandSpec {
        command: Command::Workset,
        name: "workset",
        aliases: &[],
    },
    CommandSpec {
        command: Command::BarrelCheck,
        name: "barrel-check",
        aliases: &[],
    },
    CommandSpec {
        command: Command::OrphanFiles,
        name: "orphan-files",
        aliases: &[],
    },
    CommandSpec {
        command: Command::ShimCheck,
        name: "shim-check",
        aliases: &[],
    },
    CommandSpec {
        command: Command::LargeFiles,
        name: "large-files",
        aliases: &[],
    },
    CommandSpec {
        command: Command::AssetFileCheck,
        name: "asset-file-check",
        aliases: &[],
    },
    CommandSpec {
        command: Command::CaseCheck,
        name: "case-check",
        aliases: &[],
    },
    CommandSpec {
        command: Command::TextCheck,
        name: "text-check",
        aliases: &[],
    },
    CommandSpec {
        command: Command::PatchPreview,
        name: "patch-preview",
        aliases: &[],
    },
    CommandSpec {
        command: Command::PatchCheck,
        name: "patch-check",
        aliases: &[],
    },
    CommandSpec {
        command: Command::PatchApply,
        name: "patch-apply",
        aliases: &[],
    },
    CommandSpec {
        command: Command::OpsPreview,
        name: "ops-preview",
        aliases: &[],
    },
    CommandSpec {
        command: Command::OpsCheck,
        name: "ops-check",
        aliases: &[],
    },
    CommandSpec {
        command: Command::OpsApply,
        name: "ops-apply",
        aliases: &[],
    },
    CommandSpec {
        command: Command::OpsRawPreview,
        name: "ops-raw-preview",
        aliases: &[],
    },
    CommandSpec {
        command: Command::OpsRawCheck,
        name: "ops-raw-check",
        aliases: &[],
    },
    CommandSpec {
        command: Command::OpsRawApply,
        name: "ops-raw-apply",
        aliases: &[],
    },
    CommandSpec {
        command: Command::OpsSkeleton,
        name: "ops-skeleton",
        aliases: &[],
    },
    CommandSpec {
        command: Command::OpsSchema,
        name: "ops-schema",
        aliases: &[],
    },
    CommandSpec {
        command: Command::OpsSplit,
        name: "ops-split",
        aliases: &[],
    },
    CommandSpec {
        command: Command::OpsVerify,
        name: "ops-verify",
        aliases: &[],
    },
    CommandSpec {
        command: Command::OpsSummary,
        name: "ops-summary",
        aliases: &[],
    },
    CommandSpec {
        command: Command::RangeForSymbol,
        name: "range-for-symbol",
        aliases: &[],
    },
    CommandSpec {
        command: Command::RangeForLines,
        name: "range-for-lines",
        aliases: &[],
    },
    CommandSpec {
        command: Command::AnchorRange,
        name: "anchor-range",
        aliases: &[],
    },
    CommandSpec {
        command: Command::CommitFiles,
        name: "commit-files",
        aliases: &[],
    },
    CommandSpec {
        command: Command::ResolveSymbol,
        name: "resolve-symbol",
        aliases: &[],
    },
    CommandSpec {
        command: Command::PreviewEdit,
        name: "preview-edit",
        aliases: &[],
    },
    CommandSpec {
        command: Command::CompileEdit,
        name: "compile-edit",
        aliases: &[],
    },
    CommandSpec {
        command: Command::ReplaceSymbol,
        name: "replace-symbol",
        aliases: &[],
    },
    CommandSpec {
        command: Command::ReplaceMethodBody,
        name: "replace-method-body",
        aliases: &[],
    },
    CommandSpec {
        command: Command::ReplaceRange,
        name: "replace-range",
        aliases: &[],
    },
    CommandSpec {
        command: Command::InsertBeforeSymbol,
        name: "insert-before-symbol",
        aliases: &[],
    },
    CommandSpec {
        command: Command::InsertAfterSymbol,
        name: "insert-after-symbol",
        aliases: &[],
    },
    CommandSpec {
        command: Command::VerifyScope,
        name: "verify-scope",
        aliases: &[],
    },
];

pub(crate) fn command_spec_by_name(value: &str) -> Option<&'static CommandSpec> {
    COMMAND_SPECS
        .iter()
        .find(|spec| spec.name == value || spec.aliases.contains(&value))
}

pub(crate) fn command_family(command: Command) -> CommandFamily {
    match command {
        Command::Scan | Command::Refresh | Command::Status | Command::Watch => {
            CommandFamily::Snapshot
        }
        Command::CommandMap
        | Command::Docs
        | Command::Brief
        | Command::Explain
        | Command::Anchors
        | Command::AnchorCheck
        | Command::Taxonomy => CommandFamily::Meta,
        Command::Changes
        | Command::Files
        | Command::Changed
        | Command::CommitPlan
        | Command::CommitSummary
        | Command::CommitFiles => CommandFamily::Git,
        Command::Verify
        | Command::VerifyPlan
        | Command::VerifyScope
        | Command::ArchGuard
        | Command::Fallout => CommandFamily::Verify,
        Command::PatchPreview
        | Command::PatchCheck
        | Command::PatchApply
        | Command::PreviewEdit
        | Command::CompileEdit
        | Command::ResolveSymbol
        | Command::ReplaceSymbol
        | Command::ReplaceMethodBody
        | Command::ReplaceRange
        | Command::InsertBeforeSymbol
        | Command::InsertAfterSymbol => CommandFamily::Patch,
        Command::OpsPreview
        | Command::OpsCheck
        | Command::OpsApply
        | Command::OpsRawPreview
        | Command::OpsRawCheck
        | Command::OpsRawApply
        | Command::OpsSkeleton
        | Command::OpsSchema
        | Command::OpsSplit
        | Command::OpsVerify
        | Command::OpsSummary => CommandFamily::Ops,
        Command::Symbols
        | Command::Where
        | Command::Signature
        | Command::Trace
        | Command::TraceField
        | Command::Refs
        | Command::Scope
        | Command::Slice
        | Command::RangeForSymbol
        | Command::RangeForLines
        | Command::AnchorRange
        | Command::OpenSet
        | Command::Neighbors
        | Command::ExplainFile
        | Command::ChangePlan
        | Command::Find
        | Command::Compact => CommandFamily::Navigation,
        Command::AppendPlan
        | Command::CopyPlan
        | Command::DeletePlan
        | Command::FileMovePlan
        | Command::RenamePlan
        | Command::ImportFixPlan
        | Command::DescriptorSkeleton
        | Command::Workset
        | Command::DiffScope => CommandFamily::Planning,
        Command::OperationsSummary => CommandFamily::Summary,
        Command::Impact
        | Command::MovePlan
        | Command::Dup
        | Command::ServiceShape
        | Command::RegistryCheck
        | Command::MetadataAudit
        | Command::ApiSurface
        | Command::ComponentGraph
        | Command::Plugins
        | Command::TauriGraph
        | Command::CallsiteCandidates
        | Command::TodoIndex
        | Command::RiskIndex
        | Command::Smells
        | Command::LargeFiles
        | Command::Stale
        | Command::ShimCheck
        | Command::BarrelCheck
        | Command::OrphanFiles
        | Command::AssetFileCheck
        | Command::CaseCheck
        | Command::TextCheck
        | Command::TauriCommands => CommandFamily::Analysis,
    }
}
pub(crate) fn command_route(command: Command) -> CommandRoute {
    match command_family(command) {
        CommandFamily::Snapshot => CommandRoute::Snapshot,
        CommandFamily::Patch => CommandRoute::Edit,
        CommandFamily::Ops => CommandRoute::Ops,
        CommandFamily::Meta if matches!(command, Command::Brief | Command::Docs) => {
            CommandRoute::Help
        }
        CommandFamily::Navigation | CommandFamily::Planning => CommandRoute::FileOps,
        CommandFamily::Analysis
        | CommandFamily::Verify
        | CommandFamily::Git
        | CommandFamily::Summary
        | CommandFamily::Meta => CommandRoute::Report,
    }
}

pub(crate) fn command_scan_policy(command: Command) -> ScanPolicy {
    match command {
        Command::Brief
        | Command::Changed
        | Command::Changes
        | Command::Find
        | Command::Docs
        | Command::CommandMap
        | Command::Files
        | Command::VerifyPlan
        | Command::ArchGuard
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
        | Command::OpsRawPreview
        | Command::RiskIndex
        | Command::CommitFiles => ScanPolicy::Level0NoAi,
        Command::Scope
        | Command::Refs
        | Command::Anchors
        | Command::AnchorCheck
        | Command::Impact
        | Command::ServiceShape
        | Command::Where
        | Command::Symbols
        | Command::Signature
        | Command::Trace
        | Command::ChangePlan
        | Command::ExplainFile
        | Command::Neighbors
        | Command::ApiSurface
        | Command::ComponentGraph
        | Command::Plugins
        | Command::TauriGraph
        | Command::CallsiteCandidates
        | Command::TodoIndex
        | Command::OpsApply
        | Command::OpsRawCheck
        | Command::OpsRawApply
        | Command::OpsSkeleton
        | Command::OpsSchema
        | Command::OpsSplit
        | Command::OpsVerify
        | Command::OpsSummary
        | Command::RangeForSymbol
        | Command::RangeForLines
        | Command::AnchorRange
        | Command::ResolveSymbol
        | Command::PreviewEdit
        | Command::CompileEdit
        | Command::ReplaceSymbol
        | Command::ReplaceMethodBody
        | Command::ReplaceRange
        | Command::InsertBeforeSymbol
        | Command::InsertAfterSymbol
        | Command::VerifyScope
        | Command::OpenSet
        | Command::LargeFiles
        | Command::Smells
        | Command::PatchPreview
        | Command::AppendPlan
        | Command::CopyPlan => ScanPolicy::MinLevel(2),
        Command::OpsCheck => ScanPolicy::OpsCheck,
        Command::Slice => ScanPolicy::Slice,
        Command::Workset => ScanPolicy::Workset,
        Command::OrphanFiles => ScanPolicy::OrphanFiles,
        Command::Refresh => ScanPolicy::ForceLevel(2),
        _ => ScanPolicy::Preserve,
    }
}

pub(crate) fn command_positional_mode(command: Command) -> PositionalMode {
    match command {
        Command::Find
        | Command::Symbols
        | Command::Where
        | Command::Signature
        | Command::Trace
        | Command::TraceField
        | Command::ChangePlan
        | Command::ExplainFile
        | Command::Neighbors
        | Command::CallsiteCandidates
        | Command::Scope
        | Command::Refs
        | Command::Docs
        | Command::CommandMap
        | Command::Anchors
        | Command::Impact
        | Command::MovePlan
        | Command::Dup
        | Command::ServiceShape
        | Command::RegistryCheck
        | Command::MetadataAudit
        | Command::DescriptorSkeleton
        | Command::AppendPlan
        | Command::CopyPlan
        | Command::Slice
        | Command::DeletePlan
        | Command::FileMovePlan
        | Command::RenamePlan
        | Command::OpenSet
        | Command::Workset
        | Command::BarrelCheck
        | Command::OrphanFiles
        | Command::AssetFileCheck
        | Command::PatchPreview
        | Command::PatchCheck
        | Command::PatchApply
        | Command::OpsPreview
        | Command::OpsCheck
        | Command::OpsApply
        | Command::OpsRawPreview
        | Command::OpsRawCheck
        | Command::OpsRawApply
        | Command::OpsSkeleton
        | Command::OpsSchema
        | Command::OpsSplit
        | Command::OpsVerify
        | Command::OpsSummary
        | Command::RangeForSymbol
        | Command::AnchorRange
        | Command::PreviewEdit
        | Command::CompileEdit
        | Command::CommitFiles => PositionalMode::Query,
        Command::RangeForLines => PositionalMode::RangeLines,
        Command::Verify => PositionalMode::VerifyArgs,
        _ => PositionalMode::None,
    }
}
