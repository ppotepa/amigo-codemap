use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;

use crate::model::CodeMap;
use crate::report::common::slash_path;

use super::common::read_text_at_root;
use super::model::{FileOpReport, NextAction, Risk, RiskLevel};

pub fn print_orphan_files(root: &Path, map: &CodeMap, query: &str, limit: usize) -> Result<()> {
    let prefix = query.replace('\\', "/");
    let scope = vec![format!("scope: {query}")];
    let mut findings = Vec::new();
    let mut candidates = Vec::new();
    let mut not_orphan = Vec::new();

    let inbound_counts = inbound_counts(map);

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
        let path_refs = textual_path_refs(map, &path, &basename);
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

    super::model::print_report(&FileOpReport {
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
    });
    Ok(())
}

fn inbound_counts(map: &CodeMap) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for dep in &map.dependencies {
        *counts.entry(dep.to.clone()).or_default() += 1;
    }
    counts
}

fn textual_path_refs(map: &CodeMap, path: &str, basename: &str) -> usize {
    let normalized = path.replace('\\', "/");
    let mut refs = 0usize;

    for change in &map.git.changed {
        let changed = slash_path(&change.path);
        if changed == normalized {
            continue;
        }
        if changed.contains(basename) {
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
    use super::{classify_shim, is_entry_point};

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
}
