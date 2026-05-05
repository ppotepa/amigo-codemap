mod files;
mod packages;
mod symbols;

use std::collections::BTreeMap;

use anyhow::Result;

use crate::cli::Options;
use crate::git;
use crate::model::{AreaEntry, CodeMap};

pub fn scan_project(options: &Options) -> Result<CodeMap> {
    let mut files = files::scan_files(&options.root)?;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    for (index, file) in files.iter_mut().enumerate() {
        file.id = format!("f{}", index + 1);
    }

    let mut stats = BTreeMap::new();
    stats.insert("f".to_string(), files.len());
    for file in &files {
        *stats.entry(file.language.clone()).or_insert(0) += 1;
    }

    let file_ids = files
        .iter()
        .map(|file| (file.path.clone(), file.id.clone()))
        .collect::<BTreeMap<_, _>>();

    let packages = packages::scan_packages(&options.root, &files)?;

    let symbols = if options.level > 0 {
        symbols::scan_symbols(&options.root, &files, options.level)?
    } else {
        Vec::new()
    };
    let mut dependencies = if options.level >= 3 || options.ai {
        symbols::scan_dependencies(&options.root, &files, &file_ids)?
    } else {
        Vec::new()
    };
    if options.ai || options.level >= 2 {
        dependencies.extend(symbols::scan_ai_relations(
            &options.root,
            &files,
            &file_ids,
        )?);
        dependencies.sort_by(|a, b| (&a.from, &a.to, &a.kind).cmp(&(&b.from, &b.to, &b.kind)));
        dependencies.dedup();
    }
    let areas = build_areas(&files);
    let git = git::read_git_info(&options.root, &file_ids);

    Ok(CodeMap {
        root_name: options
            .root
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "repo".to_string()),
        stats,
        files,
        packages,
        symbols,
        dependencies,
        areas,
        git,
    })
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
