use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use anyhow::Result;

use crate::cli::Options;
use crate::model::{
    AreaEntry, CodeMap, DependencyEntry, RelationEntry, SymbolEntry, TextOccurrenceEntry,
};
use crate::{output, snapshot_store};
use amigo_symbol_explorer::git;
use amigo_symbol_explorer::scan::{
    ScanDiagnostics, SymbolExplorerScanOptions, scan_files_with_options, scan_project,
    scan_project_with_files,
};

#[derive(Debug, Clone)]
pub struct IndexDelta {
    pub generation_before: u64,
    pub generation_after: u64,
    pub dirty_paths: usize,
    pub full_refresh: bool,
}

#[derive(Debug, Clone)]
pub struct WorkspaceIndex {
    pub map: CodeMap,
    pub generation: u64,
    pub dirty_paths: BTreeSet<PathBuf>,
    pub last_refresh_ms: u64,
}

impl WorkspaceIndex {
    pub fn from_map(map: CodeMap) -> Self {
        Self {
            map,
            generation: 1,
            dirty_paths: BTreeSet::new(),
            last_refresh_ms: 0,
        }
    }

    pub fn from_full_scan(options: &Options) -> Result<Self> {
        let map = scan_project(&scan_options(options))?;
        Ok(Self {
            map,
            generation: 1,
            dirty_paths: BTreeSet::new(),
            last_refresh_ms: 0,
        })
    }

    pub fn mark_dirty_paths<I>(&mut self, paths: I)
    where
        I: IntoIterator<Item = PathBuf>,
    {
        self.dirty_paths.extend(paths);
    }

    pub fn refresh(&mut self, options: &Options) -> Result<IndexDelta> {
        let generation_before = self.generation;
        let started = Instant::now();
        let scan_options = scan_options(options);
        let full_refresh = self.dirty_paths.is_empty();
        let current_files = scan_files_with_options(&options.root, &scan_options.diagnostics)?;
        let current_files = assign_stable_file_ids(current_files, &self.map.files);
        let dirty_paths = normalize_dirty_paths(&options.root, &self.dirty_paths);

        let map = if full_refresh {
            scan_project_with_files(&scan_options, current_files.clone())?
        } else {
            let touched_files = current_files
                .iter()
                .filter(|file| dirty_paths.contains(&file.path))
                .cloned()
                .collect::<Vec<_>>();
            let touched_map = if touched_files.is_empty() {
                CodeMap::default()
            } else {
                scan_project_with_files(&scan_options, touched_files)?
            };
            merge_map(
                &scan_options.root,
                &self.map,
                &current_files,
                &touched_map,
                &dirty_paths,
            )
        };

        self.map = map;
        self.generation = self.generation.saturating_add(1);
        self.last_refresh_ms = started.elapsed().as_millis().try_into().unwrap_or(u64::MAX);
        self.dirty_paths.clear();

        Ok(IndexDelta {
            generation_before,
            generation_after: self.generation,
            dirty_paths: 0,
            full_refresh,
        })
    }

    pub fn refresh_touched(
        &mut self,
        options: &Options,
        touched: &[PathBuf],
    ) -> Result<IndexDelta> {
        self.mark_dirty_paths(touched.iter().cloned());
        self.refresh(options)
    }
}

pub fn write_outputs(options: &Options, map: &CodeMap) -> Result<bool> {
    let wrote = output::write_codemap(options, map)?;
    snapshot_store::write_snapshot(options, map)?;
    Ok(wrote)
}

pub fn current_changed_paths(root: &Path) -> Result<Vec<PathBuf>> {
    let output = Command::new("git")
        .args(["status", "--short"])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Ok(Vec::new());
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(status_path)
        .collect())
}

fn status_path(line: &str) -> Option<PathBuf> {
    if line.len() < 4 {
        return None;
    }
    let path_text = line[2..].trim();
    let path_text = path_text
        .split(" -> ")
        .last()
        .unwrap_or(path_text)
        .trim_matches('"');
    if path_text.is_empty() {
        None
    } else {
        Some(PathBuf::from(path_text.replace('\\', "/")))
    }
}

fn normalize_dirty_paths(root: &Path, dirty_paths: &BTreeSet<PathBuf>) -> BTreeSet<PathBuf> {
    dirty_paths
        .iter()
        .map(|path| {
            path.strip_prefix(root)
                .map(Path::to_path_buf)
                .unwrap_or_else(|_| path.to_path_buf())
        })
        .collect()
}

fn scan_options(options: &Options) -> SymbolExplorerScanOptions {
    SymbolExplorerScanOptions {
        root: options.root.clone(),
        level: options.level,
        ai: options.ai,
        diagnostics: ScanDiagnostics {
            timings: options.timings,
            progress: options.progress,
            diagnostics: options.diagnostics,
            slow_file_threshold_ms: options.slow_file_threshold_ms,
            max_file_size_bytes: options.max_file_size_bytes,
            max_files: options.max_files,
        },
    }
}

fn assign_stable_file_ids(
    mut files: Vec<crate::model::FileEntry>,
    old_files: &[crate::model::FileEntry],
) -> Vec<crate::model::FileEntry> {
    let old_by_path = old_files
        .iter()
        .map(|file| (normalize_relative(&file.path), file.id.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut next_id = old_files
        .iter()
        .filter_map(|file| {
            file.id
                .strip_prefix('f')
                .and_then(|n| n.parse::<usize>().ok())
        })
        .max()
        .unwrap_or(0)
        + 1;

    for file in &mut files {
        let path = normalize_relative(&file.path);
        if let Some(id) = old_by_path.get(&path) {
            file.id = id.clone();
        } else if file.id.is_empty() {
            file.id = format!("f{next_id}");
            next_id += 1;
        }
    }

    files
}

fn merge_map(
    root: &Path,
    old_map: &CodeMap,
    current_files: &[crate::model::FileEntry],
    touched_map: &CodeMap,
    touched_paths: &BTreeSet<PathBuf>,
) -> CodeMap {
    let touched_ids = current_files
        .iter()
        .filter(|file| touched_paths.contains(&file.path))
        .map(|file| file.id.clone())
        .chain(
            old_map
                .files
                .iter()
                .filter(|file| touched_paths.contains(&file.path))
                .map(|file| file.id.clone()),
        )
        .collect::<BTreeSet<_>>();

    let file_ids = current_files
        .iter()
        .map(|file| (file.path.clone(), file.id.clone()))
        .collect::<BTreeMap<_, _>>();
    let git = git::read_git_info(root, &file_ids);
    let mut files = current_files.to_vec();
    apply_git_state_tags(&mut files, &git);
    let packages = old_map.packages.clone();
    let mut symbols = keep_untouched_symbols(&old_map.symbols, &touched_ids);
    let mut text_occurrences = keep_untouched_text(&old_map.text_occurrences, &touched_ids);
    let mut tags = keep_untouched_tags(&old_map.tags, &touched_ids);
    let mut dependencies = keep_untouched_dependencies(&old_map.dependencies, &touched_ids);
    let mut relations = keep_untouched_relations(&old_map.relations, &touched_ids);

    symbols.extend(touched_map.symbols.clone());
    text_occurrences.extend(touched_map.text_occurrences.clone());
    tags.extend(touched_map.tags.clone());
    dependencies.extend(touched_map.dependencies.clone());
    relations.extend(touched_map.relations.clone());

    let stats = recompute_stats(&files);
    let areas = build_areas(&files);

    CodeMap {
        root_name: old_map.root_name.clone(),
        stats,
        files,
        packages,
        symbols,
        text_occurrences,
        tags,
        dependencies,
        relations,
        areas,
        git,
    }
}

fn keep_untouched_symbols<'a>(
    items: &'a [SymbolEntry],
    touched_ids: &BTreeSet<String>,
) -> Vec<SymbolEntry> {
    items
        .iter()
        .filter(|item| !touched_ids.contains(&item.file_id))
        .cloned()
        .collect()
}

fn keep_untouched_text<'a>(
    items: &'a [TextOccurrenceEntry],
    touched_ids: &BTreeSet<String>,
) -> Vec<TextOccurrenceEntry> {
    items
        .iter()
        .filter(|item| !touched_ids.contains(&item.file_id))
        .cloned()
        .collect()
}

fn keep_untouched_tags<'a>(
    items: &'a [crate::model::CodemapTagEntry],
    touched_ids: &BTreeSet<String>,
) -> Vec<crate::model::CodemapTagEntry> {
    items
        .iter()
        .filter(|item| !touched_ids.contains(&item.file_id))
        .cloned()
        .collect()
}

fn keep_untouched_dependencies(
    items: &[DependencyEntry],
    touched_ids: &BTreeSet<String>,
) -> Vec<DependencyEntry> {
    items
        .iter()
        .filter(|item| !touched_ids.contains(&item.from) && !touched_ids.contains(&item.to))
        .cloned()
        .collect()
}

fn keep_untouched_relations(
    items: &[RelationEntry],
    touched_ids: &BTreeSet<String>,
) -> Vec<RelationEntry> {
    items
        .iter()
        .filter(|item| !touched_ids.contains(&item.from) && !touched_ids.contains(&item.to))
        .cloned()
        .collect()
}

fn recompute_stats(files: &[crate::model::FileEntry]) -> BTreeMap<String, usize> {
    let mut stats = BTreeMap::new();
    stats.insert("f".to_string(), files.len());
    for file in files {
        *stats.entry(file.language.clone()).or_insert(0) += 1;
    }
    stats
}

fn build_areas(files: &[crate::model::FileEntry]) -> Vec<AreaEntry> {
    let mut areas: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for file in files {
        let path = file.path.to_string_lossy().replace('\\', "/");
        let names = area_names(&path);
        for name in names {
            areas.entry(name).or_default().push(file.id.clone());
        }
    }
    areas
        .into_iter()
        .map(|(name, files)| AreaEntry { name, files })
        .collect()
}

fn apply_git_state_tags(files: &mut [crate::model::FileEntry], git: &crate::model::GitInfo) {
    let changed_by_id = git
        .changed
        .iter()
        .filter_map(|change| change.file_id.as_ref().map(|id| (id.clone(), change)))
        .collect::<BTreeMap<_, _>>();

    for file in files {
        file.tags
            .retain(|tag| !tag.starts_with("state:") && !tag.starts_with("status:"));
        if let Some(change) = changed_by_id.get(&file.id) {
            push_unique_tag(&mut file.tags, "state:changed");
            push_unique_tag(&mut file.tags, &format!("status:{}", change.status));
        } else {
            push_unique_tag(&mut file.tags, "state:clean");
        }
    }
}

fn push_unique_tag(tags: &mut Vec<String>, tag: &str) {
    if !tags.iter().any(|item| item == tag) {
        tags.push(tag.to_string());
    }
}

fn area_names(path: &str) -> Vec<String> {
    let mut names = Vec::new();
    if let Some(top_level) = path.split('/').next().filter(|part| !part.is_empty()) {
        names.push(format!("dir:{top_level}"));
    }
    if let Some(crate_path) = crate_area(path) {
        names.push(format!("crate:{crate_path}"));
    }
    if let Some(package_path) = package_area(path) {
        names.push(format!("package:{package_path}"));
    }
    if path.ends_with(".rhai") {
        names.push("lang:rhai".to_string());
    }
    if path.ends_with(".yml") || path.ends_with(".yaml") {
        names.push("lang:yaml".to_string());
    }
    if path.contains("/tests/") || path.ends_with(".test.ts") || path.ends_with(".test.tsx") {
        names.push("tests".to_string());
    }
    names
}

fn crate_area(path: &str) -> Option<String> {
    let parts = path.split('/').collect::<Vec<_>>();
    let crates_index = parts.iter().position(|part| *part == "crates")?;
    let crate_root = parts.get(crates_index + 1)?;
    if *crate_root == "apps" || *crate_root == "tools" {
        let app = parts.get(crates_index + 2)?;
        return Some(format!("{crate_root}/{app}"));
    }
    Some((*crate_root).to_owned())
}

fn package_area(path: &str) -> Option<String> {
    let parts = path.split('/').collect::<Vec<_>>();
    let package_index = parts.iter().position(|part| *part == "packages")?;
    let package = parts.get(package_index + 1)?;
    Some((*package).to_owned())
}

fn normalize_relative(path: &Path) -> PathBuf {
    path.to_path_buf()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    use crate::model::{CodeMap, FileEntry, SymbolEntry};

    use super::{merge_map, status_path};

    #[test]
    fn parses_git_status_path_and_rename_target() {
        assert_eq!(
            status_path(" M src/lib.rs"),
            Some(PathBuf::from("src/lib.rs"))
        );
        assert_eq!(
            status_path("R  old.rs -> src/new.rs"),
            Some(PathBuf::from("src/new.rs"))
        );
    }

    #[test]
    fn incremental_merge_drops_deleted_file_symbols() {
        let old_map = CodeMap {
            root_name: "amigo".to_string(),
            files: vec![file("f1", "src/keep.rs"), file("f2", "src/delete.rs")],
            symbols: vec![symbol("Keep", "f1"), symbol("Delete", "f2")],
            ..CodeMap::default()
        };
        let current_files = vec![file("f1", "src/keep.rs")];
        let touched_paths = BTreeSet::from([PathBuf::from("src/delete.rs")]);

        let merged = merge_map(
            Path::new("."),
            &old_map,
            &current_files,
            &CodeMap::default(),
            &touched_paths,
        );

        assert_eq!(merged.files.len(), 1);
        assert_eq!(merged.symbols.len(), 1);
        assert_eq!(merged.symbols[0].name, "Keep");
    }

    fn file(id: &str, path: &str) -> FileEntry {
        FileEntry {
            id: id.to_string(),
            path: PathBuf::from(path),
            language: "rs".to_string(),
            tags: vec!["state:clean".to_string()],
            ..FileEntry::default()
        }
    }

    fn symbol(name: &str, file_id: &str) -> SymbolEntry {
        SymbolEntry {
            name: name.to_string(),
            file_id: file_id.to_string(),
            line: 1,
            line_end: 1,
            line_count: 1,
            ..SymbolEntry::default()
        }
    }
}
