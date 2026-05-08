use std::path::Path;

use anyhow::Result;

use super::ops_plan::{OpsEntry, read_plan};

pub fn print_ops_verify(from: Option<&Path>, yaml: Option<&str>, run: bool) -> Result<()> {
    let plan = read_plan(from, yaml)?;
    println!("ops-verify:");
    if plan.verify.is_empty() {
        println!("  none");
    } else {
        for command in &plan.verify {
            println!("  {command}");
        }
    }
    if run {
        println!("run: skipped; execute listed verify commands explicitly");
    }
    Ok(())
}

pub fn print_ops_summary(from: Option<&Path>, yaml: Option<&str>, changed: bool) -> Result<()> {
    let plan = read_plan(from, yaml)?;
    let task = plan.task.as_deref().unwrap_or("Ops Plan");
    println!("### {}", title(task));
    println!(
        "- Task: {}{}",
        plan.description.as_deref().unwrap_or(task),
        if changed { " (changed scope)" } else { "" }
    );
    println!("- Ops: {}.", op_kinds(&plan.ops).join(", "));
    println!("- Files: {}.", op_paths(&plan.ops).join(", "));
    println!(
        "- Verify: {}.",
        if plan.verify.is_empty() {
            "not specified".to_string()
        } else {
            plan.verify.join(", ")
        }
    );
    println!("- Tokens: used ~TODO, saved ~TODO.");
    Ok(())
}

pub fn print_ops_split(from: Option<&Path>, yaml: Option<&str>, by: Option<&str>) -> Result<()> {
    let plan = read_plan(from, yaml)?;
    let by = by.unwrap_or("domain");
    let mut groups = std::collections::BTreeMap::<String, Vec<&OpsEntry>>::new();
    for op in &plan.ops {
        groups.entry(group_key(op, by)).or_default().push(op);
    }
    for (index, (group, ops)) in groups.into_iter().enumerate() {
        let name = format!("{:03}-{}.yml", index + 1, slug(&group));
        println!("{name}");
        println!("  group: {group}");
        println!("  ops: {}", ops.len());
    }
    Ok(())
}

fn op_kinds(ops: &[OpsEntry]) -> Vec<String> {
    let values = ops
        .iter()
        .map(kind)
        .collect::<std::collections::BTreeSet<_>>();
    values.iter().map(|value| (*value).to_string()).collect()
}

fn op_paths(ops: &[OpsEntry]) -> Vec<String> {
    ops.iter()
        .map(path)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn kind(op: &OpsEntry) -> &'static str {
    match op {
        OpsEntry::CreateFile { .. } => "create_file",
        OpsEntry::ReplaceFile { .. } => "replace_file",
        OpsEntry::DeleteFile { .. } => "delete_file",
        OpsEntry::InsertBeforeAnchor { .. } => "insert_before_anchor",
        OpsEntry::InsertAfterAnchor { .. } => "insert_after_anchor",
        OpsEntry::InsertBeforeText { .. } => "insert_before_text",
        OpsEntry::InsertAfterText { .. } => "insert_after_text",
        OpsEntry::ReplaceText { .. } => "replace_text",
        OpsEntry::ReplaceBetweenAnchors { .. } => "replace_between_anchors",
        OpsEntry::ReplaceSymbol { .. } => "replace_symbol",
        OpsEntry::DeleteSymbol { .. } => "delete_symbol",
        OpsEntry::InsertBeforeSymbol { .. } => "insert_before_symbol",
        OpsEntry::InsertAfterSymbol { .. } => "insert_after_symbol",
        OpsEntry::ReplaceMethodBody { .. } => "replace_method_body",
        OpsEntry::ReplaceRange { .. } => "replace_range",
        OpsEntry::DeleteRange { .. } => "delete_range",
        OpsEntry::AppendToFile { .. } => "append_to_file",
        OpsEntry::CopyFile { .. } => "copy_file",
        OpsEntry::MoveFile { .. } => "move_file",
        OpsEntry::RenameFile { .. } => "rename_file",
        OpsEntry::CreateDir { .. } => "create_dir",
        OpsEntry::DeleteDir { .. } => "delete_dir",
    }
}

fn path(op: &OpsEntry) -> String {
    match op {
        OpsEntry::CreateFile { path, .. }
        | OpsEntry::ReplaceFile { path, .. }
        | OpsEntry::DeleteFile { path, .. }
        | OpsEntry::InsertBeforeText { path, .. }
        | OpsEntry::InsertAfterText { path, .. }
        | OpsEntry::ReplaceText { path, .. }
        | OpsEntry::InsertBeforeAnchor { path, .. }
        | OpsEntry::InsertAfterAnchor { path, .. }
        | OpsEntry::ReplaceBetweenAnchors { path, .. }
        | OpsEntry::ReplaceSymbol { path, .. }
        | OpsEntry::DeleteSymbol { path, .. }
        | OpsEntry::InsertBeforeSymbol { path, .. }
        | OpsEntry::InsertAfterSymbol { path, .. }
        | OpsEntry::ReplaceMethodBody { path, .. }
        | OpsEntry::ReplaceRange { path, .. }
        | OpsEntry::DeleteRange { path, .. }
        | OpsEntry::AppendToFile { path, .. }
        | OpsEntry::CreateDir { path, .. }
        | OpsEntry::DeleteDir { path, .. } => path.to_string_lossy().replace('\\', "/"),
        OpsEntry::CopyFile { from, to, .. }
        | OpsEntry::MoveFile { from, to, .. }
        | OpsEntry::RenameFile { from, to, .. } => format!(
            "{} -> {}",
            from.to_string_lossy().replace('\\', "/"),
            to.to_string_lossy().replace('\\', "/"),
        ),
    }
}

fn group_key(op: &OpsEntry, by: &str) -> String {
    if by == "risk" {
        if matches!(
            op,
            OpsEntry::ReplaceFile { .. } | OpsEntry::DeleteFile { .. }
        ) {
            return "high".to_string();
        }
        return "medium".to_string();
    }
    path(op).split('/').take(4).collect::<Vec<_>>().join("-")
}

fn title(value: &str) -> String {
    value
        .split(['-', '_', ' '])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn slug(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}
