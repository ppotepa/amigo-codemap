use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::Result;

use crate::model::CodeMap;

use super::common::{changed_by_path, changed_status_by_path, slash_path, text_refs_like};
use super::model::{FileOpReport, NextAction, Risk, RiskLevel};

pub fn print_open_set(
    root: &Path,
    map: &CodeMap,
    query: &str,
    task: Option<&str>,
    limit: usize,
) -> Result<()> {
    let changed_paths = changed_by_path(map);
    let changed_status = changed_status_by_path(map);
    let refs = text_refs_like(root, map, query, usize::MAX).unwrap_or_default();

    let mut definition_paths = BTreeSet::<String>::new();
    let mut ref_counts = BTreeMap::<String, usize>::new();

    for reference in &refs {
        let path = slash_path(&reference.path);
        *ref_counts.entry(path).or_default() += 1;
    }

    for symbol in map.symbols.iter().filter(|symbol| symbol.name == query) {
        if let Some(file) = map.files.iter().find(|file| file.id == symbol.file_id) {
            definition_paths.insert(slash_path(&file.path));
        }
    }

    let rankings = rank_open_set_items(
        &definition_paths,
        &changed_paths,
        &changed_status,
        &ref_counts,
    );

    let mut ranked: Vec<_> = rankings.into_iter().collect();
    ranked.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));

    let first_limit = limit.min(4).max(1).div_ceil(2); // stable first bucket split
    let second_limit = limit.saturating_sub(first_limit);

    let mut findings = Vec::new();
    findings.push("read first:".to_string());
    if ranked.is_empty() {
        findings.push("  no direct matches found".to_string());
    } else {
    for (path, score, reasons) in ranked.iter().take(first_limit).map(|item| (item.0.as_str(), &item.1.0, &item.1.1)) {
        findings.push(format!("  {} [score {}, {}]", path, score, reasons.join(", ")));
    }
    }

    if ranked.len() > first_limit {
        findings.push("read second:".to_string());
        for (path, score, reasons) in ranked
            .iter()
            .skip(first_limit)
            .take(second_limit)
            .map(|item| (item.0.as_str(), &item.1.0, &item.1.1))
        {
            findings.push(format!("  {} [score {}, {}]", path, score, reasons.join(", ")));
        }
    }

    let mut risks = Vec::new();
    if definition_paths.is_empty() {
        risks.push(Risk {
            level: RiskLevel::Medium,
            message: "query has no indexed definitions; widen task context".to_string(),
        });
    } else if definition_paths.iter().any(|path| changed_paths.contains(path)) {
        risks.push(Risk {
            level: RiskLevel::Low,
            message: "query definition changed; start with highest-scored files".to_string(),
        });
    } else {
        risks.push(Risk {
            level: RiskLevel::Medium,
            message: "no changed definitions; validate usage before editing".to_string(),
        });
    }

    super::model::print_report(&FileOpReport {
        task: format!("open-set {query}"),
        scope: vec![
            format!("query: {query}"),
            format!("task: {}", task.unwrap_or("read")),
        ],
        findings,
        risks,
        verify: vec![
            "npm run build".to_string(),
            "cargo test -p amigo-editor --lib".to_string(),
        ],
        next: vec![
            NextAction {
                label: "read first list".to_string(),
            },
            NextAction {
                label: "run impact for query symbol".to_string(),
            },
        ],
    });

    Ok(())
}

fn rank_open_set_items(
    definition_paths: &BTreeSet<String>,
    changed_paths: &BTreeSet<String>,
    changed_status: &BTreeMap<String, String>,
    ref_counts: &BTreeMap<String, usize>,
) -> BTreeMap<String, (i32, Vec<String>)> {
    let mut scores = BTreeMap::<String, (i32, Vec<String>)>::new();

    for (path, refs) in ref_counts {
        let is_definition = definition_paths.contains(path);
        let is_changed = changed_paths.contains(path);
        let is_boundary = path.contains("/app/store/") || path.contains("/src-tauri/");
        let is_test = is_test_path(path);
        let is_low_value = path.ends_with(".css") || path.ends_with(".scss") || path.ends_with(".snap");

        let mut score = 0i32;
        let mut reasons = Vec::new();

        if is_definition {
            score += 100;
            reasons.push("definition".to_string());
        }
        if is_changed {
            score += 80;
            reasons.push("changed".to_string());
        }
        if let Some(status) = changed_status.get(path) {
            reasons.push(format!("git:{status}"));
        }
        if *refs > 0 {
            score += (std::cmp::min(*refs, 10) * 6) as i32;
            reasons.push(format!("refs:{refs}"));
        }
        if is_boundary {
            score += 50;
            reasons.push("boundary".to_string());
        }
        if is_test {
            score += 30;
            reasons.push("test".to_string());
        }
        if is_low_value {
            score -= 50;
            reasons.push("low-value file".to_string());
        }

        scores.insert(path.clone(), (score, reasons));
    }

    if definition_paths.iter().any(|path| !ref_counts.contains_key(path)) {
        for path in definition_paths {
            scores
                .entry(path.clone())
                .and_modify(|entry| {
                    entry.0 += 100;
                    if !entry.1.iter().any(|item| item == "definition") {
                        entry.1.push("definition".to_string());
                    }
                })
                .or_insert((100, vec!["definition".to_string()]));
        }
    }

    scores
}

fn is_test_path(path: &str) -> bool {
    path.contains(".test.") || path.contains(".spec.") || path.contains("/tests/")
}

#[cfg(test)]
mod tests {
    use super::rank_open_set_items;
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn prefers_definition_and_changed_file() {
        let mut refs = BTreeMap::new();
        refs.insert("crates/apps/amigo-editor/src/app/store/main.ts".to_string(), 3);
        refs.insert("crates/apps/amigo-editor/src/app/main.ts".to_string(), 8);
        let mut defs = BTreeSet::new();
        defs.insert("crates/apps/amigo-editor/src/app/store/main.ts".to_string());
        let mut changed = BTreeSet::new();
        changed.insert("crates/apps/amigo-editor/src/app/store/main.ts".to_string());
        let mut changed_status = BTreeMap::new();
        changed_status.insert(
            "crates/apps/amigo-editor/src/app/store/main.ts".to_string(),
            "M".to_string(),
        );

        let ranked = rank_open_set_items(&defs, &changed, &changed_status, &refs);
        let first = ranked
            .get("crates/apps/amigo-editor/src/app/store/main.ts")
            .map(|(score, _)| *score)
            .unwrap_or(0);
        let second = ranked
            .get("crates/apps/amigo-editor/src/app/main.ts")
            .map(|(score, _)| *score)
            .unwrap_or(0);
        assert!(first > second);
    }
}
