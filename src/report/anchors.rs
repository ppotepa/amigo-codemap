use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use serde_json::{Value, json};

use crate::model::{
    AnchorDiagnostic, AnchorIndex, AnchorIndexCounts, AnchorIndexEntry, CodeMap, CodemapTagEntry,
};
use crate::taxonomy::CodemapTaxonomy;

pub fn print_anchors(
    root: &Path,
    map: &CodeMap,
    query: Option<&str>,
    write: bool,
    limit: usize,
) -> Result<()> {
    let taxonomy = CodemapTaxonomy::try_load(root);
    let index = build_anchor_index(map, taxonomy.as_ref());

    if write {
        write_anchor_index(root, &index)?;
        write_coverage_report(root, &index)?;
        println!("wrote .amigo/codemap.anchors.generated.json");
        println!("wrote .amigo/codemap.coverage.generated.md");
        return Ok(());
    }

    print_anchor_summary(&index, query, limit);
    Ok(())
}

pub fn build_anchor_index(map: &CodeMap, taxonomy: Option<&CodemapTaxonomy>) -> AnchorIndex {
    let files = map
        .files
        .iter()
        .map(|file| {
            (
                file.id.as_str(),
                file.path.to_string_lossy().replace('\\', "/"),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut diagnostics = Vec::new();
    let mut anchors = Vec::new();
    let mut by_anchor = BTreeMap::<String, Vec<(String, usize)>>::new();
    let mut manually_anchored_file_ids = std::collections::BTreeSet::<String>::new();

    for tag in &map.tags {
        manually_anchored_file_ids.insert(tag.file_id.clone());
        let domain = tag.domain.clone().unwrap_or_else(|| "unknown".to_string());
        let role = tag.role.clone().unwrap_or_else(|| "file".to_string());
        let priority = tag
            .priority
            .clone()
            .or_else(|| taxonomy.map(CodemapTaxonomy::default_priority))
            .unwrap_or_else(|| "P2".to_string());
        let status = tag
            .status
            .clone()
            .or_else(|| taxonomy.map(CodemapTaxonomy::default_status));
        let file = files.get(tag.file_id.as_str()).cloned().unwrap_or_default();
        let score = anchor_score(tag, &domain, &role, &priority, taxonomy);

        by_anchor
            .entry(tag.anchor.clone())
            .or_default()
            .push((file.clone(), tag.line));

        push_validation_diagnostics(
            &mut diagnostics,
            tag,
            &domain,
            &role,
            &priority,
            &file,
            taxonomy,
        );

        anchors.push(AnchorIndexEntry {
            anchor: tag.anchor.clone(),
            domain,
            role,
            priority,
            layer: tag.layer.clone(),
            status,
            risk: tag.risk.clone(),
            tags: tag.tags.clone(),
            file,
            line: tag.line,
            comment: tag.raw.clone(),
            generated: tag.generated,
            score,
        });
    }

    for file in &map.files {
        if manually_anchored_file_ids.contains(&file.id)
            || !is_commentable_anchor_file(&file.language)
        {
            continue;
        }

        let path = file.path.to_string_lossy().replace('\\', "/");
        let (domain, layer) = infer_domain_and_layer(&path);
        let role = infer_role(&path, &file.language);
        let priority = "P2".to_string();
        let anchor = format!("file-{}", normalize_anchor_path(&path));
        let score = taxonomy
            .map(|taxonomy| taxonomy.priority_score(&priority) + taxonomy.role_score(&role))
            .unwrap_or_else(|| anchor_priority_score(&priority))
            - 5;

        anchors.push(AnchorIndexEntry {
            anchor,
            domain,
            role,
            priority,
            layer: Some(layer),
            status: taxonomy.map(CodemapTaxonomy::default_status),
            risk: None,
            tags: vec!["generated".to_string(), "file-level".to_string()],
            file: path,
            line: 1,
            comment: String::new(),
            generated: true,
            score,
        });
    }

    for (anchor, locations) in by_anchor {
        if locations.len() > 1 {
            diagnostics.push(AnchorDiagnostic {
                severity: "error".to_string(),
                kind: "duplicate-anchor".to_string(),
                message: format!(
                    "duplicate anchor `{anchor}` appears {} times",
                    locations.len()
                ),
                file: locations.first().map(|(file, _)| file.clone()),
                line: locations.first().map(|(_, line)| *line),
                anchor: Some(anchor),
            });
        }
    }

    anchors.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.domain.cmp(&right.domain))
            .then_with(|| left.anchor.cmp(&right.anchor))
    });

    let counts = anchor_counts(&anchors, &diagnostics);
    AnchorIndex {
        version: 1,
        repo: map.root_name.clone(),
        generated_at: unix_timestamp().to_string(),
        taxonomy_path: ".amigo/codemap.taxonomy.yml".to_string(),
        counts,
        anchors,
        diagnostics,
    }
}

fn is_commentable_anchor_file(language: &str) -> bool {
    matches!(
        language,
        "rs" | "ts"
            | "tsx"
            | "js"
            | "jsx"
            | "css"
            | "scss"
            | "html"
            | "md"
            | "yaml"
            | "toml"
            | "json"
            | "rhai"
            | "ps1"
    )
}

fn infer_domain_and_layer(path: &str) -> (String, String) {
    let pairs = [
        ("crates/tools/amigo-codemap/", ("codemap", "tool")),
        (
            "crates/apps/amigo-editor/src/editors/ui-document/",
            ("ui-document", "app"),
        ),
        (
            "crates/apps/amigo-editor/src/features/scenes/editor/",
            ("scene-editor", "app"),
        ),
        (
            "crates/apps/amigo-editor/src-tauri/src/editor_mode/",
            ("editor-mode", "app-backend"),
        ),
        (
            "crates/apps/amigo-editor/src-tauri/",
            ("editor-backend", "app-backend"),
        ),
        ("crates/apps/amigo-editor/src/api/", ("editor-api", "app")),
        (
            "crates/apps/amigo-editor/src/main-window/",
            ("workspace", "app"),
        ),
        ("crates/apps/amigo-editor/src/dock/", ("workspace", "app")),
        (
            "crates/apps/amigo-editor/src/properties/",
            ("properties", "app"),
        ),
        (
            "crates/apps/amigo-editor/src/features/inspector/",
            ("properties", "app"),
        ),
        (
            "crates/apps/amigo-editor/src/features/project/",
            ("project", "app"),
        ),
        (
            "crates/apps/amigo-editor/src/features/assets/",
            ("assets", "app"),
        ),
        ("crates/apps/amigo-editor/src/assets/", ("assets", "app")),
        (
            "crates/apps/amigo-editor/src/ui/context-dock/",
            ("context-dock", "app"),
        ),
        ("crates/apps/app/", ("runtime-app", "runtime-app")),
        ("crates/engine/scene/", ("engine-scene", "engine")),
        ("crates/ui/core/", ("ui-core", "engine")),
        ("crates/scripting/", ("scripting", "scripting")),
        ("mods/they-are-rotten/", ("they-are-rotten", "mod")),
        ("mods/playground-", ("playground", "mod")),
        ("docs/", ("docs", "docs")),
    ];

    for (prefix, (domain, layer)) in pairs {
        if path.starts_with(prefix) {
            return (domain.to_string(), layer.to_string());
        }
    }

    if path.ends_with(".md") {
        return ("docs".to_string(), "docs".to_string());
    }

    ("codemap".to_string(), "tool".to_string())
}

fn infer_role(path: &str, language: &str) -> String {
    if path.ends_with(".css") || path.ends_with(".scss") {
        return "style".to_string();
    }
    if path.ends_with("scene.yml") || path.ends_with("scene.yaml") {
        return "scene-yaml".to_string();
    }
    if path.ends_with(".rhai") {
        return "scene-script".to_string();
    }
    if path.ends_with("mod.toml") || path.ends_with("manifest.toml") {
        return "mod-manifest".to_string();
    }
    if path.ends_with(".md") {
        return "docs".to_string();
    }
    if matches!(language, "ts" | "tsx" | "rs") {
        return "file".to_string();
    }
    "file".to_string()
}

fn normalize_anchor_path(path: &str) -> String {
    let mut normalized = String::new();
    for ch in path.chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch.to_ascii_lowercase());
        } else if !normalized.ends_with('-') {
            normalized.push('-');
        }
    }
    normalized.trim_matches('-').to_string()
}

pub fn anchor_priority_score(priority: &str) -> i32 {
    match priority {
        "P0" => 100,
        "P1" => 60,
        "P2" => 25,
        "P3" => 10,
        "PX" => 5,
        _ => 0,
    }
}

pub fn tag_matches_query(tag: &CodemapTagEntry, query: &str) -> bool {
    let query = query.to_ascii_lowercase();
    if let Some((key, value)) = query.split_once(':') {
        return match key {
            "anchor" => tag.anchor.eq_ignore_ascii_case(value),
            "domain" => tag
                .domain
                .as_deref()
                .is_some_and(|domain| domain.eq_ignore_ascii_case(value)),
            "role" => tag
                .role
                .as_deref()
                .is_some_and(|role| role.eq_ignore_ascii_case(value)),
            "priority" => tag
                .priority
                .as_deref()
                .is_some_and(|priority| priority.eq_ignore_ascii_case(value)),
            "layer" => tag
                .layer
                .as_deref()
                .is_some_and(|layer| layer.eq_ignore_ascii_case(value)),
            "tag" | "tags" => tag.tags.iter().any(|tag| tag.eq_ignore_ascii_case(value)),
            _ => false,
        };
    }

    anchor_haystack(tag).contains(&query)
}

pub fn anchor_haystack(tag: &CodemapTagEntry) -> String {
    format!(
        "{} {} {} {} {} {}",
        tag.anchor,
        tag.domain.as_deref().unwrap_or_default(),
        tag.role.as_deref().unwrap_or_default(),
        tag.priority.as_deref().unwrap_or_default(),
        tag.layer.as_deref().unwrap_or_default(),
        tag.tags.join(",")
    )
    .to_ascii_lowercase()
}

fn push_validation_diagnostics(
    diagnostics: &mut Vec<AnchorDiagnostic>,
    tag: &CodemapTagEntry,
    domain: &str,
    role: &str,
    priority: &str,
    file: &str,
    taxonomy: Option<&CodemapTaxonomy>,
) {
    if tag.domain.is_none() {
        diagnostics.push(warning("missing-domain", "anchor has no domain", file, tag));
    }
    if tag.role.is_none() {
        diagnostics.push(warning("missing-role", "anchor has no role", file, tag));
    }
    if tag.priority.is_none() {
        diagnostics.push(warning(
            "missing-priority",
            "anchor has no priority; using default",
            file,
            tag,
        ));
    }

    let Some(taxonomy) = taxonomy else {
        return;
    };

    if !taxonomy.known_domain(domain) {
        diagnostics.push(error(
            "unknown-domain",
            &format!("unknown anchor domain `{domain}`"),
            file,
            tag,
        ));
    }
    if !taxonomy.known_role(role) {
        diagnostics.push(warning(
            "unknown-role",
            &format!("unknown anchor role `{role}`"),
            file,
            tag,
        ));
    }
    if !taxonomy.known_priority(priority) {
        diagnostics.push(warning(
            "unknown-priority",
            &format!("unknown anchor priority `{priority}`"),
            file,
            tag,
        ));
    }
}

fn anchor_score(
    tag: &CodemapTagEntry,
    domain: &str,
    role: &str,
    priority: &str,
    taxonomy: Option<&CodemapTaxonomy>,
) -> i32 {
    let mut score = taxonomy
        .map(|taxonomy| taxonomy.priority_score(priority) + taxonomy.role_score(role))
        .unwrap_or_else(|| anchor_priority_score(priority));

    if domain != "unknown" {
        score += 10;
    }
    if tag.generated {
        score -= 5;
    }
    score
}

fn anchor_counts(
    anchors: &[AnchorIndexEntry],
    diagnostics: &[AnchorDiagnostic],
) -> AnchorIndexCounts {
    let mut counts = AnchorIndexCounts {
        anchors: anchors.len(),
        ..Default::default()
    };

    for anchor in anchors {
        if anchor.generated {
            counts.generated += 1;
        } else {
            counts.manual += 1;
        }
        match anchor.priority.as_str() {
            "P0" => counts.p0 += 1,
            "P1" => counts.p1 += 1,
            "P2" => counts.p2 += 1,
            "P3" => counts.p3 += 1,
            "PX" => counts.px += 1,
            _ => {}
        }
    }

    counts.errors = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == "error")
        .count();
    counts.warnings = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == "warning")
        .count();
    counts
}

fn write_anchor_index(root: &Path, index: &AnchorIndex) -> Result<()> {
    let path = root.join(".amigo").join("codemap.anchors.generated.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        path,
        serde_json::to_string_pretty(&anchor_index_json(index))?,
    )?;
    Ok(())
}

fn write_coverage_report(root: &Path, index: &AnchorIndex) -> Result<()> {
    let path = root.join(".amigo").join("codemap.coverage.generated.md");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, render_coverage(index))?;
    Ok(())
}

fn print_anchor_summary(index: &AnchorIndex, query: Option<&str>, limit: usize) {
    println!("anchors: {}", index.counts.anchors);
    println!(
        "manual: {} generated: {} errors: {} warnings: {}",
        index.counts.manual, index.counts.generated, index.counts.errors, index.counts.warnings
    );

    let anchors = index
        .anchors
        .iter()
        .filter(|anchor| query.is_none_or(|query| anchor_entry_matches(anchor, query)))
        .take(limit)
        .collect::<Vec<_>>();

    println!("matches:");
    if anchors.is_empty() {
        println!("  none");
    } else {
        for anchor in anchors {
            println!(
                "  {} {} domain={} role={} file={}:{} score={}",
                anchor.priority,
                anchor.anchor,
                anchor.domain,
                anchor.role,
                anchor.file,
                anchor.line,
                anchor.score
            );
        }
    }
}

pub fn anchor_entry_matches(anchor: &AnchorIndexEntry, query: &str) -> bool {
    let query = query.to_ascii_lowercase();
    if let Some((key, value)) = query.split_once(':') {
        return match key {
            "anchor" => anchor.anchor.eq_ignore_ascii_case(value),
            "domain" => anchor.domain.eq_ignore_ascii_case(value),
            "role" => anchor.role.eq_ignore_ascii_case(value),
            "priority" => anchor.priority.eq_ignore_ascii_case(value),
            "layer" => anchor
                .layer
                .as_deref()
                .is_some_and(|layer| layer.eq_ignore_ascii_case(value)),
            "tag" | "tags" => anchor
                .tags
                .iter()
                .any(|tag| tag.eq_ignore_ascii_case(value)),
            _ => false,
        };
    }
    format!(
        "{} {} {} {} {} {}",
        anchor.anchor,
        anchor.domain,
        anchor.role,
        anchor.priority,
        anchor.layer.as_deref().unwrap_or_default(),
        anchor.tags.join(",")
    )
    .to_ascii_lowercase()
    .contains(&query)
}

fn render_coverage(index: &AnchorIndex) -> String {
    let mut domains = BTreeMap::<String, usize>::new();
    let mut roles = BTreeMap::<String, usize>::new();
    for anchor in &index.anchors {
        *domains.entry(anchor.domain.clone()).or_default() += 1;
        *roles.entry(anchor.role.clone()).or_default() += 1;
    }

    let mut out = String::new();
    out.push_str("# Codemap Anchor Coverage\n\n");
    out.push_str("Generated by `amigo-codemap anchors --write`.\n\n");
    out.push_str("## Summary\n\n");
    out.push_str("| Metric | Value |\n|---|---:|\n");
    out.push_str(&format!("| Anchors | {} |\n", index.counts.anchors));
    out.push_str(&format!("| Manual anchors | {} |\n", index.counts.manual));
    out.push_str(&format!(
        "| Generated anchors | {} |\n",
        index.counts.generated
    ));
    out.push_str(&format!("| Errors | {} |\n", index.counts.errors));
    out.push_str(&format!("| Warnings | {} |\n", index.counts.warnings));

    out.push_str("\n## Priority distribution\n\n| Priority | Count |\n|---|---:|\n");
    out.push_str(&format!("| P0 | {} |\n", index.counts.p0));
    out.push_str(&format!("| P1 | {} |\n", index.counts.p1));
    out.push_str(&format!("| P2 | {} |\n", index.counts.p2));
    out.push_str(&format!("| P3 | {} |\n", index.counts.p3));
    out.push_str(&format!("| PX | {} |\n", index.counts.px));

    out.push_str("\n## Domain distribution\n\n| Domain | Count |\n|---|---:|\n");
    for (domain, count) in domains {
        out.push_str(&format!("| {domain} | {count} |\n"));
    }

    out.push_str("\n## Diagnostics\n\n");
    if index.diagnostics.is_empty() {
        out.push_str("None.\n");
    } else {
        out.push_str("| Severity | Kind | File | Line | Message |\n|---|---|---|---:|---|\n");
        for diagnostic in &index.diagnostics {
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                diagnostic.severity,
                diagnostic.kind,
                diagnostic.file.as_deref().unwrap_or("-"),
                diagnostic
                    .line
                    .map(|line| line.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                diagnostic.message.replace('|', "\\|")
            ));
        }
    }

    out.push_str("\n## P0/P1 anchors\n\n| Priority | Anchor | Domain | Role | File |\n|---|---|---|---|---|\n");
    for anchor in index
        .anchors
        .iter()
        .filter(|anchor| matches!(anchor.priority.as_str(), "P0" | "P1"))
    {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {}:{} |\n",
            anchor.priority, anchor.anchor, anchor.domain, anchor.role, anchor.file, anchor.line
        ));
    }

    out
}

fn anchor_index_json(index: &AnchorIndex) -> Value {
    json!({
        "version": index.version,
        "repo": index.repo,
        "generated_at": index.generated_at,
        "taxonomy": index.taxonomy_path,
        "counts": {
            "anchors": index.counts.anchors,
            "manual": index.counts.manual,
            "generated": index.counts.generated,
            "p0": index.counts.p0,
            "p1": index.counts.p1,
            "p2": index.counts.p2,
            "p3": index.counts.p3,
            "px": index.counts.px,
            "errors": index.counts.errors,
            "warnings": index.counts.warnings,
        },
        "anchors": index.anchors.iter().map(|anchor| json!({
            "anchor": anchor.anchor,
            "domain": anchor.domain,
            "role": anchor.role,
            "priority": anchor.priority,
            "layer": anchor.layer,
            "status": anchor.status,
            "risk": anchor.risk,
            "tags": anchor.tags,
            "file": anchor.file,
            "line": anchor.line,
            "comment": anchor.comment,
            "generated": anchor.generated,
            "score": anchor.score,
        })).collect::<Vec<_>>(),
        "diagnostics": index.diagnostics.iter().map(|diagnostic| json!({
            "severity": diagnostic.severity,
            "kind": diagnostic.kind,
            "message": diagnostic.message,
            "file": diagnostic.file,
            "line": diagnostic.line,
            "anchor": diagnostic.anchor,
        })).collect::<Vec<_>>(),
    })
}

fn warning(kind: &str, message: &str, file: &str, tag: &CodemapTagEntry) -> AnchorDiagnostic {
    diagnostic("warning", kind, message, file, tag)
}

fn error(kind: &str, message: &str, file: &str, tag: &CodemapTagEntry) -> AnchorDiagnostic {
    diagnostic("error", kind, message, file, tag)
}

fn diagnostic(
    severity: &str,
    kind: &str,
    message: &str,
    file: &str,
    tag: &CodemapTagEntry,
) -> AnchorDiagnostic {
    AnchorDiagnostic {
        severity: severity.to_string(),
        kind: kind.to_string(),
        message: message.to_string(),
        file: Some(file.to_string()),
        line: Some(tag.line),
        anchor: Some(tag.anchor.clone()),
    }
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}
