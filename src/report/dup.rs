use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;
use regex::Regex;

use crate::model::CodeMap;

use super::common::{changed_by_path, files_by_id, print_next, slash_path};

#[allow(dead_code)]
pub fn normalize_body(body: &str) -> String {
    body.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<String>()
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect()
}

pub fn print_dup(root: &std::path::Path, map: &CodeMap, query: Option<&str>, changed_only: bool, limit: usize) -> Result<()> {
    println!("task: dup{}", query.map(|q| format!(" {q}")).unwrap_or_default());
    let mut matches = BTreeMap::<String, Vec<String>>::new();
    let files = files_by_id(map);
    let changed = changed_by_path(map);

    for symbol in &map.symbols {
        if let Some(query) = query {
            if symbol.name != query {
                continue;
            }
        }
        if !matches!(symbol.kind.as_str(), "fn" | "component" | "hook") {
            continue;
        }
        let Some(file) = files.get(symbol.file_id.as_str()) else {
            continue;
        };
        let path = slash_path(&file.path);
        if changed_only && !changed.contains_key(&path) {
            continue;
        }
        matches
            .entry(symbol.name.clone())
            .or_default()
            .push(format!("{}:{} {}", path, symbol.line, symbol.kind));
    }

    let body_matches = if query.is_none() {
        duplicate_bodies(root, map, changed_only, limit)?
    } else {
        BTreeMap::new()
    };
    println!("matches:");
    let mut any = false;
    for (name, paths) in matches.into_iter().filter(|(_, paths)| paths.len() > 1).take(limit) {
        any = true;
        println!("  {name}:");
        for path in paths {
            println!("    {path}");
        }
    }
    if !any {
        println!("  none");
    }
    println!("body matches:");
    if body_matches.is_empty() {
        println!("  none");
    } else {
        for (hash, paths) in body_matches.into_iter().take(limit) {
            println!("  {hash}:");
            for path in paths {
                println!("    {path}");
            }
        }
    }
    println!("suggest:");
    println!("  canonical: shared module closest to existing callers");
    println!("  visibility: pub(crate)");
    print_next(&["keep canonical implementation", "delete local copies", "run verify-plan"]);
    Ok(())
}

fn duplicate_bodies(
    root: &std::path::Path,
    map: &CodeMap,
    changed_only: bool,
    limit: usize,
) -> Result<BTreeMap<String, Vec<String>>> {
    let changed = changed_by_path(map);
    let re = Regex::new(r"(?s)\b(?:pub\s+)?(?:async\s+)?fn\s+([A-Za-z0-9_]+)\s*\([^)]*\)\s*(?:->[^{]+)?\{(?P<body>.*?)\n\}")?;
    let mut by_body = BTreeMap::<String, Vec<String>>::new();
    for file in &map.files {
        let path = slash_path(&file.path);
        if changed_only && !changed.contains_key(&path) {
            continue;
        }
        let Ok(text) = fs::read_to_string(root.join(&file.path)) else {
            continue;
        };
        for caps in re.captures_iter(&text).take(limit) {
            let body = normalize_body(caps.name("body").map(|m| m.as_str()).unwrap_or(""));
            if body.len() < 12 {
                continue;
            }
            by_body.entry(body).or_default().push(format!("{} {}", path, &caps[1]));
        }
    }
    Ok(by_body
        .into_iter()
        .filter(|(_, paths)| paths.len() > 1)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::normalize_body;

    #[test]
    fn detects_exact_normalized_body() {
        assert_eq!(normalize_body("fn a() {\n //x\n 1\n}"), "fna(){1}");
    }
}
