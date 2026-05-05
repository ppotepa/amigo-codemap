use anyhow::Result;
use std::path::Path;

use crate::model::CodeMap;

use super::common::{
    find_file_by_path, read_text_at_root, relative_module_from_paths, resolve_relative_import,
    slash_path,
};
use super::imports::parse_imports;
use super::model::{render_report, FileOpReport, NextAction, Risk, RiskLevel};

pub fn print_file_move_plan(
    root: &Path,
    map: &CodeMap,
    query: &str,
    to: &Path,
    limit: usize,
) -> Result<()> {
    let report = build_file_move_plan_report(root, map, query, to, limit)?;
    print!("{}", render_report(&report));
    Ok(())
}

fn build_file_move_plan_report(
    root: &Path,
    map: &CodeMap,
    query: &str,
    to: &Path,
    limit: usize,
) -> Result<FileOpReport> {
    let from = find_file_by_path(map, query)
        .ok_or_else(|| anyhow::anyhow!("file-move-plan requires a real file path"))?;
    let text = read_text_at_root(root, &from.path)?;
    let out_to = if to.is_absolute() {
        to.to_path_buf()
    } else {
        root.join(to)
    };
    let to_relative = out_to
        .strip_prefix(root)
        .map_or_else(|_| out_to.clone(), Path::to_path_buf);
    let to_display = slash_path(&to_relative);

    let imports = parse_imports(&from.path, &text);
    let mut findings = Vec::new();
    findings.push("imports to rewrite:".to_string());
    let mut rewrite_count = 0usize;

    for import in imports.iter().filter(|item| item.specifier.starts_with('.')) {
        if rewrite_count >= limit {
            break;
        }
        if let Some(resolved) = resolve_relative_import(root, &from.path, &import.specifier) {
            let resolved_abs = root.join(&resolved);
            if let Some(new_specifier) =
                relative_module_from_paths(root, &to_relative, &resolved_abs)
            {
                findings.push(format!("  {} -> {}", import.specifier, new_specifier));
                rewrite_count += 1;
            }
        }
    }

    findings.push("inbound imports to update:".to_string());
    let mut inbound_count = 0usize;
    for other in &map.files {
        if other.id == from.id {
            continue;
        }
        let other_text = read_text_at_root(root, &other.path).unwrap_or_default();
        for import in parse_imports(&other.path, &other_text) {
            if !import.specifier.starts_with('.') {
                continue;
            }
            if let Some(resolved) = resolve_relative_import(root, &other.path, &import.specifier) {
                if resolved == from.path && inbound_count < limit {
                    let new_specifier = relative_module_from_paths(root, &other.path, &out_to)
                        .unwrap_or_else(|| import.specifier.clone());
                    findings.push(format!(
                        "  {}:{} {} -> {}",
                        slash_path(&other.path),
                        import.line,
                        import.specifier,
                        new_specifier
                    ));
                    inbound_count += 1;
                }
            }
        }
    }

    let mut risks = vec![Risk {
        level: RiskLevel::Medium,
        message: "relative path depth may change imports".to_string(),
    }];
    if slash_path(&from.path).contains("/features/") || to_display.contains("/features/") {
        risks.push(Risk {
            level: RiskLevel::Medium,
            message: "feature boundary imports should be verified".to_string(),
        });
    }
    if findings.iter().any(|line| line.matches("../").count() >= 3) {
        risks.push(Risk {
            level: RiskLevel::Medium,
            message: "deep relative imports increase rewrite risk".to_string(),
        });
    }
    if inbound_count > 0 {
        risks.push(Risk {
            level: RiskLevel::Low,
            message: "inbound imports need synchronized updates".to_string(),
        });
    }

    Ok(FileOpReport {
        task: format!("file-move-plan {}", query),
        scope: vec![
            format!("from: {}", slash_path(&from.path)),
            format!("to: {to_display}"),
            format!("lines: {}", from.lines),
        ],
        findings,
        risks,
        verify: vec!["npm run build".to_string()],
        next: vec![
            NextAction {
                label: "move file".to_string(),
            },
            NextAction {
                label: "update rewritten and inbound imports".to_string(),
            },
            NextAction {
                label: "run verify-plan".to_string(),
            },
        ],
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use crate::model::{CodeMap, FileEntry, GitInfo};

    use super::build_file_move_plan_report;

    #[test]
    fn snapshot_file_move_plan() {
        let root = temp_root("file-move");
        std::fs::create_dir_all(root.join("crates/apps/amigo-editor/src/assets"))
            .expect("create assets dir");
        std::fs::create_dir_all(root.join("crates/apps/amigo-editor/src/features/assets"))
            .expect("create features assets dir");
        std::fs::create_dir_all(root.join("crates/apps/amigo-editor/src/api"))
            .expect("create api dir");
        std::fs::write(
            root.join("crates/apps/amigo-editor/src/assets/AssetTreePanel.tsx"),
            "import { dto } from \"../api/dto\";\nexport const AssetTreePanel = () => null;\n",
        )
        .expect("write source file");
        std::fs::write(
            root.join("crates/apps/amigo-editor/src/features/assets/AssetBrowserPanel.tsx"),
            "import { AssetTreePanel } from \"../../assets/AssetTreePanel\";\n",
        )
        .expect("write inbound file");
        std::fs::write(
            root.join("crates/apps/amigo-editor/src/api/dto.ts"),
            "export type Dto = string;\n",
        )
        .expect("write dto file");

        let map = CodeMap {
            root_name: "repo".to_string(),
            stats: BTreeMap::new(),
            files: vec![
                FileEntry {
                    id: "f1".to_string(),
                    path: PathBuf::from("crates/apps/amigo-editor/src/assets/AssetTreePanel.tsx"),
                    language: "tsx".to_string(),
                    lines: 2,
                    hash: String::new(),
                    size: 0,
                },
                FileEntry {
                    id: "f2".to_string(),
                    path: PathBuf::from(
                        "crates/apps/amigo-editor/src/features/assets/AssetBrowserPanel.tsx",
                    ),
                    language: "tsx".to_string(),
                    lines: 1,
                    hash: String::new(),
                    size: 0,
                },
                FileEntry {
                    id: "f3".to_string(),
                    path: PathBuf::from("crates/apps/amigo-editor/src/api/dto.ts"),
                    language: "ts".to_string(),
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
                dirty: true,
                changed: Vec::new(),
            },
        };

        let report = build_file_move_plan_report(
            root.as_path(),
            &map,
            "crates/apps/amigo-editor/src/assets/AssetTreePanel.tsx",
            PathBuf::from("crates/apps/amigo-editor/src/features/assets/AssetTreePanel.tsx")
                .as_path(),
            10,
        )
        .expect("move plan should build");
        assert_eq!(
            crate::report::file_ops::model::render_report(&report).trim(),
            include_str!("../../../tests/snapshots/file_move_plan.snap").trim()
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
