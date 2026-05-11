use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpsInputFormat {
    Yaml,
    Raw,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpsPlan {
    #[serde(default = "default_ops_plan_version")]
    pub version: u16,
    #[serde(default)]
    pub task: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub content_root: Option<PathBuf>,
    pub ops: Vec<OpsEntry>,
    #[serde(default)]
    pub verify: Vec<String>,
    #[serde(skip)]
    pub plan_dir: Option<PathBuf>,
}

const fn default_ops_plan_version() -> u16 {
    1
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind")]
#[serde(deny_unknown_fields)]
pub enum OpsEntry {
    #[serde(rename = "create_file")]
    CreateFile {
        #[serde(default)]
        id: Option<String>,
        path: PathBuf,
        #[serde(default)]
        content: Option<String>,
        #[serde(default)]
        content_from: Option<PathBuf>,
        #[serde(default)]
        overwrite: bool,
    },
    #[serde(rename = "replace_file")]
    ReplaceFile {
        path: PathBuf,
        #[serde(default)]
        content: Option<String>,
        #[serde(default)]
        content_from: Option<PathBuf>,
        #[serde(default)]
        overwrite: bool,
        expected_hash: Option<String>,
        #[serde(default)]
        id: Option<String>,
    },
    #[serde(rename = "replace_range")]
    ReplaceRange {
        path: PathBuf,
        start_line: usize,
        end_line: usize,
        #[serde(default)]
        content: Option<String>,
        #[serde(default)]
        content_from: Option<PathBuf>,
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
        #[serde(default)]
        content: Option<String>,
        #[serde(default)]
        content_from: Option<PathBuf>,
        expected_hash: Option<String>,
    },
    #[serde(rename = "insert_before_text")]
    InsertBeforeText {
        #[serde(default)]
        id: Option<String>,
        path: PathBuf,
        find: String,
        #[serde(default)]
        content: Option<String>,
        #[serde(default)]
        content_from: Option<PathBuf>,
    },
    #[serde(rename = "insert_after_text")]
    InsertAfterText {
        #[serde(default)]
        id: Option<String>,
        path: PathBuf,
        find: String,
        #[serde(default)]
        content: Option<String>,
        #[serde(default)]
        content_from: Option<PathBuf>,
    },
    #[serde(rename = "replace_text")]
    ReplaceText {
        #[serde(default)]
        id: Option<String>,
        path: PathBuf,
        find: String,
        #[serde(default)]
        replace: Option<String>,
        #[serde(default)]
        content_from: Option<PathBuf>,
        #[serde(default)]
        within_symbol: Option<String>,
        #[serde(default)]
        expected_matches: Option<usize>,
    },
    #[serde(rename = "insert_before_anchor")]
    InsertBeforeAnchor {
        #[serde(default)]
        id: Option<String>,
        path: PathBuf,
        anchor: String,
        #[serde(default)]
        content: Option<String>,
        #[serde(default)]
        content_from: Option<PathBuf>,
    },
    #[serde(rename = "insert_after_anchor")]
    InsertAfterAnchor {
        #[serde(default)]
        id: Option<String>,
        path: PathBuf,
        anchor: String,
        #[serde(default)]
        content: Option<String>,
        #[serde(default)]
        content_from: Option<PathBuf>,
    },
    #[serde(rename = "replace_between_anchors")]
    ReplaceBetweenAnchors {
        path: PathBuf,
        start_anchor: String,
        end_anchor: String,
        #[serde(default)]
        content: Option<String>,
        #[serde(default)]
        content_from: Option<PathBuf>,
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
    #[serde(rename = "copy_file")]
    CopyFile {
        #[serde(default)]
        id: Option<String>,
        from: PathBuf,
        to: PathBuf,
        expected_hash: Option<String>,
        #[serde(default)]
        overwrite: bool,
    },
    #[serde(rename = "move_file")]
    MoveFile {
        #[serde(default)]
        id: Option<String>,
        from: PathBuf,
        to: PathBuf,
        expected_hash: Option<String>,
        #[serde(default)]
        overwrite: bool,
    },
    #[serde(rename = "rename_file")]
    RenameFile {
        #[serde(default)]
        id: Option<String>,
        from: PathBuf,
        to: PathBuf,
        expected_hash: Option<String>,
        #[serde(default)]
        overwrite: bool,
    },
    #[serde(rename = "create_dir")]
    CreateDir {
        #[serde(default)]
        id: Option<String>,
        path: PathBuf,
    },
    #[serde(rename = "delete_dir")]
    DeleteDir {
        #[serde(default)]
        id: Option<String>,
        path: PathBuf,
        #[serde(default)]
        recursive: bool,
    },
    #[serde(rename = "replace_symbol")]
    ReplaceSymbol {
        path: PathBuf,
        symbol: String,
        #[serde(default)]
        content: Option<String>,
        #[serde(default)]
        content_from: Option<PathBuf>,
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
    #[serde(rename = "delete_symbol_if_exists")]
    DeleteSymbolIfExists {
        path: PathBuf,
        symbol: String,
        expected_hash: Option<String>,
        #[serde(default)]
        id: Option<String>,
    },
    #[serde(rename = "assert_symbol_absent")]
    AssertSymbolAbsent {
        #[serde(default)]
        id: Option<String>,
        #[serde(default)]
        path: Option<PathBuf>,
        symbol: String,
    },
    #[serde(rename = "assert_text_absent")]
    AssertTextAbsent {
        #[serde(default)]
        id: Option<String>,
        #[serde(default)]
        path: Option<PathBuf>,
        text: String,
        #[serde(default)]
        changed_only: bool,
    },
    #[serde(rename = "insert_before_symbol")]
    InsertBeforeSymbol {
        path: PathBuf,
        symbol: String,
        #[serde(default)]
        content: Option<String>,
        #[serde(default)]
        content_from: Option<PathBuf>,
        expected_hash: Option<String>,
        #[serde(default)]
        id: Option<String>,
    },
    #[serde(rename = "insert_after_symbol")]
    InsertAfterSymbol {
        path: PathBuf,
        symbol: String,
        #[serde(default)]
        content: Option<String>,
        #[serde(default)]
        content_from: Option<PathBuf>,
        expected_hash: Option<String>,
        #[serde(default)]
        id: Option<String>,
    },
    #[serde(rename = "replace_method_body")]
    ReplaceMethodBody {
        path: PathBuf,
        symbol: String,
        #[serde(default)]
        content: Option<String>,
        #[serde(default)]
        content_from: Option<PathBuf>,
        expected_hash: Option<String>,
        #[serde(default)]
        id: Option<String>,
    },
    #[serde(rename = "replace_field_access")]
    ReplaceFieldAccess {
        #[serde(default)]
        id: Option<String>,
        path: Option<PathBuf>,
        find: String,
        replace: String,
        #[serde(default)]
        scope: Option<String>,
        #[serde(default)]
        expected_matches: Option<usize>,
    },
}
