use std::fs;
use std::path::Path;

use anyhow::{Result, bail};

use crate::model::CodeMap;

use super::OpsPlan;
use super::content::op_content_from_paths;
use super::describe::{op_id, op_kind, op_path};
use super::hash::short_hash;
use super::model::OpsEntry;
use super::path_safety::repo_path;
use super::validate::validate_op;

mod handlers;

pub(super) fn print_ops_apply(
    root: &Path,
    map: Option<&CodeMap>,
    from: Option<&Path>,
    yaml: Option<&str>,
    input_format: super::OpsInputFormat,
    write: bool,
    backup: bool,
    stop_on_error: bool,
    strict: bool,
    _limit: usize,
    verbose: bool,
) -> Result<()> {
    let plan = super::read_plan_with_format(from, yaml, input_format)?;
    if !write {
        println!("ops-apply: dry-run only; pass --write");
        return super::print_ops_check(root, map, from, yaml, input_format, strict, _limit);
    }

    if verbose {
        println!(
            "ops-apply: task={}",
            plan.task.as_deref().unwrap_or("ops-plan")
        );
        if let Some(from) = from {
            println!("ops-apply: plan={}", from.display());
        }
        if let Some(content_root) = &plan.content_root {
            println!("ops-apply: content_root={}", content_root.display());
        }
        println!(
            "ops-apply: ops={} write={} backup={} strict={} stop_on_error={}",
            plan.ops.len(),
            write,
            backup,
            strict,
            stop_on_error
        );
    }

    if backup {
        backup_plan_files(root, &plan)?;
    }

    let mut applied = 0usize;
    let mut failed = 0usize;
    for (index, op) in plan.ops.iter().enumerate() {
        if verbose {
            if let Err(error) = print_apply_op_header(root, &plan, op, index, plan.ops.len()) {
                println!("  verbose: failed to inspect before state: {error}");
            }
        }
        if let Err(error) = validate_op(root, &plan, map, op, strict)
            .and_then(|_| handlers::apply_op(root, &plan, map, op, write))
        {
            failed += 1;
            if verbose {
                println!("  result: failed");
                println!("  reason: {error}");
            } else {
                println!("failed {}: {error}", super::describe::describe_op(op));
            }
            if stop_on_error {
                break;
            }
        } else {
            applied += 1;
            if verbose {
                if let Err(error) = print_apply_op_after(root, op) {
                    println!("  verbose: failed to inspect after state: {error}");
                }
                println!("  result: applied");
            } else {
                println!("applied {}", super::describe::describe_op(op));
            }
        }
    }
    println!("ops-apply: applied={applied} failed={failed}");
    if failed > 0 {
        bail!("ops-apply failed: applied={applied} failed={failed}");
    }
    Ok(())
}

fn print_apply_op_header(
    root: &Path,
    plan: &OpsPlan,
    op: &OpsEntry,
    index: usize,
    total: usize,
) -> Result<()> {
    println!(
        "[{}/{}] {} {}",
        index + 1,
        total,
        op_kind(op),
        op_id(op).unwrap_or("-")
    );
    println!("  path: {}", op_path(op));
    println!("  source: {}", op_content_source(op));
    for path in super::describe::op_paths(op) {
        let full = repo_path(root, path)?;
        println!("  before {}: {}", path.display(), describe_file(&full));
    }
    if let Some(plan_dir) = &plan.plan_dir {
        println!("  plan_dir: {}", plan_dir.display());
    }
    Ok(())
}

fn print_apply_op_after(root: &Path, op: &OpsEntry) -> Result<()> {
    for path in super::describe::op_paths(op) {
        let full = repo_path(root, path)?;
        println!("  after {}: {}", path.display(), describe_file(&full));
    }
    Ok(())
}

fn describe_file(path: &Path) -> String {
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => "exists yes, directory".to_string(),
        Ok(metadata) => match fs::read(path) {
            Ok(bytes) => {
                let lines = String::from_utf8_lossy(&bytes).lines().count();
                format!(
                    "exists yes, {lines} lines, {} bytes, hash {}",
                    metadata.len(),
                    short_hash(&bytes)
                )
            }
            Err(_) => format!("exists yes, {} bytes, hash unavailable", metadata.len()),
        },
        Err(_) => "exists no".to_string(),
    }
}

fn op_content_source(op: &OpsEntry) -> String {
    let sources = op_content_from_paths(op);
    if sources.is_empty() {
        if matches!(
            op,
            OpsEntry::DeleteRange { .. }
                | OpsEntry::DeleteFile { .. }
                | OpsEntry::CopyFile { .. }
                | OpsEntry::MoveFile { .. }
                | OpsEntry::RenameFile { .. }
                | OpsEntry::CreateDir { .. }
                | OpsEntry::DeleteDir { .. }
                | OpsEntry::DeleteSymbol { .. }
        ) {
            "none".to_string()
        } else {
            "inline content".to_string()
        }
    } else {
        sources
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn backup_plan_files(root: &Path, plan: &OpsPlan) -> Result<()> {
    let task = plan
        .task
        .as_deref()
        .unwrap_or("ops-plan")
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>();
    let backup_root = root.join(".amigo").join("ops-backups").join(task);
    for op in &plan.ops {
        let path = std::path::PathBuf::from(op_path(op));
        let source = repo_path(root, &path)?;
        if !source.exists() || source.is_dir() {
            continue;
        }
        let target = backup_root.join(&path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&source, &target)?;
    }
    println!("backup: {}", backup_root.display());
    Ok(())
}
