use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use crate::model::{CodeMap, FileEntry};
use crate::report::common::{
    area_for_file, feature_group, is_docs, is_test_file, package_for_file, package_for_path,
    slash_path,
};

use super::common::{find_file_by_path, import_block, read_text_at_root, symbols_in_file};
use super::exports::parse_ts_exports;
use super::imports::parse_imports;
use super::model::{FileOpReport, NextAction, Risk, RiskLevel, print_report};

#[derive(Debug, Clone)]
struct RankedDonor<'a> {
    file: &'a FileEntry,
    score: i32,
    reasons: Vec<String>,
}

#[derive(Debug, Clone)]
struct TargetAnchor {
    line: usize,
    label: String,
}

pub fn print_copy_plan(
    root: &Path,
    map: &CodeMap,
    query: &str,
    from: Option<&Path>,
    task: Option<&str>,
    limit: usize,
) -> Result<()> {
    if query.trim().is_empty() {
        bail!("copy-plan requires a target file path");
    }

    let task = task.unwrap_or("generic");
    let target_path = normalized_query_path(query);
    let target_file = find_file_by_path(map, query);
    let target_text = target_file
        .map(|file| read_text_at_root(root, &file.path))
        .transpose()?;
    let target_display = target_file
        .map(|file| slash_path(&file.path))
        .unwrap_or_else(|| slash_path(&target_path));

    let ranked = rank_donors(map, &target_path, task, limit);
    let donor = if let Some(source) = from {
        let source_query = slash_path(source);
        let file = find_file_by_path(map, &source_query).ok_or_else(|| {
            anyhow::anyhow!("copy-plan could not find donor file `{}`", source.display())
        })?;
        RankedDonor {
            file,
            score: 999,
            reasons: vec!["manual donor".to_string()],
        }
    } else {
        ranked
            .first()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("copy-plan could not find a donor candidate"))?
    };

    let donor_text = read_text_at_root(root, &donor.file.path)?;
    let scope = build_scope(
        map,
        &target_path,
        target_file,
        &target_display,
        &donor,
        task,
    );
    let findings = build_findings(
        map,
        &target_path,
        target_file,
        target_text.as_deref(),
        &donor,
        &ranked,
        &donor_text,
        limit,
    );
    let risks = build_risks(&target_path, target_file, &donor, &donor_text);
    let verify = verify_steps(target_file.unwrap_or(donor.file));
    let next = build_next(target_file.is_some());

    print_report(&FileOpReport {
        task: format!("copy-plan {target_display}"),
        scope,
        findings,
        risks,
        verify,
        next,
    });

    Ok(())
}

fn build_scope(
    map: &CodeMap,
    target_path: &Path,
    target_file: Option<&FileEntry>,
    target_display: &str,
    donor: &RankedDonor<'_>,
    task: &str,
) -> Vec<String> {
    let mut scope = vec![
        format!("target: {target_display}"),
        format!("task: {task}"),
        format!(
            "target exists: {}",
            if target_file.is_some() { "yes" } else { "no" }
        ),
        format!("donor: {}", slash_path(&donor.file.path)),
    ];

    let target_extension = target_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default();
    if !target_extension.is_empty() {
        scope.push(format!("target language: {target_extension}"));
    }

    if let Some(file) = target_file {
        scope.push(format!("target lines: {}", file.lines));
        if let Some(package) = package_for_file(map, file) {
            scope.push(format!("package: {package}"));
        }
        if let Some(area) = area_for_file(map, file) {
            scope.push(format!("area: {area}"));
        }
    } else if let Some(package) = package_for_path(map, target_path) {
        scope.push(format!("package: {package}"));
        scope.push(format!(
            "feature guess: {}",
            feature_group(&slash_path(target_path))
        ));
    }

    scope
}

fn build_findings(
    map: &CodeMap,
    target_path: &Path,
    target_file: Option<&FileEntry>,
    target_text: Option<&str>,
    donor: &RankedDonor<'_>,
    ranked: &[RankedDonor<'_>],
    donor_text: &str,
    limit: usize,
) -> Vec<String> {
    let mut findings = Vec::new();
    findings.push("selected donor:".to_string());
    findings.push(format!(
        "  {} [score {}, {}]",
        slash_path(&donor.file.path),
        donor.score,
        donor.reasons.join(", ")
    ));

    findings.push("alternate donors:".to_string());
    let mut alternates = 0usize;
    for candidate in ranked
        .iter()
        .filter(|candidate| candidate.file.id != donor.file.id)
    {
        findings.push(format!(
            "  {} [score {}, {}]",
            slash_path(&candidate.file.path),
            candidate.score,
            candidate.reasons.join(", ")
        ));
        alternates += 1;
        if alternates >= limit.min(4) {
            break;
        }
    }
    if alternates == 0 {
        findings.push("  none".to_string());
    }

    findings.push("copy surface:".to_string());
    let donor_imports = import_block(donor_text);
    if donor_imports.is_empty() {
        findings.push("  import block: none".to_string());
    } else {
        let first = donor_imports.first().map(|item| item.0).unwrap_or(1);
        let last = donor_imports.last().map(|item| item.0).unwrap_or(first);
        findings.push(format!("  import block: lines {first}-{last}"));
    }
    let donor_symbols = symbol_names(map, donor.file, donor_text);
    if donor_symbols.is_empty() {
        findings.push("  exported or indexed symbols: none".to_string());
    } else {
        findings.push(format!(
            "  exported or indexed symbols: {}",
            donor_symbols.join(", ")
        ));
    }

    findings.push("rename hotspots:".to_string());
    let hotspots = rename_hotspots(map, target_path, donor.file, donor_text);
    if hotspots.is_empty() {
        findings.push("  none".to_string());
    } else {
        for hotspot in hotspots.into_iter().take(8) {
            findings.push(format!("  {hotspot}"));
        }
    }

    findings.push("mirrored companion files:".to_string());
    let companions = mirrored_companions(map, target_path, donor.file);
    if companions.is_empty() {
        findings.push("  none".to_string());
    } else {
        for item in companions.into_iter().take(8) {
            findings.push(format!("  {item}"));
        }
    }

    findings.push("target anchors:".to_string());
    let anchors = target_anchors(target_file, target_text);
    if anchors.is_empty() {
        findings.push("  none".to_string());
    } else {
        for anchor in anchors {
            findings.push(format!("  line {}: {}", anchor.line, anchor.label));
        }
    }

    findings
}

fn build_risks(
    target_path: &Path,
    target_file: Option<&FileEntry>,
    donor: &RankedDonor<'_>,
    donor_text: &str,
) -> Vec<Risk> {
    let mut risks = vec![Risk {
        level: RiskLevel::High,
        message: "blind copy can preserve stale names, imports, and test labels".to_string(),
    }];

    if let Some(target_file) = target_file {
        if target_file.language != donor.file.language {
            risks.push(Risk {
                level: RiskLevel::High,
                message: "target and donor use different languages; copy only the smallest block"
                    .to_string(),
            });
        }
    } else if target_path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext != donor.file.language)
    {
        risks.push(Risk {
            level: RiskLevel::Medium,
            message: "target extension differs from donor language; review file template first"
                .to_string(),
        });
    }

    let relative_imports = parse_imports(&donor.file.path, donor_text)
        .into_iter()
        .filter(|entry| entry.specifier.starts_with('.'))
        .count();
    if relative_imports >= 3 {
        risks.push(Risk {
            level: RiskLevel::Medium,
            message: format!("donor has {relative_imports} relative imports to rewrite"),
        });
    }

    if slash_path(target_path) == slash_path(&donor.file.path) {
        risks.push(Risk {
            level: RiskLevel::High,
            message: "target matches donor path; use append-plan instead of copy-plan".to_string(),
        });
    }

    risks
}

fn verify_steps(file: &FileEntry) -> Vec<String> {
    match file.language.as_str() {
        "tsx" | "ts" | "css" => vec!["npm run build".to_string(), "npm test".to_string()],
        "rs" => vec!["cargo test -p amigo-codemap".to_string()],
        _ => vec!["run the smallest affected check".to_string()],
    }
}

fn build_next(target_exists: bool) -> Vec<NextAction> {
    let mut next = vec![
        NextAction {
            label: "copy the smallest matching surface from the selected donor".to_string(),
        },
        NextAction {
            label: "rename hotspots before fixing imports and props".to_string(),
        },
    ];
    if target_exists {
        next.push(NextAction {
            label: "run append-plan on the target file before inserting copied code".to_string(),
        });
    } else {
        next.push(NextAction {
            label: "create mirrored companion files only if the donor actually needs them"
                .to_string(),
        });
    }
    next
}

fn rank_donors<'a>(
    map: &'a CodeMap,
    target_path: &Path,
    task: &str,
    limit: usize,
) -> Vec<RankedDonor<'a>> {
    let target_dir = target_path.parent().map(PathBuf::from);
    let target_extension = target_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_string();
    let target_stem = target_path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_default();
    let target_tokens = name_tokens(&format!("{target_stem} {task}"));
    let target_package = package_for_path(map, target_path);
    let target_group = feature_group(&slash_path(target_path));
    let wants_tests = task.contains("test") || slash_path(target_path).contains(".test.");

    let mut ranked = map
        .files
        .iter()
        .filter(|file| !is_docs(&file.path))
        .filter(|file| wants_tests || !is_test_file(&file.path))
        .filter(|file| {
            file.path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or_default()
                == target_extension
        })
        .map(|file| {
            let path = slash_path(&file.path);
            let mut score = 0i32;
            let mut reasons = Vec::new();
            let file_stem = file
                .path
                .file_stem()
                .map(|stem| stem.to_string_lossy().to_string())
                .unwrap_or_default();

            if file.path.parent().map(|path| path.to_path_buf()) == target_dir {
                score += 120;
                reasons.push("same directory".to_string());
            }

            if package_for_file(map, file) == target_package {
                score += 80;
                reasons.push("same package".to_string());
            }

            if feature_group(&path) == target_group {
                score += 60;
                reasons.push("same feature group".to_string());
            }

            let shared_prefix = shared_prefix_len(&target_stem, &file_stem);
            if shared_prefix > 0 {
                score += (shared_prefix.min(12) as i32) * 4;
                reasons.push(format!("shared prefix {shared_prefix}"));
            }

            let file_tokens = name_tokens(&file_stem);
            let token_overlap = overlapping_tokens(&target_tokens, &file_tokens);
            if token_overlap > 0 {
                score += (token_overlap.min(4) as i32) * 20;
                reasons.push(format!("token overlap {token_overlap}"));
            }

            if name_suffix(&target_stem) == name_suffix(&file_stem)
                && !name_suffix(&file_stem).is_empty()
            {
                score += 40;
                reasons.push("same suffix".to_string());
            }

            if file.lines < 600 {
                score += 10;
                reasons.push("copyable size".to_string());
            }

            RankedDonor {
                file,
                score,
                reasons,
            }
        })
        .collect::<Vec<_>>();

    ranked.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| slash_path(&left.file.path).cmp(&slash_path(&right.file.path)))
    });
    ranked.truncate(limit.max(3));
    ranked
}

fn symbol_names(map: &CodeMap, donor: &FileEntry, donor_text: &str) -> Vec<String> {
    let mut names = BTreeSet::new();
    for symbol in symbols_in_file(map, &donor.id) {
        names.insert(symbol.name.clone());
        if names.len() >= 6 {
            break;
        }
    }
    if names.is_empty() {
        for export in parse_ts_exports(donor_text) {
            names.insert(export.name);
            if names.len() >= 6 {
                break;
            }
        }
    }
    names.into_iter().collect()
}

fn rename_hotspots(
    map: &CodeMap,
    target_path: &Path,
    donor: &FileEntry,
    donor_text: &str,
) -> Vec<String> {
    let donor_stem = donor
        .path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_default();
    let target_stem = target_path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut hotspots = Vec::new();

    if !donor_stem.is_empty() && !target_stem.is_empty() && donor_stem != target_stem {
        hotspots.push(format!("file stem `{donor_stem}` -> `{target_stem}`"));
    }

    for name in symbol_names(map, donor, donor_text) {
        if donor_stem.is_empty() || name.contains(&donor_stem) || starts_with_uppercase(&name) {
            hotspots.push(format!("review exported or indexed symbol `{name}`"));
        }
    }

    let stem_refs = donor_text.matches(&donor_stem).count();
    if stem_refs > 1 {
        hotspots.push(format!(
            "raw donor stem `{donor_stem}` appears {stem_refs} times in donor text"
        ));
    }

    for import in parse_imports(&donor.path, donor_text)
        .into_iter()
        .filter(|entry| entry.specifier.starts_with('.'))
        .take(4)
    {
        hotspots.push(format!(
            "rewrite relative import at line {}: {}",
            import.line, import.specifier
        ));
    }

    hotspots
}

fn mirrored_companions(map: &CodeMap, target_path: &Path, donor: &FileEntry) -> Vec<String> {
    let donor_dir = donor.path.parent().map(PathBuf::from);
    let donor_stem = donor
        .path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_default();
    let target_stem = target_path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_default();
    let target_dir = target_path.parent().unwrap_or_else(|| Path::new(""));

    let mut items = Vec::new();
    for candidate in &map.files {
        let candidate_name = candidate
            .path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();
        if candidate.id == donor.id
            || candidate.path.parent().map(|path| path.to_path_buf()) != donor_dir
            || (!candidate_name.starts_with(&format!("{donor_stem}."))
                && !candidate_name.starts_with(&format!("{donor_stem}_")))
        {
            continue;
        }

        let mirrored_name = if candidate_name.starts_with(&format!("{donor_stem}.")) {
            candidate_name.replacen(&format!("{donor_stem}."), &format!("{target_stem}."), 1)
        } else {
            candidate_name.replacen(&format!("{donor_stem}_"), &format!("{target_stem}_"), 1)
        };
        let mirrored = target_dir.join(mirrored_name);
        items.push(format!(
            "{} -> {}",
            slash_path(&candidate.path),
            slash_path(&mirrored)
        ));
    }
    items.sort();
    items
}

fn target_anchors(target_file: Option<&FileEntry>, target_text: Option<&str>) -> Vec<TargetAnchor> {
    let mut anchors = Vec::new();
    let Some(text) = target_text else {
        return anchors;
    };
    if let Some((line, _)) = import_block(text).last() {
        anchors.push(TargetAnchor {
            line: *line,
            label: "after import block".to_string(),
        });
    }
    if let Some(file) = target_file {
        anchors.push(TargetAnchor {
            line: file.lines.max(1),
            label: "before file end".to_string(),
        });
    }
    anchors
}

fn normalized_query_path(query: &str) -> PathBuf {
    PathBuf::from(query.replace('\\', "/"))
}

fn name_tokens(value: &str) -> Vec<String> {
    let chars = value.chars().collect::<Vec<_>>();
    let mut tokens = Vec::new();
    let mut current = String::new();

    for (index, ch) in chars.iter().copied().enumerate() {
        if !ch.is_ascii_alphanumeric() {
            if !current.is_empty() {
                tokens.push(current.to_ascii_lowercase());
                current.clear();
            }
            continue;
        }

        let next = chars.get(index + 1).copied();
        let boundary = current.chars().last().is_some_and(|prev| {
            (prev.is_ascii_lowercase() && ch.is_ascii_uppercase())
                || (prev.is_ascii_alphabetic() && ch.is_ascii_digit())
                || (prev.is_ascii_digit() && ch.is_ascii_alphabetic())
                || (prev.is_ascii_uppercase()
                    && ch.is_ascii_uppercase()
                    && next.is_some_and(|next| next.is_ascii_lowercase()))
        });

        if boundary && !current.is_empty() {
            tokens.push(current.to_ascii_lowercase());
            current.clear();
        }

        current.push(ch);
    }

    if !current.is_empty() {
        tokens.push(current.to_ascii_lowercase());
    }

    if tokens.is_empty() {
        tokens = value
            .split(|ch: char| !ch.is_ascii_alphanumeric())
            .filter(|token| !token.is_empty())
            .map(|token| token.to_ascii_lowercase())
            .collect();
    }
    tokens
}

fn overlapping_tokens(left: &[String], right: &[String]) -> usize {
    let right_set = right.iter().collect::<BTreeSet<_>>();
    left.iter()
        .filter(|token| right_set.contains(token))
        .count()
}

fn name_suffix(value: &str) -> String {
    name_tokens(value).last().cloned().unwrap_or_default()
}

fn shared_prefix_len(left: &str, right: &str) -> usize {
    left.chars()
        .zip(right.chars())
        .take_while(|(left, right)| left == right)
        .count()
}

fn starts_with_uppercase(value: &str) -> bool {
    value
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::model::{CodeMap, FileEntry, GitInfo, SymbolEntry};

    use super::{mirrored_companions, rank_donors, rename_hotspots};

    fn sample_map() -> CodeMap {
        CodeMap {
            root_name: "repo".to_string(),
            stats: Default::default(),
            files: vec![
                FileEntry {
                    id: "a".to_string(),
                    path: PathBuf::from("src/startup/StartupDialog.tsx"),
                    language: "tsx".to_string(),
                    lines: 120,
                    hash: "a".to_string(),
                    size: 120,
                    ..Default::default()
                },
                FileEntry {
                    id: "b".to_string(),
                    path: PathBuf::from("src/startup/ModsPanel.tsx"),
                    language: "tsx".to_string(),
                    lines: 80,
                    hash: "b".to_string(),
                    size: 80,
                    ..Default::default()
                },
                FileEntry {
                    id: "c".to_string(),
                    path: PathBuf::from("src/startup/ModsPanel.css"),
                    language: "css".to_string(),
                    lines: 20,
                    hash: "c".to_string(),
                    size: 20,
                    ..Default::default()
                },
                FileEntry {
                    id: "d".to_string(),
                    path: PathBuf::from("src/main-window/MainEditorWindow.tsx"),
                    language: "tsx".to_string(),
                    lines: 300,
                    hash: "d".to_string(),
                    size: 300,
                    ..Default::default()
                },
            ],
            packages: vec![],
            symbols: vec![SymbolEntry {
                name: "ModsPanel".to_string(),
                kind: "component".to_string(),
                file_id: "b".to_string(),
                line: 12,
                visibility: "export".to_string(),
                ..Default::default()
            }],
            dependencies: vec![],
            areas: vec![],
            git: GitInfo::default(),
            ..Default::default()
        }
    }

    #[test]
    fn ranks_same_directory_donor_first() {
        let map = sample_map();
        let ranked = rank_donors(&map, &PathBuf::from("src/startup/NewPanel.tsx"), "panel", 4);
        assert_eq!(ranked.first().map(|item| item.file.id.as_str()), Some("b"));
    }

    #[test]
    fn suggests_mirrored_companion_files() {
        let map = sample_map();
        let donor = map
            .files
            .iter()
            .find(|file| file.id == "b")
            .expect("donor file should exist");
        let companions =
            mirrored_companions(&map, &PathBuf::from("src/startup/NewPanel.tsx"), donor);
        assert_eq!(
            companions,
            vec!["src/startup/ModsPanel.css -> src/startup/NewPanel.css".to_string()]
        );
    }

    #[test]
    fn rename_hotspots_include_stem_and_relative_imports() {
        let map = sample_map();
        let donor = map
            .files
            .iter()
            .find(|file| file.id == "b")
            .expect("donor file should exist");
        let hotspots = rename_hotspots(
            &map,
            &PathBuf::from("src/startup/NewPanel.tsx"),
            donor,
            "import { loadMods } from \"./modsModel\";\nexport function ModsPanel() {}\nconst title = \"ModsPanel\";\n",
        );
        assert!(hotspots.iter().any(|item| item.contains("ModsPanel")));
        assert!(hotspots.iter().any(|item| item.contains("./modsModel")));
    }
}
