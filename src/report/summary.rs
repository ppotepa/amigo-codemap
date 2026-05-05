use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;
use regex::Regex;

use crate::model::CodeMap;

use super::common::{group_path, print_next, slash_path, sorted_counts};
use super::verify_plan::plan_for_map;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationEntry {
    pub task: String,
    pub tokens_used: usize,
    pub text: String,
}

pub fn parse_operations(text: &str) -> Vec<OperationEntry> {
    let token_re =
        Regex::new(r"(?i)tokens:\s*(?:used\s*)?~?([0-9]+)|tokens(?: used)?:\s*~?([0-9]+)").unwrap();
    text.split("\n### ")
        .filter_map(|section| {
            let trimmed = section.trim();
            if trimmed.is_empty() {
                return None;
            }
            let task = trimmed
                .lines()
                .next()
                .unwrap_or("unknown")
                .trim_matches('#')
                .trim()
                .to_string();
            let tokens_used = token_re
                .captures(trimmed)
                .and_then(|caps| caps.get(1).or_else(|| caps.get(2)))
                .and_then(|matched| matched.as_str().parse::<usize>().ok())
                .unwrap_or(0);
            Some(OperationEntry {
                task,
                tokens_used,
                text: trimmed.to_string(),
            })
        })
        .collect()
}

pub fn print_operations_summary(root: &std::path::Path, limit: usize) -> Result<()> {
    let text = fs::read_to_string(root.join("operations.md")).unwrap_or_default();
    let entries = parse_operations(&text);
    println!("task: operations-summary");
    println!("entries: {}", entries.len());
    println!("top costly:");
    let mut sorted = entries.clone();
    sorted.sort_by(|a, b| b.tokens_used.cmp(&a.tokens_used));
    for entry in sorted.into_iter().take(limit) {
        println!("  {}: ~{} tokens", entry.task, entry.tokens_used);
    }
    let mut patterns = BTreeMap::new();
    for entry in &entries {
        for key in [
            "rg",
            "npm build",
            "cargo test",
            "registry",
            "commands/mod.rs",
            "workspaceRuntimeServices",
        ] {
            if entry.text.contains(key) {
                *patterns.entry(key.to_string()).or_default() += 1;
            }
        }
    }
    println!("repeated patterns:");
    for (key, count) in sorted_counts(patterns).into_iter().take(limit) {
        println!("  {key}: {count}");
    }
    println!("recommended codemap features:");
    println!("  move-plan: high");
    println!("  fallout: high");
    println!("  stale: high");
    println!("  tauri-commands: high");
    println!("  service-shape: medium");
    print_next(&[
        "prioritize high repeated patterns",
        "add fixtures for expensive workflows",
    ]);
    Ok(())
}

pub fn print_commit_summary(map: &CodeMap, limit: usize) {
    println!("task: commit-summary");
    let mut groups = BTreeMap::new();
    for change in &map.git.changed {
        *groups
            .entry(group_path(&change.path, Some("feature")))
            .or_default() += 1;
    }
    println!("affected:");
    for (group, count) in sorted_counts(groups).into_iter().take(limit) {
        println!("  {group}: {count}");
    }
    println!("files:");
    for change in map.git.changed.iter().take(limit) {
        println!("  {} {}", change.status, slash_path(&change.path));
    }
    println!("verify:");
    println!("  suggested:");
    let plan = plan_for_map(map, true);
    for cmd in &plan.required {
        println!("    {cmd}");
    }
    println!("risks:");
    let mut any = false;
    for change in &map.git.changed {
        let path = slash_path(&change.path);
        if path.contains("src-tauri") {
            any = true;
            println!("  medium: Tauri command/backend risk");
        } else if path.contains("/app/store/") {
            any = true;
            println!("  medium: state migration risk");
        } else if path.contains("componentRegistry") {
            any = true;
            println!("  medium: UI registry risk");
        } else if path.ends_with("lib.rs") {
            any = true;
            println!("  medium: public API risk");
        }
    }
    if !any {
        println!("  low: path heuristics only");
    }
    println!("commit:");
    println!("  update codemap reports");
    print_next(&[
        "run suggested checks",
        "write final response from affected areas",
    ]);
}

#[cfg(test)]
mod tests {
    use super::parse_operations;

    #[test]
    fn parses_operations_entries() {
        let entries = parse_operations("### Task A\nTokens used: ~12\n### Task B\n");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].tokens_used, 12);
    }
}
