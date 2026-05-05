use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

#[derive(Debug, Clone)]
struct PatchFile {
    old_path: Option<PathBuf>,
    new_path: Option<PathBuf>,
    hunks: Vec<PatchHunk>,
}

#[derive(Debug, Clone)]
struct PatchHunk {
    old_start: usize,
    lines: Vec<HunkLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum HunkLine {
    Context(String),
    Add(String),
    Remove(String),
}

#[derive(Debug, Clone)]
struct FileApplyResult {
    path: PathBuf,
    hunks: usize,
    applies: bool,
    status: String,
}

pub fn print_patch_check(root: &Path, from: Option<&Path>, limit: usize) -> Result<()> {
    let diff = read_patch_input(from)?;
    let files = parse_unified_diff(&diff)?;
    let results = check_or_apply(root, &files, false)?;
    print_summary("patch-check", &results, false, limit);
    Ok(())
}

pub fn print_patch_apply(
    root: &Path,
    from: Option<&Path>,
    write: bool,
    limit: usize,
) -> Result<()> {
    let diff = read_patch_input(from)?;
    let files = parse_unified_diff(&diff)?;
    let results = check_or_apply(root, &files, write)?;
    print_summary("patch-apply", &results, write, limit);
    Ok(())
}

fn read_patch_input(from: Option<&Path>) -> Result<String> {
    if let Some(path) = from {
        return Ok(fs::read_to_string(path)?);
    }

    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    if input.trim().is_empty() {
        bail!("patch input is empty; pass --from patch.diff or pipe unified diff to stdin");
    }
    Ok(input)
}

fn parse_unified_diff(diff: &str) -> Result<Vec<PatchFile>> {
    let lines = diff.lines().collect::<Vec<_>>();
    let mut files = Vec::new();
    let mut index = 0usize;

    while index < lines.len() {
        if !lines[index].starts_with("diff --git ") && !lines[index].starts_with("--- ") {
            index += 1;
            continue;
        }

        let mut old_path = None;
        let mut new_path = None;
        let mut hunks = Vec::new();

        while index < lines.len() {
            let line = lines[index];
            if line.starts_with("--- ") {
                old_path = parse_diff_path(line.trim_start_matches("--- "));
                index += 1;
                if index < lines.len() && lines[index].starts_with("+++ ") {
                    new_path = parse_diff_path(lines[index].trim_start_matches("+++ "));
                    index += 1;
                }
                break;
            }
            index += 1;
        }

        while index < lines.len() {
            let line = lines[index];
            if line.starts_with("diff --git ") || line.starts_with("--- ") {
                break;
            }
            if line.starts_with("@@ ") {
                let (old_start, _old_count, _new_start, _new_count) = parse_hunk_header(line)?;
                index += 1;
                let mut hunk_lines = Vec::new();
                while index < lines.len() {
                    let hunk_line = lines[index];
                    if hunk_line.starts_with("@@ ")
                        || hunk_line.starts_with("diff --git ")
                        || hunk_line.starts_with("--- ")
                    {
                        break;
                    }
                    if hunk_line == r"\ No newline at end of file" {
                        index += 1;
                        continue;
                    }
                    let Some(first) = hunk_line.chars().next() else {
                        hunk_lines.push(HunkLine::Context(String::new()));
                        index += 1;
                        continue;
                    };
                    let content = hunk_line.get(1..).unwrap_or_default().to_string();
                    match first {
                        ' ' => hunk_lines.push(HunkLine::Context(content)),
                        '+' => hunk_lines.push(HunkLine::Add(content)),
                        '-' => hunk_lines.push(HunkLine::Remove(content)),
                        _ => bail!("unsupported hunk line `{hunk_line}`"),
                    }
                    index += 1;
                }
                hunks.push(PatchHunk {
                    old_start,
                    lines: hunk_lines,
                });
                continue;
            }
            index += 1;
        }

        if old_path.is_none() && new_path.is_none() {
            bail!("patch file entry is missing ---/+++ paths");
        }
        files.push(PatchFile {
            old_path,
            new_path,
            hunks,
        });
    }

    if files.is_empty() {
        bail!("no unified diff file entries found");
    }
    Ok(files)
}

fn parse_diff_path(value: &str) -> Option<PathBuf> {
    let path = value.split_whitespace().next().unwrap_or(value);
    if path == "/dev/null" {
        return None;
    }
    let stripped = path
        .strip_prefix("a/")
        .or_else(|| path.strip_prefix("b/"))
        .unwrap_or(path);
    Some(PathBuf::from(stripped.replace('\\', "/")))
}

fn parse_hunk_header(line: &str) -> Result<(usize, usize, usize, usize)> {
    let header = line
        .strip_prefix("@@ ")
        .and_then(|value| value.split(" @@").next())
        .ok_or_else(|| anyhow::anyhow!("invalid hunk header `{line}`"))?;
    let mut parts = header.split_whitespace();
    let old = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("invalid hunk header `{line}`"))?;
    let new = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("invalid hunk header `{line}`"))?;
    let (old_start, old_count) = parse_range(old.trim_start_matches('-'))?;
    let (new_start, new_count) = parse_range(new.trim_start_matches('+'))?;
    Ok((old_start, old_count, new_start, new_count))
}

fn parse_range(value: &str) -> Result<(usize, usize)> {
    let mut parts = value.split(',');
    let start = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing hunk range start"))?
        .parse::<usize>()?;
    let count = parts
        .next()
        .map(str::parse::<usize>)
        .transpose()?
        .unwrap_or(1);
    Ok((start, count))
}

fn check_or_apply(root: &Path, files: &[PatchFile], write: bool) -> Result<Vec<FileApplyResult>> {
    let mut results = Vec::new();
    for file in files {
        let relative = file
            .new_path
            .as_ref()
            .or(file.old_path.as_ref())
            .ok_or_else(|| anyhow::anyhow!("delete-only patches are not supported"))?;
        let target = safe_join(root, relative)?;
        let existing = fs::read_to_string(&target).unwrap_or_default();
        let original_exists = target.exists();
        let old_lines = split_lines(&existing);

        if !original_exists && file.old_path.is_some() {
            results.push(FileApplyResult {
                path: relative.clone(),
                hunks: file.hunks.len(),
                applies: false,
                status: "target missing".to_string(),
            });
            continue;
        }

        let applied = apply_hunks(&old_lines, &file.hunks);
        match applied {
            Ok(new_lines) => {
                if write {
                    if let Some(parent) = target.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&target, join_lines(&new_lines))?;
                }
                results.push(FileApplyResult {
                    path: relative.clone(),
                    hunks: file.hunks.len(),
                    applies: true,
                    status: if write { "written" } else { "dry-run ok" }.to_string(),
                });
            }
            Err(error) => results.push(FileApplyResult {
                path: relative.clone(),
                hunks: file.hunks.len(),
                applies: false,
                status: error,
            }),
        }
    }
    Ok(results)
}

fn safe_join(root: &Path, relative: &Path) -> Result<PathBuf> {
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        bail!("unsafe patch path `{}`", relative.display());
    }
    Ok(root.join(relative))
}

fn split_lines(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    text.replace("\r\n", "\n")
        .trim_end_matches('\n')
        .split('\n')
        .map(str::to_string)
        .collect()
}

fn join_lines(lines: &[String]) -> String {
    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

fn apply_hunks(
    original: &[String],
    hunks: &[PatchHunk],
) -> std::result::Result<Vec<String>, String> {
    let mut lines = original.to_vec();
    let mut offset: isize = 0;
    for (index, hunk) in hunks.iter().enumerate() {
        let expected = hunk
            .lines
            .iter()
            .filter_map(|line| match line {
                HunkLine::Context(value) | HunkLine::Remove(value) => Some(value.clone()),
                HunkLine::Add(_) => None,
            })
            .collect::<Vec<_>>();
        let replacement = hunk
            .lines
            .iter()
            .filter_map(|line| match line {
                HunkLine::Context(value) | HunkLine::Add(value) => Some(value.clone()),
                HunkLine::Remove(_) => None,
            })
            .collect::<Vec<_>>();

        let hinted = (hunk.old_start.saturating_sub(1) as isize + offset).max(0) as usize;
        let Some(start) = find_hunk_position(&lines, &expected, hinted) else {
            return Err(format!(
                "hunk {} context mismatch near line {}",
                index + 1,
                hunk.old_start
            ));
        };
        lines.splice(start..start + expected.len(), replacement);
        offset += hunk_new_len(hunk) as isize - hunk_old_len(hunk) as isize;
    }
    Ok(lines)
}

fn find_hunk_position(lines: &[String], expected: &[String], hinted: usize) -> Option<usize> {
    if expected.is_empty() {
        return Some(hinted.min(lines.len()));
    }
    if matches_at(lines, expected, hinted) {
        return Some(hinted);
    }
    (0..=lines.len().saturating_sub(expected.len()))
        .find(|candidate| matches_at(lines, expected, *candidate))
}

fn matches_at(lines: &[String], expected: &[String], start: usize) -> bool {
    start + expected.len() <= lines.len()
        && lines[start..start + expected.len()]
            .iter()
            .zip(expected)
            .all(|(left, right)| left == right)
}

fn hunk_old_len(hunk: &PatchHunk) -> usize {
    hunk.lines
        .iter()
        .filter(|line| !matches!(line, HunkLine::Add(_)))
        .count()
}

fn hunk_new_len(hunk: &PatchHunk) -> usize {
    hunk.lines
        .iter()
        .filter(|line| !matches!(line, HunkLine::Remove(_)))
        .count()
}

fn print_summary(task: &str, results: &[FileApplyResult], write: bool, limit: usize) {
    let applied = results.iter().filter(|result| result.applies).count();
    let failed = results.len().saturating_sub(applied);
    println!("task: {task}");
    println!("mode: {}", if write { "write" } else { "dry-run" });
    println!("files: {}", results.len());
    println!("applies: {}", if failed == 0 { "yes" } else { "no" });
    println!("summary:");
    println!("  ok: {applied}");
    println!("  failed: {failed}");
    println!("files:");
    for result in results.iter().take(limit.max(1)) {
        println!(
            "  {} [{} hunks, {}]",
            result.path.to_string_lossy().replace('\\', "/"),
            result.hunks,
            result.status
        );
    }
    if results.len() > limit {
        println!("  ... {} more", results.len() - limit);
    }
    println!("next:");
    if failed == 0 && !write {
        println!("  1. rerun patch-apply with --write to modify files");
    } else if failed == 0 {
        println!("  1. run the smallest affected verify command");
    } else {
        println!("  1. inspect failed hunks and use slice/open-set for nearby anchors");
    }
}

#[cfg(test)]
mod tests {
    use super::{HunkLine, apply_hunks, parse_unified_diff};

    #[test]
    fn parses_unified_diff_file_and_hunk() {
        let diff = "diff --git a/demo.txt b/demo.txt\n--- a/demo.txt\n+++ b/demo.txt\n@@ -1,2 +1,2 @@\n a\n-old\n+new\n";
        let files = parse_unified_diff(diff).expect("diff should parse");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].hunks.len(), 1);
        assert_eq!(
            files[0].new_path.as_ref().unwrap().to_string_lossy(),
            "demo.txt"
        );
        assert_eq!(
            files[0].hunks[0].lines[1],
            HunkLine::Remove("old".to_string())
        );
    }

    #[test]
    fn applies_exact_hunk() {
        let diff = "diff --git a/demo.txt b/demo.txt\n--- a/demo.txt\n+++ b/demo.txt\n@@ -1,3 +1,3 @@\n a\n-old\n+new\n c\n";
        let files = parse_unified_diff(diff).expect("diff should parse");
        let original = vec!["a".to_string(), "old".to_string(), "c".to_string()];
        let updated = apply_hunks(&original, &files[0].hunks).expect("hunk should apply");
        assert_eq!(updated, vec!["a", "new", "c"]);
    }

    #[test]
    fn reports_context_mismatch() {
        let diff = "diff --git a/demo.txt b/demo.txt\n--- a/demo.txt\n+++ b/demo.txt\n@@ -1,2 +1,2 @@\n missing\n-old\n+new\n";
        let files = parse_unified_diff(diff).expect("diff should parse");
        let original = vec!["a".to_string(), "old".to_string()];
        let error = apply_hunks(&original, &files[0].hunks).expect_err("hunk should fail");
        assert!(error.contains("context mismatch"));
    }
}
