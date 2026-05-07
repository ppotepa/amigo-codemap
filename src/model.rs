use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const CODEMAP_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeMap {
    pub root_name: String,
    pub stats: BTreeMap<String, usize>,
    pub files: Vec<FileEntry>,
    pub packages: Vec<PackageEntry>,
    pub symbols: Vec<SymbolEntry>,
    pub text_occurrences: Vec<TextOccurrenceEntry>,
    pub tags: Vec<CodemapTagEntry>,
    pub dependencies: Vec<DependencyEntry>,
    pub relations: Vec<RelationEntry>,
    pub areas: Vec<AreaEntry>,
    pub git: GitInfo,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    pub id: String,
    pub path: PathBuf,
    pub language: String,
    pub lines: usize,
    pub hash: String,
    pub size: u64,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolEntry {
    pub name: String,
    pub kind: String,
    pub file_id: String,
    pub line: usize,
    pub line_end: usize,
    pub line_count: usize,
    pub signature: String,
    pub params: Vec<String>,
    pub return_type: Option<String>,
    pub generics: Vec<String>,
    pub visibility: String,
    pub owner: Option<String>,
    pub tags: Vec<String>,
    pub confidence: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextOccurrenceEntry {
    pub id: String,
    pub value: String,
    pub normalized_value: String,
    pub kind: String,
    pub file_id: String,
    pub line: usize,
    pub column: usize,
    pub owner: Option<String>,
    pub context: String,
    pub tags: Vec<String>,
    pub confidence: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodemapTagEntry {
    pub name: String,
    pub anchor: String,
    pub file_id: String,
    pub line: usize,
    pub target: String,
    pub domain: Option<String>,
    pub role: Option<String>,
    pub priority: Option<String>,
    pub layer: Option<String>,
    pub status: Option<String>,
    pub risk: Option<String>,
    pub owner: Option<String>,
    pub tags: Vec<String>,
    pub values: BTreeMap<String, String>,
    pub raw: String,
    pub generated: bool,
    pub confidence: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnchorIndex {
    pub version: u16,
    pub repo: String,
    pub generated_at: String,
    pub taxonomy_path: String,
    pub counts: AnchorIndexCounts,
    pub anchors: Vec<AnchorIndexEntry>,
    pub diagnostics: Vec<AnchorDiagnostic>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnchorIndexCounts {
    pub anchors: usize,
    pub manual: usize,
    pub generated: usize,
    pub p0: usize,
    pub p1: usize,
    pub p2: usize,
    pub p3: usize,
    pub px: usize,
    pub errors: usize,
    pub warnings: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnchorIndexEntry {
    pub anchor: String,
    pub domain: String,
    pub role: String,
    pub priority: String,
    pub layer: Option<String>,
    pub status: Option<String>,
    pub risk: Option<String>,
    pub tags: Vec<String>,
    pub file: String,
    pub line: usize,
    pub comment: String,
    pub generated: bool,
    pub score: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnchorDiagnostic {
    pub severity: String,
    pub kind: String,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<usize>,
    pub anchor: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationEntry {
    pub from: String,
    pub to: String,
    pub kind: String,
    pub confidence: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageEntry {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub version: Option<String>,
    pub manifest_path: PathBuf,
    pub dependencies: Vec<String>,
    pub scripts: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyEntry {
    pub from: String,
    pub to: String,
    pub kind: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AreaEntry {
    pub name: String,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitInfo {
    pub branch: String,
    pub rev: String,
    pub dirty: bool,
    pub changed: Vec<GitChange>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitChange {
    pub status: String,
    pub path: PathBuf,
    pub file_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codemap_schema_version_is_v1() {
        assert_eq!(CODEMAP_SCHEMA_VERSION, 1);
    }

    #[test]
    fn symbol_line_count_can_cover_single_line() {
        let symbol = SymbolEntry {
            name: "Foo".to_string(),
            kind: "struct".to_string(),
            file_id: "f1".to_string(),
            line: 10,
            line_end: 10,
            line_count: 1,
            signature: "pub struct Foo".to_string(),
            params: Vec::new(),
            return_type: None,
            generics: Vec::new(),
            visibility: "pub".to_string(),
            owner: None,
            tags: vec!["kind:struct".to_string()],
            confidence: 80,
        };

        assert_eq!(symbol.line_count, 1);
    }
}
