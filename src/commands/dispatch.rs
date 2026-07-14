use anyhow::Result;

use crate::cli::{self, Cli, Command};

mod file_ops;
mod normalize;
mod reports;

pub(super) fn run(mut cli: Cli) -> Result<()> {
    use std::time::Instant;

    let command_started = Instant::now();
    let quiet = cli.options.quiet;

    normalize::apply(&mut cli);

    match cli.command {
        Command::Slice
        | Command::AppendPlan
        | Command::CopyPlan
        | Command::DiffScope
        | Command::DeletePlan
        | Command::FileMovePlan
        | Command::RenamePlan
        | Command::ImportFixPlan
        | Command::OpenSet
        | Command::Workset
        | Command::BarrelCheck
        | Command::OrphanFiles
        | Command::ShimCheck
        | Command::LargeFiles
        | Command::Coverage
        | Command::AssetFileCheck
        | Command::CaseCheck
        | Command::TextCheck
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
        | Command::RiskIndex
        | Command::TodoIndex
        | Command::Smells
        | Command::CommitFiles => file_ops::run(cli)?,
        Command::Explain => cli::print_help(),
        _ => reports::run(cli)?,
    }

    if !quiet {
        eprintln!("timing total: {}ms", command_started.elapsed().as_millis());
    }

    Ok(())
}
