use std::fs;
use std::path::Path;

use anyhow::{Result, bail};

use crate::model::CodeMap;

pub fn print_anchor_range(
    root: &Path,
    map: &CodeMap,
    query: &str,
    to: Option<&str>,
    limit: usize,
) -> Result<()> {
    if query.trim().is_empty() {
        bail!("anchor-range requires an anchor query");
    }

    let files = map
        .files
        .iter()
        .map(|file| (file.id.as_str(), file))
        .collect::<std::collections::BTreeMap<_, _>>();
    let query_lower = query.to_ascii_lowercase();
    let matches = map
        .tags
        .iter()
        .filter(|tag| tag.anchor == query || tag.anchor.to_ascii_lowercase().contains(&query_lower))
        .collect::<Vec<_>>();

    println!("anchor-range: {query}");
    if matches.is_empty() {
        println!("no anchors matched");
        return Ok(());
    }

    for tag in matches.into_iter().take(limit) {
        let Some(file) = files.get(tag.file_id.as_str()) else {
            continue;
        };
        let path = file.path.to_string_lossy().replace('\\', "/");
        let text = fs::read_to_string(root.join(&file.path)).unwrap_or_default();
        let end_line = if let Some(to) = to {
            find_anchor_line(&text, to).unwrap_or(tag.line)
        } else {
            next_anchor_line(&text, tag.line).unwrap_or(file.lines)
        };
        println!("path: {path}");
        println!("start_anchor: {}", tag.anchor);
        if let Some(to) = to {
            println!("end_anchor: {to}");
        }
        println!("start_line: {}", tag.line);
        println!("end_line: {end_line}");
        println!("hash: {}", file.hash);
        println!("expected_hash: {}", file.hash);
    }

    Ok(())
}

fn find_anchor_line(text: &str, anchor: &str) -> Option<usize> {
    text.lines()
        .enumerate()
        .find(|(_, line)| line.contains(anchor))
        .map(|(index, _)| index + 1)
}

fn next_anchor_line(text: &str, start_line: usize) -> Option<usize> {
    text.lines()
        .enumerate()
        .skip(start_line)
        .find(|(_, line)| line.contains("@codemap"))
        .map(|(index, _)| index + 1)
}
