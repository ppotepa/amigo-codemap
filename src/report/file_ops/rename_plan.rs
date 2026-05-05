use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::model::CodeMap;

use super::common::{read_text_at_root, slash_path};
use super::model::{FileOpReport, NextAction, Risk, RiskLevel};
use crate::report::common::group_path;

pub fn print_rename_plan(
    root: &Path,
    map: &CodeMap,
    old: &str,
    new: Option<&str>,
    group: Option<&str>,
    limit: usize,
) -> Result<()> {
    let next_name = new.unwrap_or(old);
    let target = if next_name.is_empty() { old } else { old };
    let scope = vec![
        format!("from: {old}"),
        format!("to: {next_name}"),
        format!("group: {}", group.unwrap_or("file-path")),
    ];

    let mut exact_by_group: BTreeMap<String, usize> = BTreeMap::new();
    let mut partial_by_group: BTreeMap<String, usize> = BTreeMap::new();
    let mut partial_hits = BTreeSet::new();

    for file in &map.files {
        let Ok(text) = read_text_at_root(root, &file.path) else {
            continue;
        };
        let mut exact_hits = 0usize;
        let mut partial_hits_file = 0usize;
        for line in text.lines() {
            if is_word_match(line, target) {
                exact_hits += 1;
            } else if line.contains(target) {
                partial_hits_file += 1;
            }
        }

        if exact_hits > 0 || partial_hits_file > 0 {
            let key = group_path(&file.path, Some("feature"));
            if exact_hits > 0 {
                *exact_by_group.entry(key.clone()).or_default() += exact_hits;
            }
            if partial_hits_file > 0 {
                *partial_by_group.entry(key).or_default() += partial_hits_file;
            } else {
                continue;
            }
            if exact_hits == 0 && partial_hits_file > 0 {
                partial_hits.insert(slash_path(&file.path));
            }
        }
    }

    let mut findings = Vec::new();
    findings.push("exact refs:".to_string());
    if exact_by_group.is_empty() {
        findings.push("  none".to_string());
    } else {
        for (group_name, count) in exact_by_group {
            findings.push(format!("  {}: {}", group_name, count));
        }
    }

    findings.push("partial refs:".to_string());
    if partial_by_group.is_empty() {
        findings.push("  none".to_string());
    } else {
        for (group_name, count) in partial_by_group {
            findings.push(format!("  {}: {}", group_name, count));
        }
        for file in partial_hits.into_iter().take(limit) {
            findings.push(format!("  inspect: {file}"));
        }
    }

    let mut risks = vec![Risk {
        level: RiskLevel::High,
        message: if !target.is_empty() && target.starts_with('A') {
            "capitalized type-like names may collide with exports".to_string()
        } else {
            "string replace can overmatch partial names".to_string()
        },
    }];
    if !new.is_some() {
        risks.push(Risk {
            level: RiskLevel::Medium,
            message: "exact rename requested without --to".to_string(),
        });
    }

    super::model::print_report(&FileOpReport {
        task: format!("rename-plan {old}"),
        scope,
        findings,
        risks,
        verify: vec!["npm test".to_string(), "npm run build".to_string()],
        next: vec![
            NextAction {
                label: format!("replace exact refs with {next_name}"),
            },
            NextAction {
                label: "inspect partial refs manually".to_string(),
            },
            NextAction {
                label: "run fallout after build".to_string(),
            },
        ],
    });

    Ok(())
}

pub fn is_word_match(line: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    let bytes = line.as_bytes();
    let needle_bytes = needle.as_bytes();
    let mut index = 0usize;
    while index + needle_bytes.len() <= bytes.len() {
        if &bytes[index..index + needle_bytes.len()] == needle_bytes {
            let before = index
                .checked_sub(1)
                .and_then(|value| bytes.get(value))
                .copied();
            let after = bytes.get(index + needle_bytes.len()).copied();
            if !is_ident_char(before) && !is_ident_char(after) {
                return true;
            }
        }
        index += 1;
    }
    false
}

fn is_ident_char(value: Option<u8>) -> bool {
    matches!(value, Some(b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_'))
}

#[cfg(test)]
mod tests {
    use super::is_word_match;

    #[test]
    fn matches_whole_word_only() {
        assert!(is_word_match("selectedAsset + 1", "selectedAsset"));
        assert!(!is_word_match("selectedAssetKey", "selectedAsset"));
    }
}
