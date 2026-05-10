use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow, bail};

use super::path_safety::validate_repo_relative_path;
use super::{OpsEntry, OpsPlan};

pub(super) fn validate_content_sources(root: &Path, plan: &OpsPlan, op: &OpsEntry) -> Result<()> {
    match op {
        OpsEntry::CreateFile {
            content,
            content_from,
            ..
        }
        | OpsEntry::ReplaceFile {
            content,
            content_from,
            ..
        }
        | OpsEntry::ReplaceRange {
            content,
            content_from,
            ..
        }
        | OpsEntry::AppendToFile {
            content,
            content_from,
            ..
        }
        | OpsEntry::InsertBeforeText {
            content,
            content_from,
            ..
        }
        | OpsEntry::InsertAfterText {
            content,
            content_from,
            ..
        }
        | OpsEntry::InsertBeforeAnchor {
            content,
            content_from,
            ..
        }
        | OpsEntry::InsertAfterAnchor {
            content,
            content_from,
            ..
        }
        | OpsEntry::ReplaceBetweenAnchors {
            content,
            content_from,
            ..
        }
        | OpsEntry::ReplaceSymbol {
            content,
            content_from,
            ..
        }
        | OpsEntry::InsertBeforeSymbol {
            content,
            content_from,
            ..
        }
        | OpsEntry::InsertAfterSymbol {
            content,
            content_from,
            ..
        }
        | OpsEntry::ReplaceMethodBody {
            content,
            content_from,
            ..
        } => {
            let _ = op_content(root, plan, content.as_deref(), content_from.as_deref())?;
        }
        OpsEntry::ReplaceText {
            replace,
            content_from,
            ..
        } => {
            let _ = op_content(root, plan, replace.as_deref(), content_from.as_deref())?;
        }
        OpsEntry::DeleteRange { .. }
        | OpsEntry::DeleteFile { .. }
        | OpsEntry::CopyFile { .. }
        | OpsEntry::MoveFile { .. }
        | OpsEntry::RenameFile { .. }
        | OpsEntry::CreateDir { .. }
        | OpsEntry::DeleteDir { .. }
        | OpsEntry::DeleteSymbol { .. } => {}
    }
    Ok(())
}

pub(super) fn op_content(
    root: &Path,
    plan: &OpsPlan,
    inline: Option<&str>,
    content_from: Option<&Path>,
) -> Result<String> {
    match (inline, content_from) {
        (Some(_), Some(_)) => bail!("op must use only one of content/replace or content_from"),
        (Some(content), None) => Ok(content.to_owned()),
        (None, Some(path)) => {
            validate_repo_relative_path(path)?;
            fs::read_to_string(content_file_path(root, plan, path)?)
                .map_err(|error| anyhow!("failed to read content_from {}: {error}", path.display()))
        }
        (None, None) => bail!("op requires content/replace or content_from"),
    }
}

fn content_file_path(root: &Path, plan: &OpsPlan, content_from: &Path) -> Result<PathBuf> {
    let base = if let Some(plan_dir) = &plan.plan_dir {
        if plan_dir.is_absolute() {
            plan_dir.clone()
        } else {
            root.join(plan_dir)
        }
    } else {
        root.to_path_buf()
    };
    let base = if let Some(content_root) = &plan.content_root {
        validate_repo_relative_path(content_root)?;
        base.join(content_root)
    } else {
        base
    };
    Ok(base.join(content_from))
}

pub(super) fn op_content_from_paths(op: &OpsEntry) -> Vec<&Path> {
    match op {
        OpsEntry::CreateFile { content_from, .. }
        | OpsEntry::ReplaceFile { content_from, .. }
        | OpsEntry::ReplaceRange { content_from, .. }
        | OpsEntry::AppendToFile { content_from, .. }
        | OpsEntry::InsertBeforeText { content_from, .. }
        | OpsEntry::InsertAfterText { content_from, .. }
        | OpsEntry::ReplaceText { content_from, .. }
        | OpsEntry::InsertBeforeAnchor { content_from, .. }
        | OpsEntry::InsertAfterAnchor { content_from, .. }
        | OpsEntry::ReplaceBetweenAnchors { content_from, .. }
        | OpsEntry::ReplaceSymbol { content_from, .. }
        | OpsEntry::InsertBeforeSymbol { content_from, .. }
        | OpsEntry::InsertAfterSymbol { content_from, .. }
        | OpsEntry::ReplaceMethodBody { content_from, .. } => {
            content_from.as_deref().into_iter().collect()
        }
        OpsEntry::DeleteRange { .. }
        | OpsEntry::DeleteFile { .. }
        | OpsEntry::CopyFile { .. }
        | OpsEntry::MoveFile { .. }
        | OpsEntry::RenameFile { .. }
        | OpsEntry::CreateDir { .. }
        | OpsEntry::DeleteDir { .. }
        | OpsEntry::DeleteSymbol { .. } => Vec::new(),
    }
}
