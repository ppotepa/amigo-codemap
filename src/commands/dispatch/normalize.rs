use crate::cli::{Cli, Command};

pub(super) fn apply(cli: &mut Cli) {
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
        | Command::OpsRawPreview
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
        | Command::Smells
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
}
