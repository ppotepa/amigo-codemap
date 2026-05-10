use std::fs;
use std::path::Path;

use anyhow::{Result, bail};

use crate::model::CodeMap;

use super::content::{op_content_from_paths, validate_content_sources};
use super::describe::op_paths;
use super::hash::{short_hash, short_sha256_hash};
use super::model::OpsEntry;
use super::path_safety::{path_key, repo_path, validate_repo_relative_path};

pub(super) fn validate_op_for_check(
    root: &Path,
    plan: &super::OpsPlan,
    map: Option<&CodeMap>,
    op: &OpsEntry,
    strict: bool,
    virtual_existing: &std::collections::BTreeSet<String>,
) -> Result<()> {
    match op {
        OpsEntry::CopyFile {
            from,
            to,
            expected_hash,
            overwrite,
            ..
        }
        | OpsEntry::MoveFile {
            from,
            to,
            expected_hash,
            overwrite,
            ..
        }
        | OpsEntry::RenameFile {
            from,
            to,
            expected_hash,
            overwrite,
            ..
        } => {
            validate_op_paths(op)?;
            if repo_path(root, from)?.exists() {
                validate_existing_file(root, from, expected_hash.as_deref())?;
            } else if expected_hash.is_some() || !virtual_existing.contains(&path_key(from)) {
                bail!("file does not exist: {}", from.display());
            }
            let target = repo_path(root, to)?;
            if target.exists() && !overwrite {
                bail!("target already exists: {}", to.display());
            }
            Ok(())
        }
        _ => validate_op(root, plan, map, op, strict),
    }
}

pub(super) fn validate_op(
    root: &Path,
    plan: &super::OpsPlan,
    map: Option<&CodeMap>,
    op: &OpsEntry,
    strict: bool,
) -> Result<()> {
    validate_op_paths(op)?;
    validate_content_sources(root, plan, op)?;
    match op {
        OpsEntry::CreateFile {
            path, overwrite, ..
        } => {
            let full = repo_path(root, path)?;
            if full.exists() && !overwrite {
                bail!("create_file target already exists: {}", path.display());
            }
            if full.is_dir() {
                bail!("create_file target is a directory: {}", path.display());
            }
        }
        OpsEntry::ReplaceFile {
            path,
            overwrite,
            expected_hash,
            ..
        } => {
            let _ = overwrite;
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
        OpsEntry::CopyFile {
            from,
            to,
            expected_hash,
            overwrite,
            ..
        }
        | OpsEntry::MoveFile {
            from,
            to,
            expected_hash,
            overwrite,
            ..
        }
        | OpsEntry::RenameFile {
            from,
            to,
            expected_hash,
            overwrite,
            ..
        } => {
            validate_existing_file(root, from, expected_hash.as_deref())?;
            let target = repo_path(root, to)?;
            if target.exists() && !overwrite {
                bail!("target already exists: {}", to.display());
            }
        }
        OpsEntry::CreateDir { path, .. } => {
            let target = repo_path(root, path)?;
            if target.exists() && !target.is_dir() {
                bail!(
                    "create_dir target exists but is not a directory: {}",
                    path.display()
                );
            }
        }
        OpsEntry::DeleteDir {
            path, recursive, ..
        } => {
            let target = repo_path(root, path)?;
            if !target.exists() {
                bail!("directory does not exist: {}", path.display());
            }
            if !target.is_dir() {
                bail!("delete_dir target is not a directory: {}", path.display());
            }
            if !recursive && target.read_dir()?.next().is_some() {
                bail!("delete_dir target is not empty; set recursive: true");
            }
        }
        OpsEntry::InsertBeforeText { path, find, .. }
        | OpsEntry::InsertAfterText { path, find, .. } => {
            validate_text_locator(root, path, find, strict)?;
        }
        OpsEntry::ReplaceText {
            path,
            find,
            within_symbol,
            expected_matches,
            ..
        } => {
            validate_replace_text_locator(
                root,
                map,
                path,
                find,
                within_symbol.as_deref(),
                *expected_matches,
                strict,
            )?;
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
            let text = fs::read_to_string(repo_path(root, path)?)?;
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
            let text = fs::read_to_string(repo_path(root, path)?)?;
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

pub(super) fn validate_op_paths(op: &OpsEntry) -> Result<()> {
    for path in op_paths(op) {
        validate_repo_relative_path(path)?;
    }
    for content_from in op_content_from_paths(op) {
        validate_repo_relative_path(content_from)?;
    }
    Ok(())
}

pub(super) fn text_scope_for_symbol(
    map: Option<&CodeMap>,
    path: &Path,
    symbol_name: Option<&str>,
    full_text: &str,
) -> Result<TextScope> {
    let Some(symbol_name) = symbol_name else {
        return Ok(TextScope {
            start_byte: 0,
            end_byte: full_text.len(),
            text: full_text.to_string(),
        });
    };

    let map = map.ok_or_else(|| anyhow::anyhow!("within_symbol requires codemap"))?;
    let path_text = path.to_string_lossy().replace('\\', "/");
    let file = map
        .files
        .iter()
        .find(|file| file.path.to_string_lossy().replace('\\', "/") == path_text)
        .ok_or_else(|| {
            anyhow::anyhow!("file not found in codemap for within_symbol: {path_text}")
        })?;

    let matches = map
        .symbols
        .iter()
        .filter(|symbol| symbol.file_id == file.id && symbol.name == symbol_name)
        .collect::<Vec<_>>();

    let symbol = match matches.as_slice() {
        [symbol] => *symbol,
        [] => bail!(
            "within_symbol not found in {}: {}",
            path.display(),
            symbol_name
        ),
        _ => bail!(
            "within_symbol is ambiguous in {}: {}",
            path.display(),
            symbol_name
        ),
    };

    let (start_byte, end_byte) = line_range_byte_span(full_text, symbol.line, symbol.line_end)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "invalid within_symbol range for {} in {}:{}-{}",
                symbol_name,
                path.display(),
                symbol.line,
                symbol.line_end
            )
        })?;

    Ok(TextScope {
        start_byte,
        end_byte,
        text: full_text[start_byte..end_byte].to_string(),
    })
}

pub(super) struct TextScope {
    pub(super) start_byte: usize,
    pub(super) end_byte: usize,
    pub(super) text: String,
}

fn validate_existing_file(root: &Path, path: &Path, expected_hash: Option<&str>) -> Result<()> {
    let full = repo_path(root, path)?;
    if !full.exists() {
        bail!("file does not exist: {}", path.display());
    }
    if let Some(expected_hash) = expected_hash {
        let bytes = fs::read(&full)?;
        let actual = short_hash(&bytes);
        let actual_sha = short_sha256_hash(&bytes);
        if actual != expected_hash && actual_sha != expected_hash {
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
    let full = repo_path(root, path)?;
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

fn validate_text_locator(root: &Path, path: &Path, find: &str, strict: bool) -> Result<()> {
    validate_existing_file(root, path, None)?;
    let text = fs::read_to_string(repo_path(root, path)?)?;
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

fn validate_replace_text_locator(
    root: &Path,
    map: Option<&CodeMap>,
    path: &Path,
    find: &str,
    within_symbol: Option<&str>,
    expected_matches: Option<usize>,
    strict: bool,
) -> Result<()> {
    validate_existing_file(root, path, None)?;
    let text = fs::read_to_string(repo_path(root, path)?)?;
    let actual_find = text_locator_in_text(&text, find)?;
    let scope = text_scope_for_symbol(map, path, within_symbol, &text)?;
    let matches = scope.text.matches(actual_find.as_ref()).count();

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
        return Ok(());
    }

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
    let text = fs::read_to_string(repo_path(root, path)?)?;
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

pub(super) fn text_locator_in_text<'a>(
    text: &str,
    find: &'a str,
) -> Result<std::borrow::Cow<'a, str>> {
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

fn line_range_byte_span(text: &str, start_line: usize, end_line: usize) -> Option<(usize, usize)> {
    if start_line == 0 || end_line < start_line {
        return None;
    }

    let mut line = 1usize;
    let mut start_byte = None;
    let mut end_byte = None;
    for (index, ch) in text.char_indices() {
        if line == start_line && start_byte.is_none() {
            start_byte = Some(index);
        }
        if line == end_line + 1 {
            end_byte = Some(index);
            break;
        }
        if ch == '\n' {
            line += 1;
        }
    }

    if line == start_line && start_byte.is_none() {
        start_byte = Some(text.len());
    }
    if start_byte.is_some() && end_byte.is_none() {
        if line >= end_line {
            end_byte = Some(text.len());
        }
    }

    start_byte.zip(end_byte)
}
