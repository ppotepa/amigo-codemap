use std::fs;
use std::path::Path;

use anyhow::{Result, anyhow, bail};

use crate::model::CodeMap;
use crate::report::file_ops::symbol_locator::SymbolFilters;

use super::super::OpsPlan;
use super::super::content::op_content;
use super::super::model::OpsEntry;
use super::super::path_safety::repo_path;
use super::super::validate::{text_locator_in_text, text_scope_for_symbol};

pub(super) fn apply_op(
    root: &Path,
    plan: &OpsPlan,
    map: Option<&CodeMap>,
    op: &OpsEntry,
    write: bool,
) -> Result<()> {
    match op {
        OpsEntry::CreateFile {
            path,
            content,
            content_from,
            ..
        }
        | OpsEntry::ReplaceFile {
            path,
            content,
            content_from,
            ..
        } => {
            let content = op_content(root, plan, content.as_deref(), content_from.as_deref())?;
            let full = repo_path(root, path)?;
            if let Some(parent) = full.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(full, content)?;
        }
        OpsEntry::AppendToFile {
            path,
            content,
            content_from,
            ..
        } => {
            let content = op_content(root, plan, content.as_deref(), content_from.as_deref())?;
            let mut text = fs::read_to_string(repo_path(root, path)?)?;
            if !text.ends_with('\n') {
                text.push('\n');
            }
            text.push_str(content.trim_end());
            text.push('\n');
            fs::write(repo_path(root, path)?, text)?;
        }
        OpsEntry::InsertBeforeText {
            path,
            find,
            content,
            content_from,
            ..
        } => {
            let content = op_content(root, plan, content.as_deref(), content_from.as_deref())?;
            let text = fs::read_to_string(repo_path(root, path)?)?;
            let actual_find = text_locator_in_text(&text, find)?;
            let next = text.replacen(
                actual_find.as_ref(),
                &format!("{}\n{actual_find}", content.trim_end()),
                1,
            );
            fs::write(repo_path(root, path)?, next)?;
        }
        OpsEntry::InsertAfterText {
            path,
            find,
            content,
            content_from,
            ..
        } => {
            let content = op_content(root, plan, content.as_deref(), content_from.as_deref())?;
            let text = fs::read_to_string(repo_path(root, path)?)?;
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
            fs::write(repo_path(root, path)?, next)?;
        }
        OpsEntry::ReplaceText {
            path,
            find,
            replace,
            content_from,
            within_symbol,
            expected_matches,
            ..
        } => {
            let replace = op_content(root, plan, replace.as_deref(), content_from.as_deref())?;
            let text = fs::read_to_string(repo_path(root, path)?)?;
            let actual_find = text_locator_in_text(&text, find)?;
            let next = replace_text_in_scope(
                map,
                path,
                &text,
                actual_find.as_ref(),
                &replace,
                within_symbol.as_deref(),
                *expected_matches,
            )?;
            fs::write(repo_path(root, path)?, next)?;
        }
        OpsEntry::ReplaceRange {
            path,
            start_line,
            end_line,
            content,
            content_from,
            ..
        } => {
            let content = op_content(root, plan, content.as_deref(), content_from.as_deref())?;
            let text = fs::read_to_string(repo_path(root, path)?)?;
            let next = replace_range(&text, *start_line, *end_line, Some(&content));
            fs::write(repo_path(root, path)?, next)?;
        }
        OpsEntry::DeleteRange {
            path,
            start_line,
            end_line,
            ..
        } => {
            let text = fs::read_to_string(repo_path(root, path)?)?;
            let next = replace_range(&text, *start_line, *end_line, None);
            fs::write(repo_path(root, path)?, next)?;
        }
        OpsEntry::InsertBeforeAnchor {
            path,
            anchor,
            content,
            content_from,
            ..
        } => {
            let content = op_content(root, plan, content.as_deref(), content_from.as_deref())?;
            let text = fs::read_to_string(repo_path(root, path)?)?;
            let next = text.replacen(anchor, &format!("{content}\n{anchor}"), 1);
            fs::write(repo_path(root, path)?, next)?;
        }
        OpsEntry::InsertAfterAnchor {
            path,
            anchor,
            content,
            content_from,
            ..
        } => {
            let content = op_content(root, plan, content.as_deref(), content_from.as_deref())?;
            let text = fs::read_to_string(repo_path(root, path)?)?;
            let next = text.replacen(anchor, &format!("{anchor}\n{content}"), 1);
            fs::write(repo_path(root, path)?, next)?;
        }
        OpsEntry::ReplaceBetweenAnchors {
            path,
            start_anchor,
            end_anchor,
            content,
            content_from,
            ..
        } => {
            let content = op_content(root, plan, content.as_deref(), content_from.as_deref())?;
            let text = fs::read_to_string(repo_path(root, path)?)?;
            let (start_line, end_line) = anchor_inner_range(&text, path, start_anchor, end_anchor)?;
            let next = if start_line <= end_line {
                replace_range(&text, start_line, end_line, Some(&content))
            } else {
                insert_after_line(&text, start_line.saturating_sub(1), &content)
            };
            fs::write(repo_path(root, path)?, next)?;
        }
        OpsEntry::DeleteFile { path, .. } => {
            fs::remove_file(repo_path(root, path)?)?;
        }
        OpsEntry::CopyFile {
            from,
            to,
            overwrite,
            ..
        } => {
            copy_file_op(root, from, to, *overwrite)?;
        }
        OpsEntry::MoveFile {
            from,
            to,
            overwrite,
            ..
        }
        | OpsEntry::RenameFile {
            from,
            to,
            overwrite,
            ..
        } => {
            copy_file_op(root, from, to, *overwrite)?;
            fs::remove_file(repo_path(root, from)?)?;
        }
        OpsEntry::CreateDir { path, .. } => {
            fs::create_dir_all(repo_path(root, path)?)?;
        }
        OpsEntry::DeleteDir {
            path, recursive, ..
        } => {
            let target = repo_path(root, path)?;
            if *recursive {
                fs::remove_dir_all(target)?;
            } else {
                fs::remove_dir(target)?;
            }
        }
        OpsEntry::ReplaceSymbol {
            path,
            symbol,
            content,
            content_from,
            ..
        } => {
            let map = map.ok_or_else(|| anyhow!("symbol operation requires codemap"))?;
            let content = op_content(root, plan, content.as_deref(), content_from.as_deref())?;
            super::super::super::symbol_ops::replace_symbol(
                root,
                map,
                path,
                symbol,
                SymbolFilters::default(),
                &content,
                write,
            )?;
        }
        OpsEntry::DeleteSymbol { path, symbol, .. } => {
            let map = map.ok_or_else(|| anyhow!("symbol operation requires codemap"))?;
            super::super::super::symbol_ops::delete_symbol(
                root,
                map,
                path,
                symbol,
                SymbolFilters::default(),
                write,
            )?;
        }
        OpsEntry::DeleteSymbolIfExists { path, symbol, .. } => {
            let map = map.ok_or_else(|| anyhow!("symbol operation requires codemap"))?;
            match super::super::super::symbol_ops::delete_symbol(
                root,
                map,
                path,
                symbol,
                SymbolFilters::default(),
                write,
            ) {
                Ok(()) => {}
                Err(error) if error.to_string().contains("symbol not found") => {}
                Err(error) => return Err(error),
            }
        }
        OpsEntry::InsertBeforeSymbol {
            path,
            symbol,
            content,
            content_from,
            ..
        } => {
            let map = map.ok_or_else(|| anyhow!("symbol operation requires codemap"))?;
            let content = op_content(root, plan, content.as_deref(), content_from.as_deref())?;
            super::super::super::symbol_ops::insert_before_symbol(
                root,
                map,
                path,
                symbol,
                SymbolFilters::default(),
                &content,
                write,
            )?;
        }
        OpsEntry::InsertAfterSymbol {
            path,
            symbol,
            content,
            content_from,
            ..
        } => {
            let map = map.ok_or_else(|| anyhow!("symbol operation requires codemap"))?;
            let content = op_content(root, plan, content.as_deref(), content_from.as_deref())?;
            super::super::super::symbol_ops::insert_after_symbol(
                root,
                map,
                path,
                symbol,
                SymbolFilters::default(),
                &content,
                write,
            )?;
        }
        OpsEntry::ReplaceMethodBody {
            path,
            symbol,
            content,
            content_from,
            ..
        } => {
            let map = map.ok_or_else(|| anyhow!("symbol operation requires codemap"))?;
            let content = op_content(root, plan, content.as_deref(), content_from.as_deref())?;
            super::super::super::symbol_ops::replace_method_body(
                root,
                map,
                path,
                symbol,
                SymbolFilters::default(),
                &content,
                write,
            )?;
        }
        OpsEntry::AssertSymbolAbsent { .. } | OpsEntry::AssertTextAbsent { .. } => {}
        OpsEntry::ReplaceFieldAccess {
            path,
            find,
            replace,
            scope,
            expected_matches,
            ..
        } => {
            let map = map.ok_or_else(|| anyhow!("field access operation requires codemap"))?;
            let path = path.as_ref().map(|path| path.as_path());
            super::super::super::symbol_ops::replace_field_access(
                root,
                map,
                path,
                find,
                replace,
                scope.as_deref(),
                *expected_matches,
                write,
            )?;
        }
    }
    Ok(())
}

fn copy_file_op(root: &Path, from: &Path, to: &Path, overwrite: bool) -> Result<()> {
    let source = repo_path(root, from)?;
    let target = repo_path(root, to)?;
    if target.exists() && !overwrite {
        bail!("target already exists: {}", to.display());
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source, target)?;
    Ok(())
}

fn replace_text_in_scope(
    map: Option<&CodeMap>,
    path: &Path,
    full_text: &str,
    find: &str,
    replace: &str,
    within_symbol: Option<&str>,
    expected_matches: Option<usize>,
) -> Result<String> {
    let scope = text_scope_for_symbol(map, path, within_symbol, full_text)?;
    let matches = scope.text.matches(find).count();

    if let Some(expected) = expected_matches {
        if expected == 0 {
            bail!("replace_text expected_matches must be at least 1");
        }
        if matches != expected {
            bail!(
                "replace_text expected {} matches in {}{}, got {}",
                expected,
                path.display(),
                within_symbol
                    .map(|symbol| format!(" within symbol `{symbol}`"))
                    .unwrap_or_default(),
                matches
            );
        }
    }

    if matches == 0 {
        bail!(
            "replace_text locator not found in scope for {}",
            path.display()
        );
    }

    let replacement_count = expected_matches.unwrap_or(1);
    let replaced_scope = scope.text.replacen(find, replace, replacement_count);
    let mut output = String::with_capacity(full_text.len() + replaced_scope.len());
    output.push_str(&full_text[..scope.start_byte]);
    output.push_str(&replaced_scope);
    output.push_str(&full_text[scope.end_byte..]);
    Ok(output)
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
