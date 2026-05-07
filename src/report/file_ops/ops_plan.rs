use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow, bail};
use serde::Deserialize;

use crate::model::CodeMap;

#[derive(Debug, Deserialize)]
pub struct OpsPlan {
    pub version: u16,
    #[serde(default)]
    pub task: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    pub ops: Vec<OpsEntry>,
    #[serde(default)]
    pub verify: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind")]
pub enum OpsEntry {
    #[serde(rename = "create_file")]
    CreateFile {
        #[serde(default)]
        id: Option<String>,
        path: PathBuf,
        content: String,
    },
    #[serde(rename = "replace_file")]
    ReplaceFile {
        path: PathBuf,
        content: String,
        expected_hash: Option<String>,
        #[serde(default)]
        id: Option<String>,
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
        #[serde(default)]
        id: Option<String>,
    },
    #[serde(rename = "delete_range")]
    DeleteRange {
        path: PathBuf,
        start_line: usize,
        end_line: usize,
        expected_hash: Option<String>,
        context_before: Option<String>,
        context_after: Option<String>,
        #[serde(default)]
        id: Option<String>,
    },
    #[serde(rename = "append_to_file")]
    AppendToFile {
        #[serde(default)]
        id: Option<String>,
        path: PathBuf,
        content: String,
        expected_hash: Option<String>,
    },
    #[serde(rename = "insert_before_text")]
    InsertBeforeText {
        #[serde(default)]
        id: Option<String>,
        path: PathBuf,
        find: String,
        content: String,
    },
    #[serde(rename = "insert_after_text")]
    InsertAfterText {
        #[serde(default)]
        id: Option<String>,
        path: PathBuf,
        find: String,
        content: String,
    },
    #[serde(rename = "replace_text")]
    ReplaceText {
        #[serde(default)]
        id: Option<String>,
        path: PathBuf,
        find: String,
        replace: String,
    },
    #[serde(rename = "insert_before_anchor")]
    InsertBeforeAnchor {
        #[serde(default)]
        id: Option<String>,
        path: PathBuf,
        anchor: String,
        content: String,
    },
    #[serde(rename = "insert_after_anchor")]
    InsertAfterAnchor {
        #[serde(default)]
        id: Option<String>,
        path: PathBuf,
        anchor: String,
        content: String,
    },
    #[serde(rename = "replace_between_anchors")]
    ReplaceBetweenAnchors {
        path: PathBuf,
        start_anchor: String,
        end_anchor: String,
        content: String,
        expected_hash: Option<String>,
        #[serde(default)]
        id: Option<String>,
    },
    #[serde(rename = "delete_file")]
    DeleteFile {
        path: PathBuf,
        expected_hash: Option<String>,
        #[serde(default)]
        id: Option<String>,
    },
    #[serde(rename = "replace_symbol")]
    ReplaceSymbol {
        path: PathBuf,
        symbol: String,
        content: String,
        expected_hash: Option<String>,
        context_before: Option<String>,
        context_after: Option<String>,
        #[serde(default)]
        id: Option<String>,
    },
    #[serde(rename = "delete_symbol")]
    DeleteSymbol {
        path: PathBuf,
        symbol: String,
        expected_hash: Option<String>,
        #[serde(default)]
        id: Option<String>,
    },
    #[serde(rename = "insert_before_symbol")]
    InsertBeforeSymbol {
        path: PathBuf,
        symbol: String,
        content: String,
        expected_hash: Option<String>,
        #[serde(default)]
        id: Option<String>,
    },
    #[serde(rename = "insert_after_symbol")]
    InsertAfterSymbol {
        path: PathBuf,
        symbol: String,
        content: String,
        expected_hash: Option<String>,
        #[serde(default)]
        id: Option<String>,
    },
    #[serde(rename = "replace_method_body")]
    ReplaceMethodBody {
        path: PathBuf,
        symbol: String,
        content: String,
        expected_hash: Option<String>,
        #[serde(default)]
        id: Option<String>,
    },
}

pub fn print_ops_preview(
    root: &Path,
    from: Option<&Path>,
    yaml: Option<&str>,
    limit: usize,
) -> Result<()> {
    let plan = read_plan(from, yaml)?;
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

pub fn print_ops_check(
    root: &Path,
    map: Option<&CodeMap>,
    from: Option<&Path>,
    yaml: Option<&str>,
    strict: bool,
    limit: usize,
) -> Result<()> {
    let plan = read_plan(from, yaml)?;
    println!("ops-check: version {}", plan.version);
    if let Some(task) = &plan.task {
        println!("task: {task}");
    }

    let mut errors = Vec::new();
    for (index, op) in plan.ops.iter().take(limit).enumerate() {
        let description = describe_op(op);
        match validate_op(root, map, op, strict) {
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
            }
            Err(error) => {
                println!("  {}. {} applies: no", index + 1, description);
                println!("     risk: high");
                println!("     reason: {error}");
                errors.push(error.to_string());
            }
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
    map: &CodeMap,
    from: Option<&Path>,
    yaml: Option<&str>,
    write: bool,
    backup: bool,
    stop_on_error: bool,
    limit: usize,
) -> Result<()> {
    let plan = read_plan(from, yaml)?;
    if !write {
        println!("ops-apply: dry-run only; pass --write");
        return print_ops_check(root, Some(map), from, yaml, false, limit);
    }

    if backup {
        backup_plan_files(root, &plan)?;
    }
    let mut applied = 0usize;
    let mut failed = 0usize;
    for op in plan.ops.iter().take(limit) {
        if let Err(error) =
            validate_op(root, Some(map), op, false).and_then(|_| apply_op(root, map, op, write))
        {
            failed += 1;
            println!("failed {}: {error}", describe_op(op));
            if stop_on_error {
                break;
            }
        } else {
            applied += 1;
            println!("applied {}", describe_op(op));
        }
    }
    println!("ops-apply: applied={applied} failed={failed}");
    Ok(())
}

pub fn read_plan(from: Option<&Path>, yaml: Option<&str>) -> Result<OpsPlan> {
    let text = if let Some(yaml) = yaml {
        yaml.to_string()
    } else {
        let path = from.ok_or_else(|| {
            anyhow!("ops plan requires --from <plan.yml>, --from -, or --yaml <text>")
        })?;
        if path == Path::new("-") {
            let mut text = String::new();
            io::stdin().read_to_string(&mut text)?;
            text
        } else {
            fs::read_to_string(path)?
        }
    };
    Ok(serde_yaml::from_str(&text)?)
}

fn validate_op(root: &Path, map: Option<&CodeMap>, op: &OpsEntry, strict: bool) -> Result<()> {
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
        } => {
            validate_replace_file(root, path, expected_hash.as_deref())?;
        }
        OpsEntry::DeleteFile {
            path,
            expected_hash,
            ..
        }
        | OpsEntry::AppendToFile {
            path,
            expected_hash,
            ..
        } => {
            validate_existing_file(root, path, expected_hash.as_deref())?;
        }
        OpsEntry::InsertBeforeText { path, find, .. }
        | OpsEntry::InsertAfterText { path, find, .. }
        | OpsEntry::ReplaceText { path, find, .. } => {
            validate_text_locator(root, path, find, strict)?;
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
            ..
        } => {
            if strict
                && expected_hash.is_none()
                && context_before.is_none()
                && context_after.is_none()
            {
                bail!(
                    "strict mode requires expected_hash or context for range op {}",
                    path.display()
                );
            }
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
        OpsEntry::InsertBeforeAnchor { path, anchor, .. }
        | OpsEntry::InsertAfterAnchor { path, anchor, .. } => {
            let text = fs::read_to_string(root.join(path))?;
            if !text.contains(anchor) {
                bail!("anchor not found in {}: {}", path.display(), anchor);
            }
        }
        OpsEntry::ReplaceBetweenAnchors {
            path,
            start_anchor,
            end_anchor,
            expected_hash,
            ..
        } => {
            validate_existing_file(root, path, expected_hash.as_deref())?;
            let text = fs::read_to_string(root.join(path))?;
            anchor_inner_range(&text, path, start_anchor, end_anchor)?;
        }
        OpsEntry::ReplaceSymbol {
            path,
            expected_hash,
            ..
        }
        | OpsEntry::DeleteSymbol {
            path,
            expected_hash,
            ..
        }
        | OpsEntry::InsertBeforeSymbol {
            path,
            expected_hash,
            ..
        }
        | OpsEntry::InsertAfterSymbol {
            path,
            expected_hash,
            ..
        }
        | OpsEntry::ReplaceMethodBody {
            path,
            expected_hash,
            ..
        } => {
            if strict && expected_hash.is_none() {
                bail!(
                    "strict mode requires expected_hash for symbol op {}",
                    path.display()
                );
            }
            validate_existing_file(root, path, expected_hash.as_deref())?;
            if strict {
                validate_symbol_locator(map, path, symbol_name(op).unwrap_or_default())?;
            }
        }
    }
    Ok(())
}

fn validate_symbol_locator(map: Option<&CodeMap>, path: &Path, symbol: &str) -> Result<()> {
    let Some(map) = map else {
        bail!("strict mode requires codemap for symbol locator checks");
    };
    let path_text = path.to_string_lossy().replace('\\', "/");
    let Some(file) = map
        .files
        .iter()
        .find(|file| file.path.to_string_lossy().replace('\\', "/") == path_text)
    else {
        bail!("file not found in codemap for symbol locator: {path_text}");
    };
    let matches = map
        .symbols
        .iter()
        .filter(|entry| entry.file_id == file.id && entry.name == symbol)
        .count();
    match matches {
        1 => Ok(()),
        0 => bail!("symbol not found in {}: {}", path_text, symbol),
        _ => bail!("symbol is ambiguous in {}: {}", path_text, symbol),
    }
}

fn symbol_name(op: &OpsEntry) -> Option<&str> {
    match op {
        OpsEntry::ReplaceSymbol { symbol, .. }
        | OpsEntry::DeleteSymbol { symbol, .. }
        | OpsEntry::InsertBeforeSymbol { symbol, .. }
        | OpsEntry::InsertAfterSymbol { symbol, .. }
        | OpsEntry::ReplaceMethodBody { symbol, .. } => Some(symbol),
        _ => None,
    }
}

fn apply_op(root: &Path, map: &CodeMap, op: &OpsEntry, write: bool) -> Result<()> {
    match op {
        OpsEntry::CreateFile { path, content, .. } => {
            let full = root.join(path);
            if let Some(parent) = full.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(full, content)?;
        }
        OpsEntry::ReplaceFile { path, content, .. } => {
            let full = root.join(path);
            if let Some(parent) = full.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(full, content)?;
        }
        OpsEntry::AppendToFile { path, content, .. } => {
            let mut text = fs::read_to_string(root.join(path))?;
            if !text.ends_with('\n') {
                text.push('\n');
            }
            text.push_str(content.trim_end());
            text.push('\n');
            fs::write(root.join(path), text)?;
        }
        OpsEntry::InsertBeforeText {
            path,
            find,
            content,
            ..
        } => {
            let text = fs::read_to_string(root.join(path))?;
            let actual_find = text_locator_in_text(&text, find)?;
            let next = text.replacen(
                actual_find.as_ref(),
                &format!("{}\n{actual_find}", content.trim_end()),
                1,
            );
            fs::write(root.join(path), next)?;
        }
        OpsEntry::InsertAfterText {
            path,
            find,
            content,
            ..
        } => {
            let text = fs::read_to_string(root.join(path))?;
            let actual_find = text_locator_in_text(&text, find)?;
            let separator = if actual_find.ends_with('\n') {
                ""
            } else {
                "\n"
            };
            let next = text.replacen(
                actual_find.as_ref(),
                &format!("{actual_find}{separator}{}\n", content.trim_end()),
                1,
            );
            fs::write(root.join(path), next)?;
        }
        OpsEntry::ReplaceText {
            path,
            find,
            replace,
            ..
        } => {
            let text = fs::read_to_string(root.join(path))?;
            let actual_find = text_locator_in_text(&text, find)?;
            let next = text.replacen(actual_find.as_ref(), replace, 1);
            fs::write(root.join(path), next)?;
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
        OpsEntry::InsertBeforeAnchor {
            path,
            anchor,
            content,
            ..
        } => {
            let text = fs::read_to_string(root.join(path))?;
            let next = text.replacen(anchor, &format!("{content}\n{anchor}"), 1);
            fs::write(root.join(path), next)?;
        }
        OpsEntry::InsertAfterAnchor {
            path,
            anchor,
            content,
            ..
        } => {
            let text = fs::read_to_string(root.join(path))?;
            let next = text.replacen(anchor, &format!("{anchor}\n{content}"), 1);
            fs::write(root.join(path), next)?;
        }
        OpsEntry::ReplaceBetweenAnchors {
            path,
            start_anchor,
            end_anchor,
            content,
            ..
        } => {
            let text = fs::read_to_string(root.join(path))?;
            let (start_line, end_line) = anchor_inner_range(&text, path, start_anchor, end_anchor)?;
            let next = if start_line <= end_line {
                replace_range(&text, start_line, end_line, Some(content))
            } else {
                insert_after_line(&text, start_line.saturating_sub(1), content)
            };
            fs::write(root.join(path), next)?;
        }
        OpsEntry::DeleteFile { path, .. } => {
            fs::remove_file(root.join(path))?;
        }
        OpsEntry::ReplaceSymbol {
            path,
            symbol,
            content,
            ..
        } => {
            super::symbol_ops::replace_symbol(root, map, path, symbol, content, write)?;
        }
        OpsEntry::DeleteSymbol { path, symbol, .. } => {
            super::symbol_ops::delete_symbol(root, map, path, symbol, write)?;
        }
        OpsEntry::InsertBeforeSymbol {
            path,
            symbol,
            content,
            ..
        } => {
            super::symbol_ops::insert_before_symbol(root, map, path, symbol, content, write)?;
        }
        OpsEntry::InsertAfterSymbol {
            path,
            symbol,
            content,
            ..
        } => {
            super::symbol_ops::insert_after_symbol(root, map, path, symbol, content, write)?;
        }
        OpsEntry::ReplaceMethodBody {
            path,
            symbol,
            content,
            ..
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

fn validate_replace_file(root: &Path, path: &Path, expected_hash: Option<&str>) -> Result<()> {
    let full = root.join(path);
    if full.exists() {
        return validate_existing_file(root, path, expected_hash);
    }
    if expected_hash.is_some() {
        bail!(
            "replace_file target is missing but expected_hash was supplied: {}",
            path.display()
        );
    }
    Ok(())
}

fn validate_text_locator(root: &Path, path: &Path, find: &str, strict: bool) -> Result<()> {
    validate_existing_file(root, path, None)?;
    let text = fs::read_to_string(root.join(path))?;
    let actual_find = text_locator_in_text(&text, find)?;
    let matches = text.matches(actual_find.as_ref()).count();
    match matches {
        1 => Ok(()),
        0 => bail!("text locator not found in {}: {}", path.display(), find),
        _ if strict => bail!(
            "text locator is ambiguous in {}: {} matches for {}",
            path.display(),
            matches,
            find
        ),
        _ => Ok(()),
    }
}

fn text_locator_in_text<'a>(text: &str, find: &'a str) -> Result<std::borrow::Cow<'a, str>> {
    if text.contains(find) {
        return Ok(std::borrow::Cow::Borrowed(find));
    }
    if find.contains('\n') {
        let crlf = find.replace('\n', "\r\n");
        if text.contains(&crlf) {
            return Ok(std::borrow::Cow::Owned(crlf));
        }
    }
    bail!("text locator not found: {find}");
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
        OpsEntry::AppendToFile { path, .. } => format!("append_to_file {}", path.display()),
        OpsEntry::InsertBeforeText { path, .. } => {
            format!("insert_before_text {}", path.display())
        }
        OpsEntry::InsertAfterText { path, .. } => format!("insert_after_text {}", path.display()),
        OpsEntry::ReplaceText { path, .. } => format!("replace_text {}", path.display()),
        OpsEntry::InsertBeforeAnchor { path, anchor, .. } => {
            format!("insert_before_anchor {} anchor={}", path.display(), anchor)
        }
        OpsEntry::InsertAfterAnchor { path, anchor, .. } => {
            format!("insert_after_anchor {} anchor={}", path.display(), anchor)
        }
        OpsEntry::ReplaceBetweenAnchors {
            path,
            start_anchor,
            end_anchor,
            ..
        } => format!(
            "replace_between_anchors {} start_anchor={} end_anchor={}",
            path.display(),
            start_anchor,
            end_anchor
        ),
        OpsEntry::DeleteFile { path, .. } => format!("delete_file {}", path.display()),
        OpsEntry::ReplaceSymbol { path, symbol, .. } => {
            format!("replace_symbol {} symbol={}", path.display(), symbol)
        }
        OpsEntry::DeleteSymbol { path, symbol, .. } => {
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

fn op_id(op: &OpsEntry) -> Option<&str> {
    match op {
        OpsEntry::CreateFile { id, .. }
        | OpsEntry::ReplaceFile { id, .. }
        | OpsEntry::ReplaceRange { id, .. }
        | OpsEntry::DeleteRange { id, .. }
        | OpsEntry::AppendToFile { id, .. }
        | OpsEntry::InsertBeforeText { id, .. }
        | OpsEntry::InsertAfterText { id, .. }
        | OpsEntry::ReplaceText { id, .. }
        | OpsEntry::InsertBeforeAnchor { id, .. }
        | OpsEntry::InsertAfterAnchor { id, .. }
        | OpsEntry::ReplaceBetweenAnchors { id, .. }
        | OpsEntry::DeleteFile { id, .. }
        | OpsEntry::ReplaceSymbol { id, .. }
        | OpsEntry::DeleteSymbol { id, .. }
        | OpsEntry::InsertBeforeSymbol { id, .. }
        | OpsEntry::InsertAfterSymbol { id, .. }
        | OpsEntry::ReplaceMethodBody { id, .. } => id.as_deref(),
    }
}

fn op_kind(op: &OpsEntry) -> &'static str {
    match op {
        OpsEntry::CreateFile { .. } => "create_file",
        OpsEntry::ReplaceFile { .. } => "replace_file",
        OpsEntry::ReplaceRange { .. } => "replace_range",
        OpsEntry::DeleteRange { .. } => "delete_range",
        OpsEntry::AppendToFile { .. } => "append_to_file",
        OpsEntry::InsertBeforeText { .. } => "insert_before_text",
        OpsEntry::InsertAfterText { .. } => "insert_after_text",
        OpsEntry::ReplaceText { .. } => "replace_text",
        OpsEntry::InsertBeforeAnchor { .. } => "insert_before_anchor",
        OpsEntry::InsertAfterAnchor { .. } => "insert_after_anchor",
        OpsEntry::ReplaceBetweenAnchors { .. } => "replace_between_anchors",
        OpsEntry::DeleteFile { .. } => "delete_file",
        OpsEntry::ReplaceSymbol { .. } => "replace_symbol",
        OpsEntry::DeleteSymbol { .. } => "delete_symbol",
        OpsEntry::InsertBeforeSymbol { .. } => "insert_before_symbol",
        OpsEntry::InsertAfterSymbol { .. } => "insert_after_symbol",
        OpsEntry::ReplaceMethodBody { .. } => "replace_method_body",
    }
}

fn op_path(op: &OpsEntry) -> String {
    match op {
        OpsEntry::CreateFile { path, .. }
        | OpsEntry::ReplaceFile { path, .. }
        | OpsEntry::ReplaceRange { path, .. }
        | OpsEntry::DeleteRange { path, .. }
        | OpsEntry::AppendToFile { path, .. }
        | OpsEntry::InsertBeforeText { path, .. }
        | OpsEntry::InsertAfterText { path, .. }
        | OpsEntry::ReplaceText { path, .. }
        | OpsEntry::InsertBeforeAnchor { path, .. }
        | OpsEntry::InsertAfterAnchor { path, .. }
        | OpsEntry::ReplaceBetweenAnchors { path, .. }
        | OpsEntry::DeleteFile { path, .. }
        | OpsEntry::ReplaceSymbol { path, .. }
        | OpsEntry::DeleteSymbol { path, .. }
        | OpsEntry::InsertBeforeSymbol { path, .. }
        | OpsEntry::InsertAfterSymbol { path, .. }
        | OpsEntry::ReplaceMethodBody { path, .. } => path.display().to_string(),
    }
}

fn locator_kind(op: &OpsEntry) -> &'static str {
    match op {
        OpsEntry::CreateFile { .. }
        | OpsEntry::ReplaceFile { .. }
        | OpsEntry::DeleteFile { .. }
        | OpsEntry::AppendToFile { .. }
        | OpsEntry::InsertBeforeText { .. }
        | OpsEntry::InsertAfterText { .. }
        | OpsEntry::ReplaceText { .. } => "file",
        OpsEntry::ReplaceRange { .. } | OpsEntry::DeleteRange { .. } => "range",
        OpsEntry::InsertBeforeAnchor { .. }
        | OpsEntry::InsertAfterAnchor { .. }
        | OpsEntry::ReplaceBetweenAnchors { .. } => "anchor",
        OpsEntry::ReplaceSymbol { .. }
        | OpsEntry::DeleteSymbol { .. }
        | OpsEntry::InsertBeforeSymbol { .. }
        | OpsEntry::InsertAfterSymbol { .. }
        | OpsEntry::ReplaceMethodBody { .. } => "symbol",
    }
}

fn locator_confidence(op: &OpsEntry) -> &'static str {
    match op {
        OpsEntry::ReplaceRange {
            expected_hash,
            context_before,
            context_after,
            ..
        }
        | OpsEntry::DeleteRange {
            expected_hash,
            context_before,
            context_after,
            ..
        } if expected_hash.is_some() && (context_before.is_some() || context_after.is_some()) => {
            "high"
        }
        OpsEntry::ReplaceBetweenAnchors { expected_hash, .. } if expected_hash.is_some() => "high",
        OpsEntry::InsertBeforeText { .. }
        | OpsEntry::InsertAfterText { .. }
        | OpsEntry::ReplaceText { .. }
        | OpsEntry::InsertBeforeAnchor { .. }
        | OpsEntry::InsertAfterAnchor { .. }
        | OpsEntry::ReplaceBetweenAnchors { .. } => "high",
        OpsEntry::ReplaceSymbol { expected_hash, .. }
        | OpsEntry::DeleteSymbol { expected_hash, .. }
        | OpsEntry::InsertBeforeSymbol { expected_hash, .. }
        | OpsEntry::InsertAfterSymbol { expected_hash, .. }
        | OpsEntry::ReplaceMethodBody { expected_hash, .. }
            if expected_hash.is_some() =>
        {
            "high"
        }
        OpsEntry::ReplaceSymbol { .. }
        | OpsEntry::DeleteSymbol { .. }
        | OpsEntry::InsertBeforeSymbol { .. }
        | OpsEntry::InsertAfterSymbol { .. }
        | OpsEntry::ReplaceMethodBody { .. } => "medium",
        _ => "medium",
    }
}

fn safety_line(op: &OpsEntry, strict: bool) -> String {
    format!(
        "hash_match={} context_match={} unique_match={} line_drift={} strict={}",
        if op_has_expected_hash(op) {
            "yes"
        } else {
            "skipped"
        },
        if op_has_context(op) { "yes" } else { "skipped" },
        if matches!(locator_kind(op), "symbol" | "anchor") {
            "yes"
        } else {
            "skipped"
        },
        "not-evaluated",
        if strict { "yes" } else { "no" }
    )
}

fn op_has_expected_hash(op: &OpsEntry) -> bool {
    match op {
        OpsEntry::ReplaceFile { expected_hash, .. }
        | OpsEntry::ReplaceRange { expected_hash, .. }
        | OpsEntry::DeleteRange { expected_hash, .. }
        | OpsEntry::AppendToFile { expected_hash, .. }
        | OpsEntry::ReplaceBetweenAnchors { expected_hash, .. }
        | OpsEntry::DeleteFile { expected_hash, .. }
        | OpsEntry::ReplaceSymbol { expected_hash, .. }
        | OpsEntry::DeleteSymbol { expected_hash, .. }
        | OpsEntry::InsertBeforeSymbol { expected_hash, .. }
        | OpsEntry::InsertAfterSymbol { expected_hash, .. }
        | OpsEntry::ReplaceMethodBody { expected_hash, .. } => expected_hash.is_some(),
        _ => false,
    }
}

fn op_has_context(op: &OpsEntry) -> bool {
    match op {
        OpsEntry::ReplaceRange {
            context_before,
            context_after,
            ..
        }
        | OpsEntry::DeleteRange {
            context_before,
            context_after,
            ..
        }
        | OpsEntry::ReplaceSymbol {
            context_before,
            context_after,
            ..
        } => context_before.is_some() || context_after.is_some(),
        _ => false,
    }
}

fn risk_for_op(op: &OpsEntry) -> &'static str {
    match op {
        OpsEntry::CreateFile { .. }
        | OpsEntry::ReplaceFile {
            expected_hash: None,
            ..
        }
        | OpsEntry::InsertBeforeText { .. }
        | OpsEntry::InsertAfterText { .. }
        | OpsEntry::ReplaceText { .. }
        | OpsEntry::InsertBeforeAnchor { .. }
        | OpsEntry::InsertAfterAnchor { .. }
        | OpsEntry::AppendToFile { .. } => "low",
        OpsEntry::ReplaceBetweenAnchors { expected_hash, .. } if expected_hash.is_some() => "low",
        OpsEntry::ReplaceRange {
            expected_hash,
            context_before,
            context_after,
            ..
        }
        | OpsEntry::DeleteRange {
            expected_hash,
            context_before,
            context_after,
            ..
        } if expected_hash.is_some() && (context_before.is_some() || context_after.is_some()) => {
            "low"
        }
        OpsEntry::ReplaceSymbol { .. }
        | OpsEntry::DeleteSymbol { .. }
        | OpsEntry::InsertBeforeSymbol { .. }
        | OpsEntry::InsertAfterSymbol { .. }
        | OpsEntry::ReplaceMethodBody { .. } => "medium",
        OpsEntry::ReplaceFile { expected_hash, .. }
        | OpsEntry::DeleteFile { expected_hash, .. }
            if expected_hash.is_some() =>
        {
            "medium"
        }
        _ => "medium",
    }
}

fn safety_reason_for_op(op: &OpsEntry) -> &'static str {
    match op {
        OpsEntry::CreateFile { .. } => "target path is unused",
        OpsEntry::ReplaceFile { expected_hash, .. } if expected_hash.is_some() => {
            "file exists and expected_hash matched"
        }
        OpsEntry::ReplaceFile { .. } => {
            "file may be created or replaced; no expected_hash supplied"
        }
        OpsEntry::ReplaceRange {
            expected_hash,
            context_before,
            context_after,
            ..
        }
        | OpsEntry::DeleteRange {
            expected_hash,
            context_before,
            context_after,
            ..
        } if expected_hash.is_some() && (context_before.is_some() || context_after.is_some()) => {
            "range is valid; expected_hash and context matched"
        }
        OpsEntry::ReplaceRange { .. } | OpsEntry::DeleteRange { .. } => {
            "range is valid; add expected_hash/context for stronger drift protection"
        }
        OpsEntry::AppendToFile { expected_hash, .. } if expected_hash.is_some() => {
            "file exists and expected_hash matched"
        }
        OpsEntry::AppendToFile { .. } => "file exists; append has no expected_hash",
        OpsEntry::InsertBeforeText { .. } => "text locator found",
        OpsEntry::InsertAfterText { .. } => "text locator found",
        OpsEntry::ReplaceText { .. } => "text locator found",
        OpsEntry::InsertBeforeAnchor { .. } | OpsEntry::InsertAfterAnchor { .. } => "anchor found",
        OpsEntry::ReplaceBetweenAnchors { expected_hash, .. } if expected_hash.is_some() => {
            "start/end anchors found in order and expected_hash matched"
        }
        OpsEntry::ReplaceBetweenAnchors { .. } => {
            "start/end anchors found in order; add expected_hash for stronger drift protection"
        }
        OpsEntry::DeleteFile { expected_hash, .. } if expected_hash.is_some() => {
            "file exists and expected_hash matched"
        }
        OpsEntry::DeleteFile { .. } => "file exists; no expected_hash supplied",
        OpsEntry::ReplaceSymbol { .. }
        | OpsEntry::DeleteSymbol { .. }
        | OpsEntry::InsertBeforeSymbol { .. }
        | OpsEntry::InsertAfterSymbol { .. }
        | OpsEntry::ReplaceMethodBody { .. } => {
            "file exists; symbol range is resolved during ops-apply"
        }
    }
}

fn anchor_inner_range(
    text: &str,
    path: &Path,
    start_anchor: &str,
    end_anchor: &str,
) -> Result<(usize, usize)> {
    let mut start_line = None;
    let mut end_line = None;

    for (index, line) in text.lines().enumerate() {
        let line_number = index + 1;
        if start_line.is_none() && line.contains(start_anchor) {
            start_line = Some(line_number);
            continue;
        }

        if start_line.is_some() && line.contains(end_anchor) {
            end_line = Some(line_number);
            break;
        }
    }

    let Some(start_line) = start_line else {
        bail!(
            "start_anchor not found in {}: {}",
            path.display(),
            start_anchor
        );
    };
    let Some(end_line) = end_line else {
        bail!("end_anchor not found in {}: {}", path.display(), end_anchor);
    };
    if end_line <= start_line {
        bail!(
            "anchors are not in replaceable order in {}: {} -> {}",
            path.display(),
            start_anchor,
            end_anchor
        );
    }

    Ok((start_line + 1, end_line.saturating_sub(1)))
}

fn insert_after_line(text: &str, line: usize, content: &str) -> String {
    let mut result = String::new();
    for (index, existing) in text.lines().enumerate() {
        let line_no = index + 1;
        result.push_str(existing);
        result.push('\n');
        if line_no == line {
            result.push_str(content.trim_end());
            result.push('\n');
        }
    }
    result
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
        let path = PathBuf::from(op_path(op));
        let source = root.join(&path);
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

fn short_hash(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")[..8].to_string()
}
