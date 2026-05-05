use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

use anyhow::{Result, bail};

use crate::model::{CodeMap, FileEntry, GitChange, PackageEntry};

pub fn print_brief(map: &CodeMap) {
    println!(
        "repo: {}  branch: {}  rev: {}  dirty: {}",
        map.root_name,
        empty_dash(&map.git.branch),
        empty_dash(&map.git.rev),
        map.git.changed.len()
    );
    println!(
        "files: {}  packages: {}  symbols: {}  deps: {}",
        map.files.len(),
        map.packages.len(),
        map.symbols.len(),
        map.dependencies.len()
    );

    print_counts("languages", language_counts(map).into_iter().take(10));
    print_counts("packages", package_kind_counts(map).into_iter().take(8));
    print_counts(
        "changed:path",
        changed_group_counts(map, "path").into_iter().take(12),
    );
}

pub fn print_changed(map: &CodeMap, group: Option<&str>, limit: usize) {
    if let Some(group) = group {
        print_counts(
            &format!("changed:{group}"),
            changed_group_counts(map, group).into_iter().take(limit),
        );
        return;
    }

    for change in map.git.changed.iter().take(limit) {
        let target = change
            .file_id
            .as_deref()
            .unwrap_or_else(|| change.path.to_str().unwrap_or(""));
        println!("{}\t{}", change.status, target);
    }
}

pub fn print_find(
    root: &Path,
    map: &CodeMap,
    query: &str,
    lines: bool,
    limit: usize,
) -> Result<()> {
    if query.is_empty() {
        bail!("find requires a query");
    }

    let needles = vec![query.to_ascii_lowercase()];
    let mut emitted = 0usize;
    for file in &map.files {
        if emitted >= limit {
            break;
        }
        let path = root.join(&file.path);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let matches = matching_lines(&text, &needles);
        if matches.is_empty() {
            continue;
        }
        emitted += 1;
        println!(
            "{}\t{}\t{}",
            slash_path(&file.path),
            matches.len(),
            line_list(&matches)
        );
        if lines {
            for (line, text) in matches.iter().take(3) {
                println!("  {line}: {}", trim_line(text));
            }
        }
    }
    if emitted == 0 {
        println!("no matches: {query}");
    }
    Ok(())
}

pub fn print_scope(map: &CodeMap, query: &str, limit: usize) {
    let selected = scope_file_ids(map, query);
    println!("scope: {query}");
    if selected.is_empty() {
        println!("no files");
        return;
    }

    let changed = changed_by_path(map);
    let files_by_id = files_by_id(map);

    println!("files:");
    for file_id in selected.iter().take(limit) {
        if let Some(file) = files_by_id.get(file_id.as_str()) {
            let status = changed
                .get(&slash_path(&file.path))
                .map(|change| format!(" changed:{}", change.status))
                .unwrap_or_default();
            println!(
                "  {}\t{}\t{}\t{}{}",
                file.id,
                slash_path(&file.path),
                file.language,
                file.lines,
                status
            );
        }
    }

    println!("symbols:");
    let mut symbol_count = 0usize;
    for symbol in &map.symbols {
        if selected.contains(&symbol.file_id) {
            symbol_count += 1;
            if symbol_count <= limit {
                println!(
                    "  {}\t{}\t{}:{}\t{}",
                    symbol.kind, symbol.name, symbol.file_id, symbol.line, symbol.visibility
                );
            }
        }
    }
    if symbol_count > limit {
        println!("  ... {} more", symbol_count - limit);
    }

    println!("relations:");
    let mut relation_count = 0usize;
    for dep in &map.dependencies {
        if selected.contains(&dep.from) || selected.contains(&dep.to) {
            relation_count += 1;
            if relation_count <= limit {
                println!("  {}\t{}\t{}", dep.from, dep.to, dep.kind);
            }
        }
    }
    if relation_count > limit {
        println!("  ... {} more", relation_count - limit);
    }
}

pub fn print_refs(
    root: &Path,
    map: &CodeMap,
    query: &str,
    lines: bool,
    limit: usize,
) -> Result<()> {
    if query.is_empty() {
        bail!("refs requires a query");
    }

    println!("refs: {query}");
    println!("defs:");
    let mut defs = 0usize;
    for symbol in &map.symbols {
        if symbol.name == query || symbol.name.contains(query) {
            defs += 1;
            if defs <= limit {
                println!(
                    "  {}\t{}\t{}:{}\t{}",
                    symbol.kind, symbol.name, symbol.file_id, symbol.line, symbol.visibility
                );
            }
        }
    }
    if defs == 0 {
        println!("  none");
    }

    let needles = ref_needles(query);
    println!("uses:");
    let mut emitted = 0usize;
    for file in &map.files {
        if emitted >= limit {
            break;
        }
        let Ok(text) = fs::read_to_string(root.join(&file.path)) else {
            continue;
        };
        let matches = matching_lines(&text, &needles);
        if matches.is_empty() {
            continue;
        }
        emitted += 1;
        println!(
            "  {}\t{}\t{}",
            slash_path(&file.path),
            matches.len(),
            line_list(&matches)
        );
        if lines {
            for (line, text) in matches.iter().take(3) {
                println!("    {line}: {}", trim_line(text));
            }
        }
    }
    if emitted == 0 {
        println!("  none");
    }
    Ok(())
}

pub fn print_docs(root: &Path, map: &CodeMap) {
    let mut missing = Vec::new();
    let mut present = 0usize;
    for package in &map.packages {
        let readme = root
            .join(&package.manifest_path)
            .parent()
            .map(|parent| parent.join("README.md"));
        if readme.as_deref().is_some_and(Path::exists) {
            present += 1;
        } else {
            missing.push(package);
        }
    }
    println!("readme coverage: {}/{}", present, map.packages.len());
    if missing.is_empty() {
        println!("missing: none");
    } else {
        println!("missing:");
        for package in missing {
            println!(
                "  {}\t{}\t{}",
                package.kind,
                package.name,
                slash_path(&package.manifest_path)
            );
        }
    }

    let doc_like = map
        .files
        .iter()
        .filter(|file| {
            let path = slash_path(&file.path);
            path.starts_with("docs/") || path.contains("/docs/") || path.contains("/tasks/")
        })
        .count();
    println!("doc/task files indexed: {doc_like}");
}

pub fn run_verify(root: &Path, args: &[String], limit: usize) -> Result<()> {
    let Some(profile) = args.first().map(String::as_str) else {
        println!("profiles: npm-build, npm-test, cargo-check, cargo-test");
        return Ok(());
    };

    let rest = &args[1..];
    let (program, command_args, cwd) = match profile {
        "npm-build" => (
            "npm",
            vec!["run".to_string(), "build".to_string()],
            verify_dir(root, rest),
        ),
        "npm-test" => ("npm", vec!["test".to_string()], verify_dir(root, rest)),
        "cargo-check" => {
            let mut args = vec!["check".to_string()];
            if let Some(package) = rest.first() {
                args.extend(["-p".to_string(), package.to_owned()]);
            }
            ("cargo", args, root.to_path_buf())
        }
        "cargo-test" => {
            let mut args = vec!["test".to_string()];
            if let Some(package) = rest.first() {
                args.extend(["-p".to_string(), package.to_owned()]);
            }
            ("cargo", args, root.to_path_buf())
        }
        _ => bail!("unknown verify profile `{profile}`"),
    };

    let output = ProcessCommand::new(program)
        .args(&command_args)
        .current_dir(&cwd)
        .output()?;
    println!("verify: {} {}", program, command_args.join(" "));
    println!("cwd: {}", cwd.display());
    println!("status: {}", output.status);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let lines = stdout
        .lines()
        .chain(stderr.lines())
        .filter(|line| !line.trim().is_empty())
        .take(limit)
        .collect::<Vec<_>>();
    for line in lines {
        println!("{line}");
    }
    Ok(())
}

fn scope_file_ids(map: &CodeMap, query: &str) -> BTreeSet<String> {
    let query_lower = query.to_ascii_lowercase();
    let mut selected = BTreeSet::new();

    for file in &map.files {
        let path = slash_path(&file.path);
        if file.id == query || path.to_ascii_lowercase().contains(&query_lower) {
            selected.insert(file.id.clone());
        }
    }

    for area in &map.areas {
        if area.name == query || area.name.to_ascii_lowercase().contains(&query_lower) {
            selected.extend(area.files.iter().cloned());
        }
    }

    for package in &map.packages {
        if package.name == query
            || slash_path(&package.manifest_path)
                .to_ascii_lowercase()
                .contains(&query_lower)
        {
            let prefix = package_prefix(package);
            for file in &map.files {
                if slash_path(&file.path).starts_with(&prefix) {
                    selected.insert(file.id.clone());
                }
            }
        }
    }

    for symbol in &map.symbols {
        if symbol.name == query || symbol.name.to_ascii_lowercase().contains(&query_lower) {
            selected.insert(symbol.file_id.clone());
        }
    }

    selected
}

fn package_prefix(package: &PackageEntry) -> String {
    slash_path(
        package
            .manifest_path
            .parent()
            .unwrap_or_else(|| Path::new("")),
    )
}

fn verify_dir(root: &Path, args: &[String]) -> PathBuf {
    args.first()
        .map(|path| root.join(path))
        .unwrap_or_else(|| root.to_path_buf())
}

fn changed_group_counts(map: &CodeMap, group: &str) -> Vec<(String, usize)> {
    let files_by_path = files_by_path(map);
    let mut counts = BTreeMap::<String, usize>::new();
    for change in &map.git.changed {
        let key = match group {
            "status" => change.status.clone(),
            "language" => files_by_path
                .get(&slash_path(&change.path))
                .map(|file| file.language.clone())
                .unwrap_or_else(|| language_from_path(&change.path)),
            "package" => {
                package_for_path(map, &change.path).unwrap_or_else(|| path_group(&change.path))
            }
            "path" => path_group(&change.path),
            _ => path_group(&change.path),
        };
        *counts.entry(key).or_default() += 1;
    }
    sort_counts(counts)
}

fn language_counts(map: &CodeMap) -> Vec<(String, usize)> {
    sort_counts(
        map.files
            .iter()
            .fold(BTreeMap::<String, usize>::new(), |mut counts, file| {
                *counts.entry(file.language.clone()).or_default() += 1;
                counts
            }),
    )
}

fn package_kind_counts(map: &CodeMap) -> Vec<(String, usize)> {
    sort_counts(map.packages.iter().fold(
        BTreeMap::<String, usize>::new(),
        |mut counts, package| {
            *counts.entry(package.kind.clone()).or_default() += 1;
            counts
        },
    ))
}

fn sort_counts(counts: BTreeMap<String, usize>) -> Vec<(String, usize)> {
    let mut counts = counts.into_iter().collect::<Vec<_>>();
    counts.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    counts
}

fn print_counts<'a>(title: &str, counts: impl Iterator<Item = (String, usize)> + 'a) {
    println!("{title}:");
    let mut any = false;
    for (key, count) in counts {
        any = true;
        println!("  {key}\t{count}");
    }
    if !any {
        println!("  none");
    }
}

fn package_for_path(map: &CodeMap, path: &Path) -> Option<String> {
    let path = slash_path(path);
    map.packages
        .iter()
        .filter_map(|package| {
            let prefix = package_prefix(package);
            path.starts_with(&prefix)
                .then(|| (prefix.len(), package.name.clone()))
        })
        .max_by_key(|(len, _)| *len)
        .map(|(_, name)| name)
}

fn path_group(path: &Path) -> String {
    let path = slash_path(path);
    let parts = path.split('/').collect::<Vec<_>>();
    if parts.first() == Some(&"crates") && parts.len() >= 3 {
        if parts.get(1) == Some(&"apps") || parts.get(1) == Some(&"tools") {
            return parts.iter().take(3).copied().collect::<Vec<_>>().join("/");
        }
        return parts.iter().take(2).copied().collect::<Vec<_>>().join("/");
    }
    parts.first().copied().unwrap_or("root").to_string()
}

fn files_by_id(map: &CodeMap) -> BTreeMap<&str, &FileEntry> {
    map.files
        .iter()
        .map(|file| (file.id.as_str(), file))
        .collect()
}

fn files_by_path(map: &CodeMap) -> BTreeMap<String, &FileEntry> {
    map.files
        .iter()
        .map(|file| (slash_path(&file.path), file))
        .collect()
}

fn changed_by_path(map: &CodeMap) -> BTreeMap<String, &GitChange> {
    map.git
        .changed
        .iter()
        .map(|change| (slash_path(&change.path), change))
        .collect()
}

fn ref_needles(query: &str) -> Vec<String> {
    let mut needles = BTreeSet::new();
    needles.insert(query.to_ascii_lowercase());
    let bare = query.trim_start_matches(['.', '#']).to_ascii_lowercase();
    if !bare.is_empty() {
        needles.insert(bare);
    }
    needles.into_iter().collect()
}

fn matching_lines<'a>(text: &'a str, needles: &[String]) -> Vec<(usize, &'a str)> {
    text.lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let lower = line.to_ascii_lowercase();
            needles
                .iter()
                .any(|needle| lower.contains(needle))
                .then_some((index + 1, line))
        })
        .collect()
}

fn line_list(matches: &[(usize, &str)]) -> String {
    matches
        .iter()
        .take(8)
        .map(|(line, _)| line.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn trim_line(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.len() > 140 {
        format!("{}...", &trimmed[..140])
    } else {
        trimmed.to_string()
    }
}

fn language_from_path(path: &Path) -> String {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if file_name == "cargo.toml" {
        return "cargo".to_string();
    }
    if file_name == "package.json" {
        return "package".to_string();
    }
    path.extension()
        .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_else(|| "txt".to_string())
}

fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn empty_dash(value: &str) -> &str {
    if value.is_empty() { "-" } else { value }
}
