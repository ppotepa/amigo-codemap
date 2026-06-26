use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::Result;

use crate::model::CodeMap;
use crate::query::descriptive_tokens;
use crate::report::common::{feature_group, is_codemap, is_docs, is_test_file, slash_path};

use crate::report::anchors::{anchor_entry_matches, anchor_priority_score, build_anchor_index};
use crate::taxonomy::CodemapTaxonomy;

use super::common::{changed_by_path, changed_status_by_path, text_refs_like};
use super::model::{FileOpReport, NextAction, Risk, RiskLevel, render_report};

pub fn print_open_set(
    root: &Path,
    map: &CodeMap,
    query: &str,
    task: Option<&str>,
    limit: usize,
    show_why: bool,
) -> Result<()> {
    let report = build_open_set_report(root, map, query, task, limit, show_why)?;
    print!("{}", render_report(&report));
    Ok(())
}

fn build_open_set_report(
    root: &Path,
    map: &CodeMap,
    query: &str,
    task: Option<&str>,
    limit: usize,
    show_why: bool,
) -> Result<FileOpReport> {
    let changed_paths = changed_by_path(map);
    let changed_status = changed_status_by_path(map);
    let refs = text_refs_like(root, map, query, usize::MAX).unwrap_or_default();
    let query_tokens = descriptive_tokens(query);
    let text_query_tokens = if query.split_whitespace().count() > 1 {
        open_set_text_tokens(&query_tokens)
    } else {
        Vec::new()
    };
    let anchor_query_tokens = if query.split_whitespace().count() > 1 {
        query_tokens.as_slice()
    } else {
        &[]
    };

    let mut definition_paths = BTreeSet::<String>::new();
    let mut ref_counts = BTreeMap::<String, usize>::new();
    let mut anchor_scores = BTreeMap::<String, (i32, Vec<String>)>::new();
    let mut token_scores = BTreeMap::<String, (i32, Vec<String>)>::new();
    let mut skip = BTreeSet::<String>::new();

    for symbol in map.symbols.iter().filter(|symbol| {
        symbol.name == query
            || query_tokens
                .iter()
                .any(|token| symbol_name_matches_query(&symbol.name, token))
    }) {
        if let Some(file) = map.files.iter().find(|file| file.id == symbol.file_id) {
            let path = slash_path(&file.path);
            definition_paths.insert(path.clone());
            if symbol.name != query {
                add_score(
                    &mut token_scores,
                    path,
                    70,
                    format!("symbol-token:{}", symbol.name),
                );
            }
        }
    }

    let editor_def = definition_paths
        .iter()
        .any(|path| path.starts_with("crates/apps/amigo-editor/"));

    for reference in &refs {
        let path = slash_path(&reference.path);
        if should_skip_open_set_path(&path, editor_def) {
            skip.insert(path);
            continue;
        }
        *ref_counts.entry(path).or_default() += 1;
    }

    for token in &text_query_tokens {
        for reference in text_refs_like(root, map, token, usize::MAX).unwrap_or_default() {
            let path = slash_path(&reference.path);
            if should_skip_open_set_path(&path, editor_def) {
                skip.insert(path);
                continue;
            }
            *ref_counts.entry(path.clone()).or_default() += 1;
            add_score(&mut token_scores, path, 12, format!("text-token:{token}"));
        }
    }

    let taxonomy = CodemapTaxonomy::try_load(root);
    let anchor_index = build_anchor_index(map, taxonomy.as_ref());
    for anchor in &anchor_index.anchors {
        let matched_by_query = anchor_entry_matches(anchor, query);
        let matched_by_token = anchor_query_tokens
            .iter()
            .any(|token| anchor_entry_matches(anchor, token));
        if !matched_by_query && !matched_by_token {
            continue;
        }

        let path = anchor.file.clone();
        if should_skip_open_set_path(&path, editor_def) {
            skip.insert(path);
            continue;
        }

        let mut score = anchor_priority_score(&anchor.priority);
        let mut reasons = vec![format!("anchor:{}", anchor.anchor)];

        if anchor.anchor.eq_ignore_ascii_case(query) {
            score += 100;
            reasons.push("exact-anchor".to_string());
        }
        if matched_by_token {
            score += 80;
            reasons.push("anchor-token".to_string());
        }

        reasons.push(format!("domain:{}", anchor.domain));
        reasons.push(format!("role:{}", anchor.role));

        anchor_scores
            .entry(path)
            .and_modify(|entry| {
                entry.0 += score;
                entry.1.extend(reasons.clone());
            })
            .or_insert((score, reasons));
    }

    let task_kind = task.unwrap_or("read").to_ascii_lowercase();
    let rankings = rank_open_set_items(
        &definition_paths,
        &changed_paths,
        &changed_status,
        &ref_counts,
        &anchor_scores,
        &token_scores,
        editor_def,
        &task_kind,
    );

    let mut ranked: Vec<_> = rankings.into_iter().collect();
    ranked.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));

    let recommended = ranked.len().min(limit);
    let first_limit = recommended.min(5);
    let second_limit = recommended.saturating_sub(first_limit);

    let mut findings = Vec::new();
    findings.push(format!("recommended files: {recommended}"));
    findings.push("read first:".to_string());
    if ranked.is_empty() {
        findings.push("  no direct matches found".to_string());
    } else {
        for (path, score, reasons) in ranked
            .iter()
            .take(first_limit)
            .map(|item| (item.0.as_str(), &item.1.0, &item.1.1))
        {
            findings.push(format!(
                "  {} [score {}{}]",
                path,
                score,
                if show_why {
                    format!(", {}", reasons.join(", "))
                } else {
                    String::new()
                }
            ));
        }
    }

    if second_limit > 0 {
        findings.push("read second:".to_string());
        for (path, score, reasons) in ranked
            .iter()
            .skip(first_limit)
            .take(second_limit)
            .map(|item| (item.0.as_str(), &item.1.0, &item.1.1))
        {
            findings.push(format!(
                "  {} [score {}{}]",
                path,
                score,
                if show_why {
                    format!(", {}", reasons.join(", "))
                } else {
                    String::new()
                }
            ));
        }
    }

    findings.push("skip:".to_string());
    let mut skip_items = skip.into_iter().take(6).collect::<Vec<_>>();
    skip_items.sort();
    if skip_items.is_empty() {
        findings.push("  none".to_string());
    } else {
        for item in skip_items {
            findings.push(format!("  {item}"));
        }
    }

    let mut risks = Vec::new();
    if definition_paths.is_empty() {
        risks.push(Risk {
            level: RiskLevel::Medium,
            message: "query has no indexed definitions; using secondary ranking".to_string(),
        });
    } else if definition_paths
        .iter()
        .any(|path| changed_paths.contains(path))
    {
        risks.push(Risk {
            level: RiskLevel::Low,
            message: "query definition changed; start with definition and store callers"
                .to_string(),
        });
    } else {
        risks.push(Risk {
            level: RiskLevel::Medium,
            message: "definition unchanged; validate usage before editing".to_string(),
        });
    }

    Ok(FileOpReport {
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
                label: "read first list only".to_string(),
            },
            NextAction {
                label: "make the focused change".to_string(),
            },
            NextAction {
                label: "run impact again for the edited symbol".to_string(),
            },
        ],
    })
}

fn rank_open_set_items(
    definition_paths: &BTreeSet<String>,
    changed_paths: &BTreeSet<String>,
    changed_status: &BTreeMap<String, String>,
    ref_counts: &BTreeMap<String, usize>,
    anchor_scores: &BTreeMap<String, (i32, Vec<String>)>,
    token_scores: &BTreeMap<String, (i32, Vec<String>)>,
    editor_def: bool,
    task: &str,
) -> BTreeMap<String, (i32, Vec<String>)> {
    let mut scores = BTreeMap::<String, (i32, Vec<String>)>::new();

    for (path, refs) in ref_counts {
        if should_skip_open_set_path(path, editor_def) {
            continue;
        }

        let is_definition = definition_paths.contains(path);
        let is_changed = changed_paths.contains(path);
        let group = feature_group(path);
        let is_test = is_test_path(path);
        let is_low_value = is_low_value_path(path);

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
        score += (std::cmp::min(*refs, 10) * 6) as i32;
        reasons.push(format!("refs:{refs}"));

        match group.as_str() {
            "app/store" => {
                score += 50;
                reasons.push("store boundary".to_string());
            }
            "app/selection" => {
                score += 45;
                reasons.push("selection".to_string());
            }
            "main-window" => {
                score += 35;
                reasons.push("main-window".to_string());
            }
            "feature/inspector" | "properties" => {
                score += 30;
                reasons.push("inspector/properties".to_string());
            }
            "startup" => {
                score += 25;
                reasons.push("startup".to_string());
            }
            _ => {}
        }

        if is_test {
            score -= 20;
            reasons.push("test".to_string());
        }
        if is_low_value {
            score -= 60;
            reasons.push("low-value file".to_string());
        }

        match task {
            "trace" => {
                if is_definition {
                    score += 80;
                    reasons.push("task:trace-definition".to_string());
                }
                if *refs > 0 {
                    score += (std::cmp::min(*refs, 8) * 8) as i32;
                    reasons.push("task:trace-refs".to_string());
                }
                if is_test {
                    score -= 25;
                    reasons.push("task:trace-deprioritize-test".to_string());
                }
            }
            "implement" | "migrate" | "refactor" => {
                if is_definition {
                    score += 60;
                    reasons.push("task:implement-definition".to_string());
                }
                if is_changed {
                    score += 50;
                    reasons.push("task:implement-changed".to_string());
                }
            }
            "test" => {
                if is_test {
                    score += 80;
                    reasons.push("task:test".to_string());
                }
            }
            "cleanup" => {
                if is_changed {
                    score += 60;
                    reasons.push("task:cleanup-changed".to_string());
                }
            }
            "verify" => {
                if is_changed {
                    score += 100;
                    reasons.push("task:verify-changed".to_string());
                }
                if is_test {
                    score += 35;
                    reasons.push("task:verify-test".to_string());
                }
            }
            _ => {}
        }

        scores.insert(path.clone(), (score, reasons));
    }

    for path in definition_paths {
        if should_skip_open_set_path(path, editor_def) {
            continue;
        }
        scores
            .entry(path.clone())
            .and_modify(|entry| {
                if !entry.1.iter().any(|item| item == "definition") {
                    entry.0 += 100;
                    entry.1.push("definition".to_string());
                }
            })
            .or_insert((100, vec!["definition".to_string()]));
    }

    for (path, (anchor_score, anchor_reasons)) in anchor_scores {
        if should_skip_open_set_path(path, editor_def) {
            continue;
        }

        scores
            .entry(path.clone())
            .and_modify(|entry| {
                entry.0 += *anchor_score;
                entry.1.extend(anchor_reasons.clone());
            })
            .or_insert((*anchor_score, anchor_reasons.clone()));
    }

    for (path, (token_score, token_reasons)) in token_scores {
        if should_skip_open_set_path(path, editor_def) {
            continue;
        }

        scores
            .entry(path.clone())
            .and_modify(|entry| {
                entry.0 += *token_score;
                entry.1.extend(token_reasons.clone());
            })
            .or_insert((*token_score, token_reasons.clone()));
    }

    scores
}

fn add_score(
    scores: &mut BTreeMap<String, (i32, Vec<String>)>,
    path: String,
    score: i32,
    reason: String,
) {
    scores
        .entry(path)
        .and_modify(|entry| {
            entry.0 += score;
            if !entry.1.iter().any(|item| item == &reason) {
                entry.1.push(reason.clone());
            }
        })
        .or_insert((score, vec![reason]));
}

fn symbol_name_matches_query(name: &str, query: &str) -> bool {
    name == query || normalize_identifier(name) == normalize_identifier(query)
}

fn open_set_text_tokens(tokens: &[String]) -> Vec<String> {
    tokens
        .iter()
        .filter(|token| {
            !matches!(
                token.as_str(),
                "document" | "yaml" | "tree" | "real" | "viewer" | "node"
            )
        })
        .cloned()
        .collect()
}

fn normalize_identifier(value: &str) -> String {
    value
        .chars()
        .filter(|char| char.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn should_skip_open_set_path(path: &str, editor_def: bool) -> bool {
    if is_low_value_path(path) {
        return true;
    }
    if editor_def && is_codemap(Path::new(path)) {
        return true;
    }
    false
}

fn is_low_value_path(path: &str) -> bool {
    let path = path.replace('\\', "/");
    is_docs(Path::new(&path))
        || path.ends_with(".css")
        || path.ends_with(".scss")
        || path.ends_with(".snap")
        || path.contains("package-lock.json")
        || path.ends_with("operations.md")
        || path.ends_with("AMIGO_WORKFLOW.md")
        || path.contains("/tests/fixtures/")
        || path.contains("/tests/snapshots/")
}

fn is_test_path(path: &str) -> bool {
    is_test_file(Path::new(path))
        || path.contains("/tests/fixtures/")
        || path.contains("/tests/snapshots/")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::model::{CodeMap, FileEntry, GitInfo, SymbolEntry};

    use super::{build_open_set_report, is_low_value_path, rank_open_set_items};
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn excludes_docs() {
        assert!(is_low_value_path("README.md"));
        assert!(is_low_value_path("AMIGO_WORKFLOW.md"));
        assert!(is_low_value_path(
            "crates/tools/amigo-codemap/tests/fixtures/x.txt"
        ));
    }

    #[test]
    fn prefers_definition_and_store() {
        let mut refs = BTreeMap::new();
        refs.insert(
            "crates/apps/amigo-editor/src/app/store/main.ts".to_string(),
            3,
        );
        refs.insert("crates/apps/amigo-editor/src/app/main.ts".to_string(), 8);
        let mut defs = BTreeSet::new();
        defs.insert("crates/apps/amigo-editor/src/app/store/main.ts".to_string());
        let changed = BTreeSet::new();
        let mut changed_status = BTreeMap::new();
        changed_status.insert(
            "crates/apps/amigo-editor/src/app/store/main.ts".to_string(),
            "M".to_string(),
        );

        let ranked = rank_open_set_items(
            &defs,
            &changed,
            &changed_status,
            &refs,
            &BTreeMap::new(),
            &BTreeMap::new(),
            true,
            "read",
        );
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

    #[test]
    fn deprioritizes_fixtures() {
        let mut refs = BTreeMap::new();
        refs.insert(
            "crates/tools/amigo-codemap/tests/fixtures/move_plan/editor_store.tsx".to_string(),
            10,
        );
        refs.insert(
            "crates/apps/amigo-editor/src/app/selectionSelectors.ts".to_string(),
            2,
        );
        let ranked = rank_open_set_items(
            &BTreeSet::new(),
            &BTreeSet::new(),
            &BTreeMap::new(),
            &refs,
            &BTreeMap::new(),
            &BTreeMap::new(),
            true,
            "read",
        );
        assert!(
            !ranked.contains_key(
                "crates/tools/amigo-codemap/tests/fixtures/move_plan/editor_store.tsx"
            )
        );
        assert!(ranked.contains_key("crates/apps/amigo-editor/src/app/selectionSelectors.ts"));
    }

    #[test]
    fn snapshot_open_set() {
        let root = temp_root("open-set");
        std::fs::create_dir_all(root.join("crates/apps/amigo-editor/src/app/store"))
            .expect("create store dir");
        std::fs::create_dir_all(root.join("crates/apps/amigo-editor/src/app"))
            .expect("create app dir");
        std::fs::write(
            root.join("crates/apps/amigo-editor/src/app/selectionTypes.ts"),
            "export type EditorSelectionRef = string;\n",
        )
        .expect("write definition");
        std::fs::write(
            root.join("crates/apps/amigo-editor/src/app/store/editorState.ts"),
            "const value: EditorSelectionRef = 'x';\n",
        )
        .expect("write ref");
        std::fs::write(root.join("README.md"), "EditorSelectionRef docs\n").expect("write docs");

        let map = CodeMap {
            root_name: "repo".to_string(),
            stats: BTreeMap::new(),
            files: vec![
                FileEntry {
                    id: "f1".to_string(),
                    path: PathBuf::from("crates/apps/amigo-editor/src/app/selectionTypes.ts"),
                    language: "ts".to_string(),
                    lines: 1,
                    hash: String::new(),
                    size: 0,
                    ..Default::default()
                },
                FileEntry {
                    id: "f2".to_string(),
                    path: PathBuf::from("crates/apps/amigo-editor/src/app/store/editorState.ts"),
                    language: "ts".to_string(),
                    lines: 1,
                    hash: String::new(),
                    size: 0,
                    ..Default::default()
                },
                FileEntry {
                    id: "f3".to_string(),
                    path: PathBuf::from("README.md"),
                    language: "md".to_string(),
                    lines: 1,
                    hash: String::new(),
                    size: 0,
                    ..Default::default()
                },
            ],
            packages: Vec::new(),
            symbols: vec![SymbolEntry {
                name: "EditorSelectionRef".to_string(),
                kind: "type".to_string(),
                file_id: "f1".to_string(),
                line: 1,
                visibility: "export".to_string(),
                ..Default::default()
            }],
            dependencies: Vec::new(),
            areas: Vec::new(),
            git: GitInfo {
                branch: "main".to_string(),
                rev: "abc".to_string(),
                dirty: true,
                changed: Vec::new(),
            },
            ..Default::default()
        };

        let report = build_open_set_report(
            root.as_path(),
            &map,
            "EditorSelectionRef",
            Some("migrate"),
            5,
            true,
        )
        .expect("open-set should build");
        assert_eq!(
            crate::report::file_ops::model::render_report(&report).trim(),
            include_str!("../../../tests/snapshots/open_set.snap").trim()
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
