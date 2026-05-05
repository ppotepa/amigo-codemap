use std::fs;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::model::CodeMap;

use super::model::{FileOpReport, NextAction};

#[derive(Debug, Serialize, Deserialize)]
pub struct Workset {
    pub name: String,
    pub query: String,
    pub task: Option<String>,
    pub git_rev: String,
    pub files: Vec<WorksetFile>,
    pub symbols: Vec<String>,
    pub checks: Vec<WorksetCheck>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorksetFile {
    pub path: String,
    pub status: String,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorksetCheck {
    pub command: String,
    pub status: String,
}

pub fn print_workset(
    root: &Path,
    map: &CodeMap,
    name: &str,
    task: Option<&str>,
    save: bool,
    status: bool,
) -> Result<()> {
    let dir = root.join(".amigo").join("worksets");
    let workset_path = dir.join(format!("{name}.json"));
    let mut scope = vec![format!("name: {name}")];
    let mut findings = Vec::new();
    let mut verify = Vec::new();
    let has_saved_workset = workset_path.exists();

    let files = map
        .git
        .changed
        .iter()
        .map(|change| change.path.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    let checks = vec![
        WorksetCheck {
            command: "verify-plan --changed".to_string(),
            status: "pending".to_string(),
        },
        WorksetCheck {
            command: "npm run build".to_string(),
            status: "pending".to_string(),
        },
    ];

    if status && has_saved_workset {
        scope.push("mode: status".to_string());
        if let Ok(text) = fs::read_to_string(&workset_path) {
            if let Ok(stored) = serde_json::from_str::<Workset>(&text) {
                findings.push(format!("files: {}", stored.files.len()));
                for item in stored.files {
                    findings.push(format!("  {} {}", item.status, item.path));
                }
                verify.push("load status and run pending checks".to_string());
            } else {
                findings.push("corrupt workset".to_string());
            }
        } else {
            findings.push("workset unreadable".to_string());
        }
    } else if status {
        scope.push("mode: status".to_string());
        findings.push(format!("missing workset: {}", workset_path.display()));
        verify.push("save workset before checking status".to_string());
    } else {
        findings.push("files:".to_string());
        for file in files.iter().take(60) {
            findings.push(format!("  {file}"));
        }
        findings.push(format!("symbols: {}", map.symbols.len()));
    }

    let workset = Workset {
        name: name.to_string(),
        query: name.to_string(),
        task: task.map(ToString::to_string),
        git_rev: map.git.rev.clone(),
        files: map
            .git
            .changed
            .iter()
            .map(|change| WorksetFile {
                path: change.path.to_string_lossy().to_string(),
                status: if change.status == "M" {
                    "changed".to_string()
                } else {
                    change.status.clone()
                },
                reason: "changed in git".to_string(),
            })
            .collect(),
        symbols: map
            .symbols
            .iter()
            .filter(|symbol| symbol.name.contains(name))
            .map(|symbol| symbol.name.clone())
            .collect(),
        checks,
    };

    if save {
        fs::create_dir_all(&dir)?;
        fs::write(&workset_path, serde_json::to_vec_pretty(&workset)?)?;
        findings.push(format!("saved: {}", workset_path.display()));
    }

    super::model::print_report(&FileOpReport {
        task: format!("workset {name}"),
        scope,
        findings,
        risks: Vec::new(),
        verify: if verify.is_empty() {
            vec!["none".to_string()]
        } else {
            verify
        },
        next: vec![
            NextAction {
                label: if has_saved_workset || save {
                    "run pending checks".to_string()
                } else {
                    "save workset before tracking status".to_string()
                },
            },
            NextAction {
                label: if has_saved_workset || save {
                    "finish all changed files".to_string()
                } else {
                    "rerun with --save to persist current scope".to_string()
                },
            },
        ],
    });

    Ok(())
}
