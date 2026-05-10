use std::path::Path;

use anyhow::{Result, bail};

use crate::model::CodeMap;
use describe::{
    describe_op, locator_confidence, locator_kind, op_id, op_kind, op_path, risk_for_op,
    safety_line, safety_reason_for_op,
};

mod content;
mod describe;
mod executor;
mod hash;
mod model;
mod parse;
mod path_safety;
mod validate;

pub use model::{OpsEntry, OpsInputFormat, OpsPlan};

pub fn read_plan_with_format(
    from: Option<&Path>,
    yaml: Option<&str>,
    input_format: OpsInputFormat,
) -> Result<OpsPlan> {
    parse::read_plan_with_format(from, yaml, input_format)
}

pub fn print_ops_preview(
    root: &Path,
    from: Option<&Path>,
    yaml: Option<&str>,
    input_format: OpsInputFormat,
    limit: usize,
) -> Result<()> {
    let plan = read_plan_with_format(from, yaml, input_format)?;
    println!("ops-preview: version {}", plan.version);
    if let Some(task) = &plan.task {
        println!("task: {task}");
    }
    if let Some(description) = &plan.description {
        println!("description: {description}");
    }
    let mut by_kind = std::collections::BTreeMap::<String, Vec<String>>::new();
    for (index, op) in plan.ops.iter().take(limit).enumerate() {
        println!("  {}. {}", index + 1, describe_op(op));
        by_kind
            .entry(op_kind(op).to_string())
            .or_default()
            .push(op_path(op));
    }
    println!("files:");
    for (kind, paths) in by_kind {
        println!("  {kind}:");
        for path in paths {
            println!("    - {path}");
        }
    }
    if !plan.verify.is_empty() {
        println!("verify:");
        for command in &plan.verify {
            println!("  {command}");
        }
    }
    println!("next:");
    println!("  1. ops-check --from <plan.yml>");
    println!("  2. ops-apply --from <plan.yml> --write");
    let _ = root;
    Ok(())
}

pub fn plan_requires_codemap(
    from: Option<&Path>,
    yaml: Option<&str>,
    input_format: OpsInputFormat,
    strict: bool,
) -> Result<bool> {
    let plan = read_plan_with_format(from, yaml, input_format)?;
    Ok(plan.ops.iter().any(|op| op_requires_codemap(op, strict)))
}

pub fn print_ops_check(
    root: &Path,
    map: Option<&CodeMap>,
    from: Option<&Path>,
    yaml: Option<&str>,
    input_format: OpsInputFormat,
    strict: bool,
    limit: usize,
) -> Result<()> {
    let plan = read_plan_with_format(from, yaml, input_format)?;
    println!("ops-check: version {}", plan.version);
    if let Some(task) = &plan.task {
        println!("task: {task}");
    }

    let mut errors = Vec::new();
    let mut virtual_existing = std::collections::BTreeSet::<String>::new();
    for (index, op) in plan.ops.iter().take(limit).enumerate() {
        let description = describe_op(op);
        let valid = match validate::validate_op_for_check(
            root,
            &plan,
            map,
            op,
            strict,
            &virtual_existing,
        ) {
            Ok(()) => {
                println!("  {}. {} applies: yes", index + 1, description);
                println!("     id: {}", op_id(op).unwrap_or("-"));
                println!(
                    "     locator: kind={} confidence={}",
                    locator_kind(op),
                    locator_confidence(op)
                );
                println!("     safety: {}", safety_line(op, strict));
                println!("     risk: {}", risk_for_op(op));
                println!("     reason: {}", safety_reason_for_op(op));
                true
            }
            Err(error) => {
                println!("  {}. {} applies: no", index + 1, description);
                println!("     risk: high");
                println!("     reason: {error}");
                errors.push(error.to_string());
                false
            }
        };
        if valid {
            record_virtual_effect(op, &mut virtual_existing);
        }
    }

    if errors.is_empty() {
        println!("ops-check: ok");
    } else {
        println!("ops-check: failed");
        for error in errors {
            println!("  {error}");
        }
        bail!("ops-check failed");
    }
    Ok(())
}

pub fn print_ops_apply(
    root: &Path,
    map: Option<&CodeMap>,
    from: Option<&Path>,
    yaml: Option<&str>,
    input_format: OpsInputFormat,
    write: bool,
    backup: bool,
    stop_on_error: bool,
    strict: bool,
    limit: usize,
    verbose: bool,
) -> Result<()> {
    executor::print_ops_apply(
        root,
        map,
        from,
        yaml,
        input_format,
        write,
        backup,
        stop_on_error,
        strict,
        limit,
        verbose,
    )
}

fn op_requires_codemap(op: &OpsEntry, strict: bool) -> bool {
    match op {
        OpsEntry::ReplaceText {
            within_symbol: Some(_),
            ..
        }
        | OpsEntry::ReplaceSymbol { .. }
        | OpsEntry::DeleteSymbol { .. }
        | OpsEntry::InsertBeforeSymbol { .. }
        | OpsEntry::InsertAfterSymbol { .. }
        | OpsEntry::ReplaceMethodBody { .. } => true,
        _ if strict && matches!(locator_kind(op), "symbol") => true,
        _ => false,
    }
}

fn record_virtual_effect(op: &OpsEntry, virtual_existing: &mut std::collections::BTreeSet<String>) {
    match op {
        OpsEntry::CreateFile { path, .. }
        | OpsEntry::ReplaceFile { path, .. }
        | OpsEntry::CreateDir { path, .. } => {
            virtual_existing.insert(path_key(path));
        }
        OpsEntry::CopyFile { to, .. } => {
            virtual_existing.insert(path_key(to));
        }
        OpsEntry::MoveFile { from, to, .. } | OpsEntry::RenameFile { from, to, .. } => {
            virtual_existing.remove(&path_key(from));
            virtual_existing.insert(path_key(to));
        }
        OpsEntry::DeleteFile { path, .. } | OpsEntry::DeleteDir { path, .. } => {
            virtual_existing.remove(&path_key(path));
        }
        _ => {}
    }
}

fn path_key(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
