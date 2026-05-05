use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::model::CodeMap;
use crate::report::common::{files_by_id, slash_path, symbols_matching, text_refs};
use crate::report::verify_plan::plan_for_paths;

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
    from_impact: Option<&str>,
    save: bool,
    status: bool,
) -> Result<()> {
    let clean_root = normalize_root(root);
    let dir = clean_root.join(".amigo").join("worksets");
    let workset_path = dir.join(format!("{name}.json"));

    if status {
        return print_workset_status(name, &workset_path);
    }

    let workset = build_workset(root, map, name, task, from_impact)?;

    let mut findings = vec!["files:".to_string()];
    for file in workset.files.iter().take(20) {
        findings.push(format!("  {} {} ({})", file.status, file.path, file.reason));
    }
    findings.push("checks:".to_string());
    for check in &workset.checks {
        findings.push(format!("  {} {}", check.command, check.status));
    }

    if save {
        fs::create_dir_all(&dir)?;
        fs::write(&workset_path, serde_json::to_vec_pretty(&workset)?)?;
        findings.push(format!("saved: {}", workset_path.display()));
    }

    super::model::print_report(&FileOpReport {
        task: format!("workset {name}"),
        scope: vec![
            format!("name: {name}"),
            format!("query: {}", workset.query),
            format!("mode: {}", if from_impact.is_some() { "impact" } else { "changed" }),
        ],
        findings,
        risks: Vec::new(),
        verify: workset
            .checks
            .iter()
            .map(|check| check.command.clone())
            .collect(),
        next: vec![
            NextAction {
                label: "save workset if you want status tracking".to_string(),
            },
            NextAction {
                label: "run pending checks from the manifest".to_string(),
            },
        ],
    });

    Ok(())
}

fn normalize_root(root: &Path) -> std::path::PathBuf {
    let text = root.to_string_lossy();
    if let Some(stripped) = text.strip_prefix(r"\\?\") {
        std::path::PathBuf::from(stripped)
    } else {
        root.to_path_buf()
    }
}

fn print_workset_status(name: &str, workset_path: &Path) -> Result<()> {
    let mut findings = Vec::new();
    let mut verify = Vec::new();

    if workset_path.exists() {
        let text = fs::read_to_string(workset_path)?;
        let stored = serde_json::from_str::<Workset>(&text)?;
        findings.push(format!("files: {}", stored.files.len()));
        for item in stored.files {
            findings.push(format!("  {} {}", item.status, item.path));
        }
        findings.push("checks:".to_string());
        for check in stored.checks {
            findings.push(format!("  {} {}", check.command, check.status));
            if check.status == "pending" {
                verify.push(check.command);
            }
        }
    } else {
        findings.push(format!("missing workset: {}", workset_path.display()));
        verify.push("save workset before checking status".to_string());
    }

    super::model::print_report(&FileOpReport {
        task: format!("workset {name}"),
        scope: vec![format!("name: {name}"), "mode: status".to_string()],
        findings,
        risks: Vec::new(),
        verify,
        next: vec![
            NextAction {
                label: "run pending checks".to_string(),
            },
            NextAction {
                label: "finish pending files".to_string(),
            },
        ],
    });

    Ok(())
}

fn build_workset(
    root: &Path,
    map: &CodeMap,
    name: &str,
    task: Option<&str>,
    from_impact: Option<&str>,
) -> Result<Workset> {
    let files = if let Some(symbol) = from_impact {
        build_impact_files(root, map, symbol)?
    } else {
        map.git
            .changed
            .iter()
            .map(|change| WorksetFile {
                path: slash_path(&change.path),
                status: if change.status == "M" {
                    "changed".to_string()
                } else {
                    "pending".to_string()
                },
                reason: "changed-in-git".to_string(),
            })
            .collect::<Vec<_>>()
    };

    let paths = files
        .iter()
        .map(|file| std::path::PathBuf::from(&file.path))
        .collect::<Vec<_>>();
    let plan = plan_for_paths(paths);
    let checks = plan
        .required
        .into_iter()
        .map(|command| WorksetCheck {
            command,
            status: "pending".to_string(),
        })
        .collect::<Vec<_>>();

    Ok(Workset {
        name: name.to_string(),
        query: from_impact.unwrap_or(name).to_string(),
        task: task.map(ToString::to_string),
        git_rev: map.git.rev.clone(),
        files,
        symbols: if let Some(symbol) = from_impact {
            vec![symbol.to_string()]
        } else {
            map.symbols
                .iter()
                .filter(|symbol| symbol.name.contains(name))
                .map(|symbol| symbol.name.clone())
                .collect()
        },
        checks,
    })
}

fn build_impact_files(root: &Path, map: &CodeMap, symbol: &str) -> Result<Vec<WorksetFile>> {
    let files = files_by_id(map);
    let changed = map
        .git
        .changed
        .iter()
        .map(|change| slash_path(&change.path))
        .collect::<std::collections::BTreeSet<_>>();
    let mut items = BTreeMap::<String, WorksetFile>::new();

    for def in symbols_matching(map, symbol) {
        if let Some(file) = files.get(def.file_id.as_str()) {
            let path = slash_path(&file.path);
            items.insert(
                path.clone(),
                WorksetFile {
                    path: path.clone(),
                    status: if changed.contains(&path) {
                        "changed".to_string()
                    } else {
                        "pending".to_string()
                    },
                    reason: "definition".to_string(),
                },
            );
        }
    }

    for reference in text_refs(root, map, symbol, usize::MAX)? {
        items.entry(reference.path.clone()).or_insert_with(|| WorksetFile {
            status: if changed.contains(&reference.path) {
                "changed".to_string()
            } else {
                "pending".to_string()
            },
            path: reference.path.clone(),
            reason: "impact-ref".to_string(),
        });
    }

    Ok(items.into_values().collect())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use crate::model::{CodeMap, FileEntry, GitChange, GitInfo, SymbolEntry};

    use super::{build_workset, normalize_root};

    fn sample_map() -> CodeMap {
        CodeMap {
            root_name: "repo".to_string(),
            stats: BTreeMap::new(),
            files: vec![FileEntry {
                id: "f1".to_string(),
                path: PathBuf::from("crates/apps/amigo-editor/src/app/selectionTypes.ts"),
                language: "ts".to_string(),
                lines: 10,
                hash: String::new(),
                size: 0,
            }],
            packages: Vec::new(),
            symbols: vec![SymbolEntry {
                name: "EditorSelectionRef".to_string(),
                kind: "type".to_string(),
                file_id: "f1".to_string(),
                line: 1,
                visibility: "export".to_string(),
            }],
            dependencies: Vec::new(),
            areas: Vec::new(),
            git: GitInfo {
                branch: "main".to_string(),
                rev: "abc".to_string(),
                dirty: true,
                changed: vec![GitChange {
                    status: "M".to_string(),
                    path: PathBuf::from("crates/apps/amigo-editor/src/app/selectionTypes.ts"),
                    file_id: Some("f1".to_string()),
                }],
            },
        }
    }

    #[test]
    fn builds_workset_from_changed_files() {
        let map = sample_map();
        let workset = build_workset(PathBuf::from(".").as_path(), &map, "selection", None, None)
            .expect("workset should build");
        assert_eq!(workset.files.len(), 1);
        assert_eq!(workset.files[0].status, "changed");
        assert!(workset.checks.iter().any(|check| !check.command.is_empty()));
    }

    #[test]
    fn builds_workset_from_impact_symbol() {
        let map = sample_map();
        let workset = build_workset(
            PathBuf::from(".").as_path(),
            &map,
            "selection-migration",
            None,
            Some("EditorSelectionRef"),
        )
        .expect("impact workset should build");
        assert_eq!(workset.query, "EditorSelectionRef");
        assert_eq!(workset.symbols, vec!["EditorSelectionRef".to_string()]);
        assert!(workset.files.iter().any(|file| file.reason == "definition"));
    }

    #[test]
    fn normalizes_windows_extended_root() {
        let normalized = normalize_root(PathBuf::from(r"\\?\D:\Git\amigo").as_path());
        assert_eq!(normalized, PathBuf::from(r"D:\Git\amigo"));
    }
}
