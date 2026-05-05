use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::model::{GitChange, GitInfo};

pub fn read_git_info(root: &Path, file_ids: &BTreeMap<PathBuf, String>) -> GitInfo {
    let branch = git_output(root, &["branch", "--show-current"]).unwrap_or_default();
    let rev = git_output(root, &["rev-parse", "--short", "HEAD"]).unwrap_or_default();
    let status = git_output(root, &["status", "--short"]).unwrap_or_default();
    let changed = parse_status(&status, file_ids);

    GitInfo {
        branch,
        rev,
        dirty: !changed.is_empty(),
        changed,
    }
}

fn git_output(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn parse_status(status: &str, file_ids: &BTreeMap<PathBuf, String>) -> Vec<GitChange> {
    status
        .lines()
        .filter_map(|line| {
            if line.len() < 4 {
                return None;
            }
            let status = line[..2].trim().to_string();
            let path_text = line[2..].trim();
            let path_text = path_text
                .split(" -> ")
                .last()
                .unwrap_or(path_text)
                .trim_matches('"');
            let path = normalize_git_path(path_text);
            let file_id = file_ids.get(&path).cloned();
            Some(GitChange {
                status,
                path,
                file_id,
            })
        })
        .collect()
}

fn normalize_git_path(path: &str) -> PathBuf {
    PathBuf::from(path.replace('\\', "/"))
}
