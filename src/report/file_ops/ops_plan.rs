use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow, bail};
use serde::Deserialize;

use crate::model::CodeMap;

#[derive(Debug, Deserialize)]
pub struct OpsPlan {
    pub version: u16,
    pub ops: Vec<OpsEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind")]
pub enum OpsEntry {
    #[serde(rename = "create_file")]
    CreateFile { path: PathBuf, content: String },
    #[serde(rename = "replace_file")]
    ReplaceFile {
        path: PathBuf,
        content: String,
        expected_hash: Option<String>,
    },
    #[serde(rename = "replace_range")]
    ReplaceRange {
        path: PathBuf,
        start_line: usize,
        end_line: usize,
        content: String,
        expected_hash: Option<String>,
        context_before: Option<String>,
        context_after: Option<String>,
    },
    #[serde(rename = "delete_range")]
    DeleteRange {
        path: PathBuf,
        start_line: usize,
        end_line: usize,
        expected_hash: Option<String>,
        context_before: Option<String>,
        context_after: Option<String>,
    },
    #[serde(rename = "insert_after_anchor")]
    InsertAfterAnchor {
        path: PathBuf,
        anchor: String,
        content: String,
    },
    #[serde(rename = "delete_file")]
    DeleteFile {
        path: PathBuf,
        expected_hash: Option<String>,
    },
    #[serde(rename = "replace_symbol")]
    ReplaceSymbol {
        path: PathBuf,
        symbol: String,
        content: String,
    },
    #[serde(rename = "delete_symbol")]
    DeleteSymbol { path: PathBuf, symbol: String },
    #[serde(rename = "insert_before_symbol")]
    InsertBeforeSymbol {
        path: PathBuf,
        symbol: String,
        content: String,
    },
    #[serde(rename = "insert_after_symbol")]
    InsertAfterSymbol {
        path: PathBuf,
        symbol: String,
        content: String,
    },
    #[serde(rename = "replace_method_body")]
    ReplaceMethodBody {
        path: PathBuf,
        symbol: String,
        content: String,
    },
}

pub fn print_ops_preview(root: &Path, from: Option<&Path>, limit: usize) -> Result<()> {
    let plan = read_plan(from)?;
    println!("ops-preview: version {}", plan.version);
    for (index, op) in plan.ops.iter().take(limit).enumerate() {
        println!("  {}. {}", index + 1, describe_op(op));
    }
    println!("next:");
    println!("  1. ops-check --from <plan.yml>");
    println!("  2. ops-apply --from <plan.yml> --write");
    let _ = root;
    Ok(())
}

pub fn print_ops_check(root: &Path, from: Option<&Path>, limit: usize) -> Result<()> {
    let plan = read_plan(from)?;
    let mut errors = Vec::new();
    for op in plan.ops.iter().take(limit) {
        if let Err(error) = validate_op(root, op) {
            errors.push(error.to_string());
        }
    }
    if errors.is_empty() {
        println!("ops-check: ok");
    } else {
        println!("ops-check: failed");
        for error in errors {
            println!("  {error}");
        }
    }
    Ok(())
}

pub fn print_ops_apply(
    root: &Path,
    map: &CodeMap,
    from: Option<&Path>,
    write: bool,
    limit: usize,
) -> Result<()> {
    let plan = read_plan(from)?;
    if !write {
        println!("ops-apply: dry-run only; pass --write");
        return print_ops_check(root, from, limit);
    }

    for op in plan.ops.iter().take(limit) {
        validate_op(root, op)?;
        apply_op(root, map, op, write)?;
        println!("applied {}", describe_op(op));
    }
    Ok(())
}

fn read_plan(from: Option<&Path>) -> Result<OpsPlan> {
    let path = from.ok_or_else(|| anyhow!("ops plan requires --from <plan.yml>"))?;
    let text = fs::read_to_string(path)?;
    Ok(serde_yaml::from_str(&text)?)
}

fn validate_op(root: &Path, op: &OpsEntry) -> Result<()> {
    match op {
        OpsEntry::CreateFile { path, .. } => {
            let full = root.join(path);
            if full.exists() {
                bail!("create_file target already exists: {}", path.display());
            }
        }
        OpsEntry::ReplaceFile {
            path,
            expected_hash,
            ..
        }
        | OpsEntry::DeleteFile {
            path,
            expected_hash,
            ..
        } => {
            validate_existing_file(root, path, expected_hash.as_deref())?;
        }
        OpsEntry::ReplaceRange {
            path,
            start_line,
            end_line,
            expected_hash,
            context_before,
            context_after,
            ..
        }
        | OpsEntry::DeleteRange {
            path,
            start_line,
            end_line,
            expected_hash,
            context_before,
            context_after,
        } => {
            validate_existing_file(root, path, expected_hash.as_deref())?;
            validate_range(
                root,
                path,
                *start_line,
                *end_line,
                context_before.as_deref(),
                context_after.as_deref(),
            )?;
        }
        OpsEntry::InsertAfterAnchor { path, anchor, .. } => {
            let text = fs::read_to_string(root.join(path))?;
            if !text.contains(anchor) {
                bail!("anchor not found in {}: {}", path.display(), anchor);
            }
        }
        OpsEntry::ReplaceSymbol { path, .. }
        | OpsEntry::DeleteSymbol { path, .. }
        | OpsEntry::InsertBeforeSymbol { path, .. }
        | OpsEntry::InsertAfterSymbol { path, .. }
        | OpsEntry::ReplaceMethodBody { path, .. } => {
            validate_existing_file(root, path, None)?;
        }
    }
    Ok(())
}

fn apply_op(root: &Path, map: &CodeMap, op: &OpsEntry, write: bool) -> Result<()> {
    match op {
        OpsEntry::CreateFile { path, content } => {
            let full = root.join(path);
            if let Some(parent) = full.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(full, content)?;
        }
        OpsEntry::ReplaceFile { path, content, .. } => {
            fs::write(root.join(path), content)?;
        }
        OpsEntry::ReplaceRange {
            path,
            start_line,
            end_line,
            content,
            ..
        } => {
            let text = fs::read_to_string(root.join(path))?;
            let next = replace_range(&text, *start_line, *end_line, Some(content));
            fs::write(root.join(path), next)?;
        }
        OpsEntry::DeleteRange {
            path,
            start_line,
            end_line,
            ..
        } => {
            let text = fs::read_to_string(root.join(path))?;
            let next = replace_range(&text, *start_line, *end_line, None);
            fs::write(root.join(path), next)?;
        }
        OpsEntry::InsertAfterAnchor {
            path,
            anchor,
            content,
        } => {
            let text = fs::read_to_string(root.join(path))?;
            let next = text.replacen(anchor, &format!("{anchor}\n{content}"), 1);
            fs::write(root.join(path), next)?;
        }
        OpsEntry::DeleteFile { path, .. } => {
            fs::remove_file(root.join(path))?;
        }
        OpsEntry::ReplaceSymbol {
            path,
            symbol,
            content,
        } => {
            super::symbol_ops::replace_symbol(root, map, path, symbol, content, write)?;
        }
        OpsEntry::DeleteSymbol { path, symbol } => {
            super::symbol_ops::delete_symbol(root, map, path, symbol, write)?;
        }
        OpsEntry::InsertBeforeSymbol {
            path,
            symbol,
            content,
        } => {
            super::symbol_ops::insert_before_symbol(root, map, path, symbol, content, write)?;
        }
        OpsEntry::InsertAfterSymbol {
            path,
            symbol,
            content,
        } => {
            super::symbol_ops::insert_after_symbol(root, map, path, symbol, content, write)?;
        }
        OpsEntry::ReplaceMethodBody {
            path,
            symbol,
            content,
        } => {
            super::symbol_ops::replace_method_body(root, map, path, symbol, content, write)?;
        }
    }
    Ok(())
}

fn validate_existing_file(root: &Path, path: &Path, expected_hash: Option<&str>) -> Result<()> {
    let full = root.join(path);
    if !full.exists() {
        bail!("file does not exist: {}", path.display());
    }
    if let Some(expected_hash) = expected_hash {
        let bytes = fs::read(&full)?;
        let actual = short_hash(&bytes);
        if actual != expected_hash {
            bail!(
                "hash mismatch for {}: expected {}, got {}",
                path.display(),
                expected_hash,
                actual
            );
        }
    }
    Ok(())
}

fn validate_range(
    root: &Path,
    path: &Path,
    start_line: usize,
    end_line: usize,
    context_before: Option<&str>,
    context_after: Option<&str>,
) -> Result<()> {
    if start_line == 0 || end_line < start_line {
        bail!(
            "invalid range {}:{}-{}",
            path.display(),
            start_line,
            end_line
        );
    }
    let text = fs::read_to_string(root.join(path))?;
    let lines = text.lines().collect::<Vec<_>>();
    if end_line > lines.len() {
        bail!(
            "range outside file {}:{}-{}",
            path.display(),
            start_line,
            end_line
        );
    }
    if let Some(context) = context_before {
        let before = lines[..start_line.saturating_sub(1)].join("\n");
        if !before.contains(context) {
            bail!("context_before not found for {}", path.display());
        }
    }
    if let Some(context) = context_after {
        let after = lines[end_line..].join("\n");
        if !after.contains(context) {
            bail!("context_after not found for {}", path.display());
        }
    }
    Ok(())
}

fn replace_range(
    text: &str,
    start_line: usize,
    end_line: usize,
    replacement: Option<&str>,
) -> String {
    let mut result = String::new();
    for (index, line) in text.lines().enumerate() {
        let line_no = index + 1;
        if line_no == start_line
            && let Some(replacement) = replacement
        {
            result.push_str(replacement.trim_end());
            result.push('\n');
        }
        if line_no < start_line || line_no > end_line {
            result.push_str(line);
            result.push('\n');
        }
    }
    result
}

fn describe_op(op: &OpsEntry) -> String {
    match op {
        OpsEntry::CreateFile { path, .. } => format!("create_file {}", path.display()),
        OpsEntry::ReplaceFile { path, .. } => format!("replace_file {}", path.display()),
        OpsEntry::ReplaceRange {
            path,
            start_line,
            end_line,
            ..
        } => format!(
            "replace_range {}:{}-{}",
            path.display(),
            start_line,
            end_line
        ),
        OpsEntry::DeleteRange {
            path,
            start_line,
            end_line,
            ..
        } => format!(
            "delete_range {}:{}-{}",
            path.display(),
            start_line,
            end_line
        ),
        OpsEntry::InsertAfterAnchor { path, anchor, .. } => {
            format!("insert_after_anchor {} anchor={}", path.display(), anchor)
        }
        OpsEntry::DeleteFile { path, .. } => format!("delete_file {}", path.display()),
        OpsEntry::ReplaceSymbol { path, symbol, .. } => {
            format!("replace_symbol {} symbol={}", path.display(), symbol)
        }
        OpsEntry::DeleteSymbol { path, symbol } => {
            format!("delete_symbol {} symbol={}", path.display(), symbol)
        }
        OpsEntry::InsertBeforeSymbol { path, symbol, .. } => {
            format!("insert_before_symbol {} symbol={}", path.display(), symbol)
        }
        OpsEntry::InsertAfterSymbol { path, symbol, .. } => {
            format!("insert_after_symbol {} symbol={}", path.display(), symbol)
        }
        OpsEntry::ReplaceMethodBody { path, symbol, .. } => {
            format!("replace_method_body {} symbol={}", path.display(), symbol)
        }
    }
}

fn short_hash(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")[..8].to_string()
}
