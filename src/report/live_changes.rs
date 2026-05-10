use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

use super::common::sorted_counts;

#[derive(Debug, Clone, PartialEq, Eq)]
struct LiveGitChange {
    status: String,
    path: PathBuf,
    original_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct LiveGitSummary {
    branch: String,
    rev: String,
    shortstat: String,
    changes: Vec<LiveGitChange>,
    warnings: Vec<String>,
}

pub fn print_changes(
    root: &Path,
    group: Option<&str>,
    limit: usize,
    compact: bool,
    hide_generated: bool,
    warnings_only: bool,
) -> Result<()> {
    let summary = read_live_git_summary(root)?;

    if warnings_only {
        print_warnings(&summary);
        return Ok(());
    }

    let visible = visible_changes(&summary.changes, hide_generated);
    println!(
        "changes: {} files{}",
        visible.len(),
        shortstat_suffix(&summary.shortstat)
    );
    println!(
        "git: branch={} rev={}",
        empty_dash(&summary.branch),
        empty_dash(&summary.rev)
    );

    if let Some(group) = group {
        print_grouped_changes(&visible, group, limit);
    } else if compact {
        print_compact_changes(&summary, &visible, limit, hide_generated);
    } else {
        print_full_changes(&summary, &visible, limit, hide_generated);
    }

    print_warnings(&summary);
    print_changes_next();
    Ok(())
}

pub fn print_commit_plan(root: &Path, limit: usize, compact: bool) -> Result<()> {
    let summary = read_live_git_summary(root)?;
    let visible = visible_changes(&summary.changes, false);
    let mut groups = BTreeMap::<String, Vec<&LiveGitChange>>::new();

    for change in &visible {
        groups
            .entry(commit_bucket(&slash_path(&change.path)).to_string())
            .or_default()
            .push(change);
    }

    println!("task: commit-plan");
    println!(
        "scope: {} files{}",
        visible.len(),
        shortstat_suffix(&summary.shortstat)
    );

    if groups.is_empty() {
        println!("commits:");
        println!("  none");
        return Ok(());
    }

    println!("commits:");
    for (bucket, files) in groups.iter().take(limit) {
        println!("  {bucket} ({} files)", files.len());
        for file in files.iter().take(if compact { 4 } else { limit }) {
            println!("    {} {}", file.status, slash_path(&file.path));
        }
        if compact && files.len() > 4 {
            println!("    ... {} more", files.len() - 4);
        }
    }

    println!("verify:");
    for command in suggested_verify(&visible) {
        println!("  {command}");
    }

    print_warnings(&summary);
    println!("next:");
    println!("  1. review each commit group");
    println!("  2. run verify commands");
    println!("  3. commit generated files with the code/docs that produced them");
    Ok(())
}

fn read_live_git_summary(root: &Path) -> Result<LiveGitSummary> {
    let branch = git_output(root, &["branch", "--show-current"])?;
    let rev = git_output(root, &["rev-parse", "--short", "HEAD"])?;
    let status = git_output(root, &["status", "--porcelain=v1", "--untracked-files=all"])?;
    let shortstat = git_output(root, &["diff", "--shortstat", "HEAD"])?;
    let autocrlf = git_output(root, &["config", "--get", "core.autocrlf"]).unwrap_or_default();

    let changes = parse_porcelain_status(&status);
    let mut warnings = Vec::new();

    if changes.iter().any(|change| is_submodule_path(&change.path)) {
        warnings.push("submodule: one or more submodules changed or dirty".to_string());
    }

    if changes.iter().any(|change| is_generated_path(&change.path)) {
        warnings.push("generated: generated codemap/index files changed".to_string());
    }

    if matches!(autocrlf.trim(), "true" | "input") && !root.join(".gitattributes").exists() {
        warnings.push("eol: core.autocrlf is set and .gitattributes is missing".to_string());
    }

    Ok(LiveGitSummary {
        branch,
        rev,
        shortstat,
        changes,
        warnings,
    })
}

fn parse_porcelain_status(status: &str) -> Vec<LiveGitChange> {
    status
        .lines()
        .filter_map(|line| {
            if line.len() < 4 {
                return None;
            }

            let status = line[..2].trim().to_string();
            let path_text = line[2..].trim().trim_matches('"');
            let (original_path, path_text) = path_text
                .split_once(" -> ")
                .map(|(from, to)| (Some(PathBuf::from(from)), to))
                .unwrap_or((None, path_text));

            Some(LiveGitChange {
                status,
                path: PathBuf::from(path_text.replace('\\', "/")),
                original_path,
            })
        })
        .collect()
}

fn visible_changes<'a>(
    changes: &'a [LiveGitChange],
    hide_generated: bool,
) -> Vec<&'a LiveGitChange> {
    changes
        .iter()
        .filter(|change| !hide_generated || !is_generated_path(&change.path))
        .collect()
}

fn print_grouped_changes(changes: &[&LiveGitChange], group: &str, limit: usize) {
    let mut counts = BTreeMap::<String, usize>::new();
    for change in changes {
        let key = match group {
            "status" => change.status.clone(),
            "domain" => domain_bucket(&slash_path(&change.path)).to_string(),
            "package" => package_bucket(&slash_path(&change.path)).to_string(),
            "path" => path_bucket(&slash_path(&change.path)).to_string(),
            _ => change_bucket(&slash_path(&change.path)).to_string(),
        };
        *counts.entry(key).or_default() += 1;
    }

    println!("groups:");
    for (key, count) in sorted_counts(counts).into_iter().take(limit) {
        println!("  {key}\t{count}");
    }
}

fn print_compact_changes(
    summary: &LiveGitSummary,
    changes: &[&LiveGitChange],
    limit: usize,
    hide_generated: bool,
) {
    let mut counts = BTreeMap::<String, usize>::new();
    for change in changes {
        *counts
            .entry(change_bucket(&slash_path(&change.path)).to_string())
            .or_default() += 1;
    }

    println!("groups:");
    for (key, count) in sorted_counts(counts).into_iter().take(8) {
        println!("  {key}\t{count}");
    }

    println!("files:");
    for change in changes.iter().take(limit.min(12)) {
        println!("  {} {}", change.status, slash_path(&change.path));
    }

    let hidden_generated = if hide_generated {
        summary
            .changes
            .iter()
            .filter(|change| is_generated_path(&change.path))
            .count()
    } else {
        0
    };
    if hidden_generated > 0 {
        println!("hidden-generated: {hidden_generated}");
    }
}

fn print_full_changes(
    summary: &LiveGitSummary,
    changes: &[&LiveGitChange],
    limit: usize,
    hide_generated: bool,
) {
    print_grouped_changes(changes, "bucket", limit);

    println!("files:");
    for change in changes.iter().take(limit) {
        if let Some(original) = &change.original_path {
            println!(
                "  {} {} -> {}",
                change.status,
                slash_path(original),
                slash_path(&change.path)
            );
        } else {
            println!("  {} {}", change.status, slash_path(&change.path));
        }
    }

    let generated = summary
        .changes
        .iter()
        .filter(|change| is_generated_path(&change.path))
        .collect::<Vec<_>>();
    if !hide_generated && !generated.is_empty() {
        println!("generated:");
        for change in generated.into_iter().take(limit) {
            println!("  {} {}", change.status, slash_path(&change.path));
        }
    }
}

fn print_warnings(summary: &LiveGitSummary) {
    println!("warnings:");
    if summary.warnings.is_empty() {
        println!("  none");
        return;
    }

    for warning in &summary.warnings {
        println!("  {warning}");
    }
}

fn print_changes_next() {
    println!("next:");
    println!("  1. codemap commit-plan --compact");
    println!("  2. codemap verify-plan --changed");
    println!("  3. git diff -- <focused-files>");
}

fn suggested_verify(changes: &[&LiveGitChange]) -> Vec<&'static str> {
    let mut commands = Vec::new();
    let has_codemap = changes
        .iter()
        .any(|change| slash_path(&change.path).contains("crates/tools/amigo-codemap"));
    let has_editor = changes
        .iter()
        .any(|change| slash_path(&change.path).contains("crates/apps/amigo-editor"));
    let has_rust = changes.iter().any(|change| {
        matches!(
            change.path.extension().and_then(|ext| ext.to_str()),
            Some("rs")
        )
    });
    let has_frontend = changes.iter().any(|change| {
        matches!(
            change.path.extension().and_then(|ext| ext.to_str()),
            Some("ts" | "tsx" | "js" | "jsx" | "css")
        )
    });

    if has_codemap {
        commands.push("cargo test -p amigo-codemap");
    }
    if has_rust && !has_codemap {
        commands.push("cargo test");
    }
    if has_editor || has_frontend {
        commands.push("npm test");
        commands.push("npm run build");
    }
    if commands.is_empty() {
        commands.push("codemap changes --compact");
    }
    commands
}

fn git_output(root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .with_context(|| format!("failed to run git {}", args.join(" ")))?;

    if !output.status.success() {
        anyhow::bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn shortstat_suffix(shortstat: &str) -> String {
    if shortstat.trim().is_empty() {
        String::new()
    } else {
        format!(", {}", shortstat.trim())
    }
}

fn change_bucket(path: &str) -> &'static str {
    if is_generated_text(path) {
        "generated-index"
    } else if is_submodule_text(path) {
        "submodule"
    } else if path.contains("crates/tools/amigo-codemap") {
        "codemap-tool"
    } else if path.ends_with(".md") || path.contains("/docs/") || path == "operations.md" {
        "docs-workflow"
    } else if path.contains("crates/apps/amigo-editor") {
        "amigo-editor"
    } else if path.contains("crates/engine") || path.contains("crates/ui") {
        "engine"
    } else if path.starts_with("mods/") {
        "mods"
    } else {
        "other"
    }
}

fn commit_bucket(path: &str) -> &'static str {
    if is_generated_text(path) {
        "codemap generated index"
    } else if path.contains("crates/tools/amigo-codemap") {
        "codemap tool"
    } else if path.contains("AMIGO_WORKFLOW.md")
        || path.contains("README.md")
        || path.contains("operations.md")
    {
        "docs and workflow"
    } else if path.contains("crates/apps/amigo-editor/src-tauri") {
        "amigo editor backend"
    } else if path.contains("crates/apps/amigo-editor/src") {
        "amigo editor frontend"
    } else if path.starts_with("mods/") {
        "mods"
    } else {
        "other"
    }
}

fn domain_bucket(path: &str) -> &'static str {
    if path.contains("crates/tools/amigo-codemap") || path.contains(".amigo/codemap") {
        "codemap"
    } else if path.ends_with(".md") || path.contains("/docs/") || path == "operations.md" {
        "docs"
    } else if path.contains("editors/ui-document") {
        "ui-document"
    } else if path.contains("main-window") || path.contains("dock") {
        "workspace"
    } else if path.contains("features/scenes") {
        "scene-editor"
    } else if path.contains("src-tauri") {
        "editor-backend"
    } else if path.starts_with("mods/") {
        "mods"
    } else {
        "other"
    }
}

fn package_bucket(path: &str) -> &str {
    let parts = path.split('/').collect::<Vec<_>>();
    if parts.first() == Some(&"crates") && parts.len() >= 3 {
        if parts.get(1) == Some(&"apps") || parts.get(1) == Some(&"tools") {
            return parts[2];
        }
        return parts[1];
    }
    parts.first().copied().unwrap_or("root")
}

fn path_bucket(path: &str) -> String {
    let parts = path.split('/').collect::<Vec<_>>();
    if parts.first() == Some(&"crates") && parts.len() >= 3 {
        return parts.iter().take(3).copied().collect::<Vec<_>>().join("/");
    }
    parts.first().copied().unwrap_or("root").to_string()
}

fn is_generated_path(path: &Path) -> bool {
    is_generated_text(&slash_path(path))
}

fn is_generated_text(path: &str) -> bool {
    path.contains(".generated.")
        || path.ends_with("codemap.anchors.generated.json")
        || path.ends_with("codemap.coverage.generated.md")
        || path.ends_with("package-lock.json")
        || path.ends_with("Cargo.lock")
}

fn is_submodule_path(path: &Path) -> bool {
    is_submodule_text(&slash_path(path))
}

fn is_submodule_text(path: &str) -> bool {
    path == "crates/tools/amigo-codemap"
}

fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn empty_dash(value: &str) -> &str {
    if value.is_empty() { "-" } else { value }
}

#[cfg(test)]
mod tests {
    use super::{change_bucket, parse_porcelain_status};

    #[test]
    fn parses_porcelain_status() {
        let changes =
            parse_porcelain_status(" M src/main.rs\n?? src/new.rs\nR  old.rs -> new.rs\n");

        assert_eq!(changes.len(), 3);
        assert_eq!(changes[0].status, "M");
        assert_eq!(changes[1].path.to_string_lossy(), "src/new.rs");
        assert_eq!(
            changes[2]
                .original_path
                .as_ref()
                .map(|path| path.to_string_lossy().to_string()),
            Some("old.rs".to_string())
        );
    }

    #[test]
    fn buckets_generated_files() {
        assert_eq!(
            change_bucket(".amigo/codemap.anchors.generated.json"),
            "generated-index"
        );
    }
}
