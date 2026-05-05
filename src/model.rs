use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeMap {
    pub root_name: String,
    pub stats: BTreeMap<String, usize>,
    pub files: Vec<FileEntry>,
    pub packages: Vec<PackageEntry>,
    pub symbols: Vec<SymbolEntry>,
    pub dependencies: Vec<DependencyEntry>,
    pub areas: Vec<AreaEntry>,
    pub git: GitInfo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub id: String,
    pub path: PathBuf,
    pub language: String,
    pub lines: usize,
    pub hash: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolEntry {
    pub name: String,
    pub kind: String,
    pub file_id: String,
    pub line: usize,
    pub visibility: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageEntry {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub version: Option<String>,
    pub manifest_path: PathBuf,
    pub dependencies: Vec<String>,
    pub scripts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyEntry {
    pub from: String,
    pub to: String,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaEntry {
    pub name: String,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitInfo {
    pub branch: String,
    pub rev: String,
    pub dirty: bool,
    pub changed: Vec<GitChange>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitChange {
    pub status: String,
    pub path: PathBuf,
    pub file_id: Option<String>,
}
