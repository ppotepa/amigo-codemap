use std::path::Path;

use super::model::OpsEntry;

pub(super) fn describe_op(op: &OpsEntry) -> String {
    match op {
        super::model::OpsEntry::CreateFile { path, .. } => {
            format!("create_file {}", path.display())
        }
        super::model::OpsEntry::ReplaceFile { path, .. } => {
            format!("replace_file {}", path.display())
        }
        super::model::OpsEntry::ReplaceRange {
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
        super::model::OpsEntry::DeleteRange {
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
        super::model::OpsEntry::AppendToFile { path, .. } => {
            format!("append_to_file {}", path.display())
        }
        super::model::OpsEntry::InsertBeforeText { path, .. } => {
            format!("insert_before_text {}", path.display())
        }
        super::model::OpsEntry::InsertAfterText { path, .. } => {
            format!("insert_after_text {}", path.display())
        }
        super::model::OpsEntry::ReplaceText { path, .. } => {
            format!("replace_text {}", path.display())
        }
        super::model::OpsEntry::InsertBeforeAnchor { path, anchor, .. } => {
            format!("insert_before_anchor {} anchor={}", path.display(), anchor)
        }
        super::model::OpsEntry::InsertAfterAnchor { path, anchor, .. } => {
            format!("insert_after_anchor {} anchor={}", path.display(), anchor)
        }
        super::model::OpsEntry::ReplaceBetweenAnchors {
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
        super::model::OpsEntry::DeleteFile { path, .. } => {
            format!("delete_file {}", path.display())
        }
        super::model::OpsEntry::CopyFile { from, to, .. } => {
            format!("copy_file {} -> {}", from.display(), to.display())
        }
        super::model::OpsEntry::MoveFile { from, to, .. } => {
            format!("move_file {} -> {}", from.display(), to.display())
        }
        super::model::OpsEntry::RenameFile { from, to, .. } => {
            format!("rename_file {} -> {}", from.display(), to.display())
        }
        super::model::OpsEntry::CreateDir { path, .. } => format!("create_dir {}", path.display()),
        super::model::OpsEntry::DeleteDir { path, .. } => format!("delete_dir {}", path.display()),
        super::model::OpsEntry::ReplaceSymbol { path, symbol, .. } => {
            format!("replace_symbol {} symbol={}", path.display(), symbol)
        }
        super::model::OpsEntry::DeleteSymbol { path, symbol, .. } => {
            format!("delete_symbol {} symbol={}", path.display(), symbol)
        }
        super::model::OpsEntry::DeleteSymbolIfExists { path, symbol, .. } => {
            format!(
                "delete_symbol_if_exists {} symbol={}",
                path.display(),
                symbol
            )
        }
        super::model::OpsEntry::AssertSymbolAbsent { path, symbol, .. } => format!(
            "assert_symbol_absent {} symbol={}",
            path.as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "-".to_string()),
            symbol
        ),
        super::model::OpsEntry::AssertTextAbsent { path, text, .. } => format!(
            "assert_text_absent {} text={}",
            path.as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "-".to_string()),
            text
        ),
        super::model::OpsEntry::InsertBeforeSymbol { path, symbol, .. } => {
            format!("insert_before_symbol {} symbol={}", path.display(), symbol)
        }
        super::model::OpsEntry::InsertAfterSymbol { path, symbol, .. } => {
            format!("insert_after_symbol {} symbol={}", path.display(), symbol)
        }
        super::model::OpsEntry::ReplaceMethodBody { path, symbol, .. } => {
            format!("replace_method_body {} symbol={}", path.display(), symbol)
        }
        super::model::OpsEntry::ReplaceFieldAccess { path, find, .. } => format!(
            "replace_field_access {} find={}",
            path.as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "-".to_string()),
            find
        ),
    }
}

pub(super) fn op_id(op: &OpsEntry) -> Option<&str> {
    match op {
        super::model::OpsEntry::CreateFile { id, .. }
        | super::model::OpsEntry::ReplaceFile { id, .. }
        | super::model::OpsEntry::ReplaceRange { id, .. }
        | super::model::OpsEntry::DeleteRange { id, .. }
        | super::model::OpsEntry::AppendToFile { id, .. }
        | super::model::OpsEntry::InsertBeforeText { id, .. }
        | super::model::OpsEntry::InsertAfterText { id, .. }
        | super::model::OpsEntry::ReplaceText { id, .. }
        | super::model::OpsEntry::InsertBeforeAnchor { id, .. }
        | super::model::OpsEntry::InsertAfterAnchor { id, .. }
        | super::model::OpsEntry::ReplaceBetweenAnchors { id, .. }
        | super::model::OpsEntry::DeleteFile { id, .. }
        | super::model::OpsEntry::CopyFile { id, .. }
        | super::model::OpsEntry::MoveFile { id, .. }
        | super::model::OpsEntry::RenameFile { id, .. }
        | super::model::OpsEntry::CreateDir { id, .. }
        | super::model::OpsEntry::DeleteDir { id, .. }
        | super::model::OpsEntry::ReplaceSymbol { id, .. }
        | super::model::OpsEntry::DeleteSymbol { id, .. }
        | super::model::OpsEntry::DeleteSymbolIfExists { id, .. }
        | super::model::OpsEntry::AssertSymbolAbsent { id, .. }
        | super::model::OpsEntry::AssertTextAbsent { id, .. }
        | super::model::OpsEntry::InsertBeforeSymbol { id, .. }
        | super::model::OpsEntry::InsertAfterSymbol { id, .. }
        | super::model::OpsEntry::ReplaceMethodBody { id, .. }
        | super::model::OpsEntry::ReplaceFieldAccess { id, .. } => id.as_deref(),
    }
}

pub(super) fn op_kind(op: &OpsEntry) -> &'static str {
    match op {
        super::model::OpsEntry::CreateFile { .. } => "create_file",
        super::model::OpsEntry::ReplaceFile { .. } => "replace_file",
        super::model::OpsEntry::ReplaceRange { .. } => "replace_range",
        super::model::OpsEntry::DeleteRange { .. } => "delete_range",
        super::model::OpsEntry::AppendToFile { .. } => "append_to_file",
        super::model::OpsEntry::InsertBeforeText { .. } => "insert_before_text",
        super::model::OpsEntry::InsertAfterText { .. } => "insert_after_text",
        super::model::OpsEntry::ReplaceText { .. } => "replace_text",
        super::model::OpsEntry::InsertBeforeAnchor { .. } => "insert_before_anchor",
        super::model::OpsEntry::InsertAfterAnchor { .. } => "insert_after_anchor",
        super::model::OpsEntry::ReplaceBetweenAnchors { .. } => "replace_between_anchors",
        super::model::OpsEntry::DeleteFile { .. } => "delete_file",
        super::model::OpsEntry::CopyFile { .. } => "copy_file",
        super::model::OpsEntry::MoveFile { .. } => "move_file",
        super::model::OpsEntry::RenameFile { .. } => "rename_file",
        super::model::OpsEntry::CreateDir { .. } => "create_dir",
        super::model::OpsEntry::DeleteDir { .. } => "delete_dir",
        super::model::OpsEntry::ReplaceSymbol { .. } => "replace_symbol",
        super::model::OpsEntry::DeleteSymbol { .. } => "delete_symbol",
        super::model::OpsEntry::DeleteSymbolIfExists { .. } => "delete_symbol_if_exists",
        super::model::OpsEntry::AssertSymbolAbsent { .. } => "assert_symbol_absent",
        super::model::OpsEntry::AssertTextAbsent { .. } => "assert_text_absent",
        super::model::OpsEntry::InsertBeforeSymbol { .. } => "insert_before_symbol",
        super::model::OpsEntry::InsertAfterSymbol { .. } => "insert_after_symbol",
        super::model::OpsEntry::ReplaceMethodBody { .. } => "replace_method_body",
        super::model::OpsEntry::ReplaceFieldAccess { .. } => "replace_field_access",
    }
}

pub(super) fn op_path(op: &OpsEntry) -> String {
    match op {
        super::model::OpsEntry::CreateFile { path, .. }
        | super::model::OpsEntry::ReplaceFile { path, .. }
        | super::model::OpsEntry::ReplaceRange { path, .. }
        | super::model::OpsEntry::DeleteRange { path, .. }
        | super::model::OpsEntry::AppendToFile { path, .. }
        | super::model::OpsEntry::InsertBeforeText { path, .. }
        | super::model::OpsEntry::InsertAfterText { path, .. }
        | super::model::OpsEntry::ReplaceText { path, .. }
        | super::model::OpsEntry::InsertBeforeAnchor { path, .. }
        | super::model::OpsEntry::InsertAfterAnchor { path, .. }
        | super::model::OpsEntry::ReplaceBetweenAnchors { path, .. }
        | super::model::OpsEntry::DeleteFile { path, .. }
        | super::model::OpsEntry::CreateDir { path, .. }
        | super::model::OpsEntry::DeleteDir { path, .. }
        | super::model::OpsEntry::ReplaceSymbol { path, .. }
        | super::model::OpsEntry::DeleteSymbol { path, .. }
        | super::model::OpsEntry::DeleteSymbolIfExists { path, .. }
        | super::model::OpsEntry::InsertBeforeSymbol { path, .. }
        | super::model::OpsEntry::InsertAfterSymbol { path, .. }
        | super::model::OpsEntry::ReplaceMethodBody { path, .. } => path.display().to_string(),
        super::model::OpsEntry::AssertSymbolAbsent { path, .. }
        | super::model::OpsEntry::AssertTextAbsent { path, .. } => path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "-".to_string()),
        super::model::OpsEntry::ReplaceFieldAccess { path, .. } => path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "-".to_string()),
        super::model::OpsEntry::CopyFile { from, to, .. }
        | super::model::OpsEntry::MoveFile { from, to, .. }
        | super::model::OpsEntry::RenameFile { from, to, .. } => {
            format!("{} -> {}", from.display(), to.display())
        }
    }
}

pub(super) fn op_paths(op: &OpsEntry) -> Vec<&Path> {
    match op {
        super::model::OpsEntry::CreateFile { path, .. }
        | super::model::OpsEntry::ReplaceFile { path, .. }
        | super::model::OpsEntry::ReplaceRange { path, .. }
        | super::model::OpsEntry::DeleteRange { path, .. }
        | super::model::OpsEntry::AppendToFile { path, .. }
        | super::model::OpsEntry::InsertBeforeText { path, .. }
        | super::model::OpsEntry::InsertAfterText { path, .. }
        | super::model::OpsEntry::ReplaceText { path, .. }
        | super::model::OpsEntry::InsertBeforeAnchor { path, .. }
        | super::model::OpsEntry::InsertAfterAnchor { path, .. }
        | super::model::OpsEntry::ReplaceBetweenAnchors { path, .. }
        | super::model::OpsEntry::DeleteFile { path, .. }
        | super::model::OpsEntry::CreateDir { path, .. }
        | super::model::OpsEntry::DeleteDir { path, .. }
        | super::model::OpsEntry::ReplaceSymbol { path, .. }
        | super::model::OpsEntry::DeleteSymbol { path, .. }
        | super::model::OpsEntry::DeleteSymbolIfExists { path, .. }
        | super::model::OpsEntry::InsertBeforeSymbol { path, .. }
        | super::model::OpsEntry::InsertAfterSymbol { path, .. }
        | super::model::OpsEntry::ReplaceMethodBody { path, .. } => vec![path.as_path()],
        super::model::OpsEntry::AssertSymbolAbsent { path, .. }
        | super::model::OpsEntry::AssertTextAbsent { path, .. } => path
            .as_ref()
            .map(|path| vec![path.as_path()])
            .unwrap_or_default(),
        super::model::OpsEntry::ReplaceFieldAccess { path, .. } => path
            .as_ref()
            .map(|path| vec![path.as_path()])
            .unwrap_or_default(),
        super::model::OpsEntry::CopyFile { from, to, .. }
        | super::model::OpsEntry::MoveFile { from, to, .. }
        | super::model::OpsEntry::RenameFile { from, to, .. } => vec![from.as_path(), to.as_path()],
    }
}

pub(super) fn locator_kind(op: &OpsEntry) -> &'static str {
    match op {
        super::model::OpsEntry::CreateFile { .. }
        | super::model::OpsEntry::ReplaceFile { .. }
        | super::model::OpsEntry::DeleteFile { .. }
        | super::model::OpsEntry::CopyFile { .. }
        | super::model::OpsEntry::MoveFile { .. }
        | super::model::OpsEntry::RenameFile { .. }
        | super::model::OpsEntry::CreateDir { .. }
        | super::model::OpsEntry::DeleteDir { .. }
        | super::model::OpsEntry::AppendToFile { .. }
        | super::model::OpsEntry::InsertBeforeText { .. }
        | super::model::OpsEntry::InsertAfterText { .. }
        | super::model::OpsEntry::ReplaceText { .. } => "file",
        super::model::OpsEntry::ReplaceRange { .. }
        | super::model::OpsEntry::DeleteRange { .. } => "range",
        super::model::OpsEntry::InsertBeforeAnchor { .. }
        | super::model::OpsEntry::InsertAfterAnchor { .. }
        | super::model::OpsEntry::ReplaceBetweenAnchors { .. } => "anchor",
        super::model::OpsEntry::ReplaceSymbol { .. }
        | super::model::OpsEntry::DeleteSymbol { .. }
        | super::model::OpsEntry::DeleteSymbolIfExists { .. }
        | super::model::OpsEntry::InsertBeforeSymbol { .. }
        | super::model::OpsEntry::InsertAfterSymbol { .. }
        | super::model::OpsEntry::ReplaceMethodBody { .. } => "symbol",
        super::model::OpsEntry::AssertSymbolAbsent { .. }
        | super::model::OpsEntry::AssertTextAbsent { .. }
        | super::model::OpsEntry::ReplaceFieldAccess { .. } => "file",
    }
}

pub(super) fn locator_confidence(op: &OpsEntry) -> &'static str {
    match op {
        super::model::OpsEntry::ReplaceRange {
            expected_hash,
            context_before,
            context_after,
            ..
        }
        | super::model::OpsEntry::DeleteRange {
            expected_hash,
            context_before,
            context_after,
            ..
        } if expected_hash.is_some() && (context_before.is_some() || context_after.is_some()) => {
            "high"
        }
        super::model::OpsEntry::ReplaceBetweenAnchors { expected_hash, .. }
            if expected_hash.is_some() =>
        {
            "high"
        }
        super::model::OpsEntry::ReplaceText {
            within_symbol: Some(_),
            expected_matches: Some(1),
            ..
        } => "high",
        super::model::OpsEntry::InsertBeforeText { .. }
        | super::model::OpsEntry::InsertAfterText { .. }
        | super::model::OpsEntry::ReplaceText { .. }
        | super::model::OpsEntry::InsertBeforeAnchor { .. }
        | super::model::OpsEntry::InsertAfterAnchor { .. }
        | super::model::OpsEntry::ReplaceBetweenAnchors { .. } => "high",
        super::model::OpsEntry::ReplaceSymbol { expected_hash, .. }
        | super::model::OpsEntry::DeleteSymbol { expected_hash, .. }
        | super::model::OpsEntry::InsertBeforeSymbol { expected_hash, .. }
        | super::model::OpsEntry::InsertAfterSymbol { expected_hash, .. }
        | super::model::OpsEntry::ReplaceMethodBody { expected_hash, .. }
            if expected_hash.is_some() =>
        {
            "high"
        }
        super::model::OpsEntry::ReplaceSymbol { .. }
        | super::model::OpsEntry::DeleteSymbol { .. }
        | super::model::OpsEntry::InsertBeforeSymbol { .. }
        | super::model::OpsEntry::InsertAfterSymbol { .. }
        | super::model::OpsEntry::ReplaceMethodBody { .. } => "medium",
        _ => "medium",
    }
}

pub(super) fn safety_line(op: &OpsEntry, strict: bool) -> String {
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

pub(super) fn op_has_expected_hash(op: &OpsEntry) -> bool {
    match op {
        super::model::OpsEntry::ReplaceFile { expected_hash, .. }
        | super::model::OpsEntry::ReplaceRange { expected_hash, .. }
        | super::model::OpsEntry::DeleteRange { expected_hash, .. }
        | super::model::OpsEntry::AppendToFile { expected_hash, .. }
        | super::model::OpsEntry::ReplaceBetweenAnchors { expected_hash, .. }
        | super::model::OpsEntry::DeleteFile { expected_hash, .. }
        | super::model::OpsEntry::CopyFile { expected_hash, .. }
        | super::model::OpsEntry::MoveFile { expected_hash, .. }
        | super::model::OpsEntry::RenameFile { expected_hash, .. }
        | super::model::OpsEntry::ReplaceSymbol { expected_hash, .. }
        | super::model::OpsEntry::DeleteSymbol { expected_hash, .. }
        | super::model::OpsEntry::InsertBeforeSymbol { expected_hash, .. }
        | super::model::OpsEntry::InsertAfterSymbol { expected_hash, .. }
        | super::model::OpsEntry::ReplaceMethodBody { expected_hash, .. } => {
            expected_hash.is_some()
        }
        _ => false,
    }
}

pub(super) fn op_has_context(op: &OpsEntry) -> bool {
    match op {
        super::model::OpsEntry::ReplaceRange {
            context_before,
            context_after,
            ..
        }
        | super::model::OpsEntry::DeleteRange {
            context_before,
            context_after,
            ..
        }
        | super::model::OpsEntry::ReplaceSymbol {
            context_before,
            context_after,
            ..
        } => context_before.is_some() || context_after.is_some(),
        _ => false,
    }
}

pub(super) fn risk_for_op(op: &OpsEntry) -> &'static str {
    match op {
        super::model::OpsEntry::ReplaceText {
            within_symbol: Some(_),
            expected_matches: Some(_),
            ..
        } => "low",
        super::model::OpsEntry::CreateFile { .. }
        | super::model::OpsEntry::CreateDir { .. }
        | super::model::OpsEntry::ReplaceFile {
            expected_hash: None,
            ..
        }
        | super::model::OpsEntry::InsertBeforeText { .. }
        | super::model::OpsEntry::InsertAfterText { .. }
        | super::model::OpsEntry::ReplaceText { .. }
        | super::model::OpsEntry::InsertBeforeAnchor { .. }
        | super::model::OpsEntry::InsertAfterAnchor { .. }
        | super::model::OpsEntry::AppendToFile { .. } => "low",
        super::model::OpsEntry::ReplaceBetweenAnchors { expected_hash, .. }
            if expected_hash.is_some() =>
        {
            "low"
        }
        super::model::OpsEntry::ReplaceRange {
            expected_hash,
            context_before,
            context_after,
            ..
        }
        | super::model::OpsEntry::DeleteRange {
            expected_hash,
            context_before,
            context_after,
            ..
        } if expected_hash.is_some() && (context_before.is_some() || context_after.is_some()) => {
            "low"
        }
        super::model::OpsEntry::ReplaceSymbol { .. }
        | super::model::OpsEntry::DeleteSymbol { .. }
        | super::model::OpsEntry::InsertBeforeSymbol { .. }
        | super::model::OpsEntry::InsertAfterSymbol { .. }
        | super::model::OpsEntry::ReplaceMethodBody { .. } => "medium",
        super::model::OpsEntry::ReplaceFile { expected_hash, .. }
        | super::model::OpsEntry::DeleteFile { expected_hash, .. }
        | super::model::OpsEntry::CopyFile { expected_hash, .. }
        | super::model::OpsEntry::MoveFile { expected_hash, .. }
        | super::model::OpsEntry::RenameFile { expected_hash, .. }
            if expected_hash.is_some() =>
        {
            "medium"
        }
        _ => "medium",
    }
}

pub(super) fn safety_reason_for_op(op: &OpsEntry) -> &'static str {
    match op {
        super::model::OpsEntry::CreateFile { .. } => "target path is unused",
        super::model::OpsEntry::CreateDir { .. } => "directory will be created if missing",
        super::model::OpsEntry::ReplaceFile { expected_hash, .. } if expected_hash.is_some() => {
            "file exists and expected_hash matched"
        }
        super::model::OpsEntry::ReplaceFile { .. } => {
            "file may be created or replaced; no expected_hash supplied"
        }
        super::model::OpsEntry::ReplaceRange {
            expected_hash,
            context_before,
            context_after,
            ..
        }
        | super::model::OpsEntry::DeleteRange {
            expected_hash,
            context_before,
            context_after,
            ..
        } if expected_hash.is_some() && (context_before.is_some() || context_after.is_some()) => {
            "range is valid; expected_hash and context matched"
        }
        super::model::OpsEntry::ReplaceRange { .. }
        | super::model::OpsEntry::DeleteRange { .. } => {
            "range is valid; add expected_hash/context for stronger drift protection"
        }
        super::model::OpsEntry::AppendToFile { expected_hash, .. } if expected_hash.is_some() => {
            "file exists and expected_hash matched"
        }
        super::model::OpsEntry::AppendToFile { .. } => "file exists; append has no expected_hash",
        super::model::OpsEntry::InsertBeforeText { .. } => "text locator found",
        super::model::OpsEntry::InsertAfterText { .. } => "text locator found",
        super::model::OpsEntry::ReplaceText { .. } => "text locator found",
        super::model::OpsEntry::InsertBeforeAnchor { .. }
        | super::model::OpsEntry::InsertAfterAnchor { .. } => "anchor found",
        super::model::OpsEntry::ReplaceBetweenAnchors { expected_hash, .. }
            if expected_hash.is_some() =>
        {
            "start/end anchors found in order and expected_hash matched"
        }
        super::model::OpsEntry::ReplaceBetweenAnchors { .. } => {
            "start/end anchors found in order; add expected_hash for stronger drift protection"
        }
        super::model::OpsEntry::DeleteFile { expected_hash, .. } if expected_hash.is_some() => {
            "file exists and expected_hash matched"
        }
        super::model::OpsEntry::DeleteFile { .. } => "file exists; no expected_hash supplied",
        super::model::OpsEntry::CopyFile { expected_hash, .. } if expected_hash.is_some() => {
            "source file exists and expected_hash matched"
        }
        super::model::OpsEntry::CopyFile { .. } => "source file exists; target path is available",
        super::model::OpsEntry::MoveFile { expected_hash, .. }
        | super::model::OpsEntry::RenameFile { expected_hash, .. }
            if expected_hash.is_some() =>
        {
            "source file exists and expected_hash matched"
        }
        super::model::OpsEntry::MoveFile { .. } | super::model::OpsEntry::RenameFile { .. } => {
            "source file exists; target path is available"
        }
        super::model::OpsEntry::DeleteDir { recursive, .. } if *recursive => {
            "directory exists and will be deleted recursively"
        }
        super::model::OpsEntry::DeleteDir { .. } => "empty directory exists",
        super::model::OpsEntry::ReplaceSymbol { .. }
        | super::model::OpsEntry::DeleteSymbol { .. }
        | super::model::OpsEntry::DeleteSymbolIfExists { .. }
        | super::model::OpsEntry::InsertBeforeSymbol { .. }
        | super::model::OpsEntry::InsertAfterSymbol { .. }
        | super::model::OpsEntry::AssertSymbolAbsent { .. }
        | super::model::OpsEntry::AssertTextAbsent { .. }
        | super::model::OpsEntry::ReplaceFieldAccess { .. }
        | super::model::OpsEntry::ReplaceMethodBody { .. } => {
            "file exists; symbol range is resolved during ops-apply"
        }
    }
}
