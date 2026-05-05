use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;

use crate::model::CodeMap;
use crate::report::common::slash_path;

use super::common::read_text_at_root;
use super::model::{render_report, FileOpReport, NextAction, Risk, RiskLevel};

pub fn print_orphan_files(root: &Path, map: &CodeMap, query: &str, limit: usize) -> Result<()> {
    let report = build_orphan_report(root, map, query, limit)?;
    print!("{}", render_report(&report));
    Ok(())
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn render_orphan_report(root: &Path, map: &CodeMap, query: &str, limit: usize) -> Result<String> {
    let report = build_orphan_report(root, map, query, limit)?;
    Ok(render_report(&report))
}

fn build_orphan_report(root: &Path, map: &CodeMap, query: &str, limit: usize) -> Result<FileOpReport> {
    let prefix = query.replace('\\', "/");
    let scope = vec![format!("scope: {query}")];
    let mut findings = Vec::new();
    let mut candidates = Vec::new();
    let mut not_orphan = Vec::new();

    let inbound_counts = inbound_counts(map);
    let repo_text_refs = build_textual_ref_counts(root, map);

    for file in &map.files {
        let path = slash_path(&file.path);
        if !path.starts_with(&prefix) {
            continue;
        }
        if is_entry_point(&path) {
            continue;
        }

        let text = read_text_at_root(root, &file.path).unwrap_or_default();
        let basename = file
            .path
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
            .unwrap_or_default();
        let path_refs = repo_text_refs
            .get(&path)
            .copied()
            .unwrap_or_else(|| textual_path_refs(root, map, &path, &basename));
        let inbound = inbound_counts.get(&file.id).copied().unwrap_or(0) + path_refs;

        if inbound == 0 {
            let status = if classify_shim(&text).is_some() {
                "shim/empty"
            } else {
                "candidate orphan"
            };
            candidates.push((path, file.lines, inbound, status.to_string()));
        } else if not_orphan.len() < limit {
            not_orphan.push((path, inbound));
        }
    }

    findings.push("candidates:".to_string());
    if candidates.is_empty() {
        findings.push("  none".to_string());
    } else {
        for (path, lines, inbound, status) in candidates.iter().take(limit) {
            findings.push(format!("  {path}"));
            findings.push(format!("    lines: {lines}"));
            findings.push(format!("    inbound refs: {inbound}"));
            findings.push(format!("    status: {status}"));
        }
    }

    findings.push("not orphan:".to_string());
    if not_orphan.is_empty() {
        findings.push("  none".to_string());
    } else {
        for (path, inbound) in not_orphan {
            findings.push(format!("  {path}"));
            findings.push(format!("    inbound refs: {inbound}"));
        }
    }

    let mut risks = Vec::new();
    if !candidates.is_empty() {
        risks.push(Risk {
            level: RiskLevel::Medium,
            message: "some zero-inbound files may be compatibility shims".to_string(),
        });
    }

    Ok(FileOpReport {
        task: format!("orphan-files {query}"),
        scope,
        findings,
        risks,
        verify: vec!["npm run build".to_string()],
        next: vec![
            NextAction {
                label: "inspect shim/empty candidates first".to_string(),
            },
            NextAction {
                label: "delete orphan files only after verifying inbound refs".to_string(),
            },
        ],
    })
}

fn inbound_counts(map: &CodeMap) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for dep in &map.dependencies {
        *counts.entry(dep.to.clone()).or_default() += 1;
    }
    counts
}

fn build_textual_ref_counts(root: &Path, map: &CodeMap) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for file in &map.files {
        let path = slash_path(&file.path);
        if is_entry_point(&path) {
            continue;
        }
        let basename = file
            .path
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
            .unwrap_or_default();
        let count = textual_path_refs(root, map, &path, &basename);
        counts.insert(path, count);
    }
    counts
}

fn textual_path_refs(root: &Path, map: &CodeMap, path: &str, basename: &str) -> usize {
    let normalized = path.replace('\\', "/");
    let filename = Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut refs = 0usize;

    for other in &map.files {
        let other_path = slash_path(&other.path);
        if other_path == normalized {
            continue;
        }

        let text = read_text_at_root(root, &other.path).unwrap_or_default();
        if text.contains(basename)
            || (!filename.is_empty() && text.contains(&filename))
            || text.contains(path)
        {
            refs += 1;
        }
    }

    refs
}

fn is_entry_point(path: &str) -> bool {
    path.ends_with("/main.rs")
        || path.ends_with("/lib.rs")
        || path.ends_with("/mod.rs")
        || path.ends_with("/main.tsx")
        || path.ends_with("/index.ts")
        || path.ends_with("/index.tsx")
        || path.contains("/tests/")
}

fn classify_shim(text: &str) -> Option<&'static str> {
    let lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();

    if lines.len() > 5 {
        return None;
    }

    if lines.is_empty() {
        return Some("empty");
    }

    if lines.iter().all(|line| {
        line.starts_with("export ")
            || line.starts_with("pub use ")
            || line.starts_with("pub mod ")
            || line.starts_with("mod ")
    }) {
        return Some("re-export/mod shim");
    }

    None
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use crate::model::{CodeMap, DependencyEntry, FileEntry, GitInfo};

    use super::{classify_shim, inbound_counts, is_entry_point, render_orphan_report, textual_path_refs};

    #[test]
    fn ignores_entrypoints() {
        assert!(is_entry_point("src/main.rs"));
        assert!(is_entry_point("src/index.ts"));
        assert!(is_entry_point("src/tests/foo.rs"));
    }

    #[test]
    fn recognizes_shim() {
        assert_eq!(
            classify_shim(r#"export { Thing } from "./thing";"#),
            Some("re-export/mod shim")
        );
    }

    #[test]
    fn counts_dependency_inbound_refs() {
        let map = CodeMap {
            root_name: "repo".to_string(),
            stats: BTreeMap::new(),
            files: Vec::new(),
            packages: Vec::new(),
            symbols: Vec::new(),
            dependencies: vec![DependencyEntry {
                from: "a".to_string(),
                to: "b".to_string(),
                kind: "imports".to_string(),
            }],
            areas: Vec::new(),
            git: GitInfo {
                branch: "main".to_string(),
                rev: "abc".to_string(),
                dirty: false,
                changed: Vec::new(),
            },
        };
        let counts = inbound_counts(&map);
        assert_eq!(counts.get("b"), Some(&1));
    }

    #[test]
    fn finds_textual_refs_outside_git_changes() {
        let root = temp_root("orphan");
        std::fs::create_dir_all(root.join("src/features/assets")).expect("create dirs");
        std::fs::write(
            root.join("src/features/assets/AssetTreePanel.tsx"),
            "export const AssetTreePanel = () => null;\n",
        )
        .expect("write panel");
        std::fs::write(
            root.join("src/features/assets/AssetBrowserPanel.tsx"),
            "import { AssetTreePanel } from \"./AssetTreePanel\";\n",
        )
        .expect("write browser");

        let map = CodeMap {
            root_name: "repo".to_string(),
            stats: BTreeMap::new(),
            files: vec![
                FileEntry {
                    id: "f1".to_string(),
                    path: PathBuf::from("src/features/assets/AssetTreePanel.tsx"),
                    language: "tsx".to_string(),
                    lines: 1,
                    hash: String::new(),
                    size: 0,
                },
                FileEntry {
                    id: "f2".to_string(),
                    path: PathBuf::from("src/features/assets/AssetBrowserPanel.tsx"),
                    language: "tsx".to_string(),
                    lines: 1,
                    hash: String::new(),
                    size: 0,
                },
            ],
            packages: Vec::new(),
            symbols: Vec::new(),
            dependencies: Vec::new(),
            areas: Vec::new(),
            git: GitInfo {
                branch: "main".to_string(),
                rev: "abc".to_string(),
                dirty: false,
                changed: Vec::new(),
            },
        };

        let refs = textual_path_refs(
            root.as_path(),
            &map,
            "src/features/assets/AssetTreePanel.tsx",
            "AssetTreePanel",
        );
        assert_eq!(refs, 1);
    }

    #[test]
    fn snapshot_orphan_report() {
        let root = temp_root("orphan-snapshot");
        std::fs::create_dir_all(root.join("src/features/assets")).expect("create dirs");
        std::fs::write(
            root.join("src/features/assets/AssetRegistryTree.tsx"),
            "export { AssetBrowserPanel } from \"./AssetBrowserPanel\";\n",
        )
        .expect("write shim");
        std::fs::write(
            root.join("src/features/assets/AssetBrowserPanel.tsx"),
            "import { AssetTreePanel } from \"./AssetTreePanel\";\n",
        )
        .expect("write browser");
        std::fs::write(
            root.join("src/features/assets/AssetTreePanel.tsx"),
            "export const AssetTreePanel = () => null;\n",
        )
        .expect("write panel");

        let map = CodeMap {
            root_name: "repo".to_string(),
            stats: BTreeMap::new(),
            files: vec![
                FileEntry {
                    id: "f1".to_string(),
                    path: PathBuf::from("src/features/assets/AssetRegistryTree.tsx"),
                    language: "tsx".to_string(),
                    lines: 1,
                    hash: String::new(),
                    size: 0,
                },
                FileEntry {
                    id: "f2".to_string(),
                    path: PathBuf::from("src/features/assets/AssetBrowserPanel.tsx"),
                    language: "tsx".to_string(),
                    lines: 1,
                    hash: String::new(),
                    size: 0,
                },
                FileEntry {
                    id: "f3".to_string(),
                    path: PathBuf::from("src/features/assets/AssetTreePanel.tsx"),
                    language: "tsx".to_string(),
                    lines: 1,
                    hash: String::new(),
                    size: 0,
                },
            ],
            packages: Vec::new(),
            symbols: Vec::new(),
            dependencies: Vec::new(),
            areas: Vec::new(),
            git: GitInfo {
                branch: "main".to_string(),
                rev: "abc".to_string(),
                dirty: false,
                changed: Vec::new(),
            },
        };

        assert_eq!(
            render_orphan_report(root.as_path(), &map, "src/features", 10)
                .expect("report should render")
                .trim(),
            include_str!("../../../tests/snapshots/orphan_files.snap").trim()
        );
    }

    fn temp_root(name: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time should advance")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("amigo-codemap-{name}-{unique}"));
        std::fs::create_dir_all(&root).expect("create temp root");
        root
    }
}
