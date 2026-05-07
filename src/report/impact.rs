use std::collections::BTreeMap;

use anyhow::{Result, bail};

use crate::model::CodeMap;

use super::common::{feature_group, print_next, sorted_counts, symbols_matching, text_refs};
use super::verify_plan::plan_for_map;

pub fn risk_for(path: &str, line: &str, kind: Option<&str>) -> Option<&'static str> {
    if matches!(kind, Some("type" | "interface"))
        && (path.contains("Reducer") || path.contains("Actions") || path.contains("/store/"))
    {
        Some("high: reducer/action compatibility")
    } else if path.contains("src-tauri") || line.contains("generate_handler") {
        Some("high: command registration/API boundary")
    } else if path.contains("/properties/") || path.contains("/inspector/") {
        Some("medium: resolved selection bridge")
    } else if path.contains("workspaceRuntimeServices") || path.contains("componentTypes") {
        Some("medium: service bag compatibility")
    } else if path.ends_with("lib.rs") {
        Some("high: public API boundary")
    } else {
        None
    }
}

pub fn print_impact(
    root: &std::path::Path,
    map: &CodeMap,
    query: &str,
    group: Option<&str>,
    lines: bool,
    limit: usize,
) -> Result<()> {
    if query.is_empty() {
        bail!("impact requires a symbol or text query");
    }
    let files = files_by_id(map);
    let defs = symbols_matching(map, query);
    let refs = text_refs(root, map, query, limit)?;
    let kind = defs.first().map(|symbol| symbol.kind.as_str());
    println!("task: impact {query}");
    println!("defs:");
    if defs.is_empty() {
        println!("  none");
    }
    for symbol in defs.iter().take(limit) {
        println!(
            "  {} {}:{} {}",
            symbol.kind, symbol.file_id, symbol.line, symbol.visibility
        );
    }
    println!("refs:");
    println!("  files: {}", refs.len());
    println!(
        "  changed: {}",
        refs.iter().filter(|item| item.changed).count()
    );
    println!(
        "  unchanged: {}",
        refs.iter().filter(|item| !item.changed).count()
    );

    let mut counts = BTreeMap::<String, usize>::new();
    for item in &refs {
        let key = match group {
            Some("feature") => feature_group(&item.path),
            _ => super::common::group_path(std::path::Path::new(&item.path), group),
        };
        *counts.entry(key).or_default() += item.lines.len();
    }
    println!("groups:");
    for (name, count) in sorted_counts(counts).into_iter().take(limit) {
        println!("  {name}: {count} refs");
        for item in refs
            .iter()
            .filter(|item| {
                let key = if group == Some("feature") {
                    feature_group(&item.path)
                } else {
                    super::common::group_path(std::path::Path::new(&item.path), group)
                };
                key == name
            })
            .take(8)
        {
            let suffix = if item.changed { " changed" } else { "" };
            println!("    {}{}", item.path, suffix);
            if lines {
                for (line, text) in item.lines.iter().take(2) {
                    println!("      {line}: {text}");
                }
            }
        }
    }
    let mut risks = refs
        .iter()
        .flat_map(|item| {
            item.lines
                .iter()
                .filter_map(|(_, line)| risk_for(&item.path, line, kind))
        })
        .collect::<Vec<_>>();
    risks.sort_unstable();
    risks.dedup();
    println!("risk:");
    if risks.is_empty() {
        println!("  none");
    }
    for risk in risks {
        println!("  {risk}");
    }
    println!("direct impact:");
    for symbol in &map.symbols {
        if (symbol.name == query || symbol.signature.contains(query))
            && let Some(path) = files.get(symbol.file_id.as_str())
        {
            println!(
                "  {}:{}-{} {} {}",
                path, symbol.line, symbol.line_end, symbol.kind, symbol.name
            );
        }
    }
    println!("text/config impact:");
    let query_lower = query.to_ascii_lowercase();
    for occurrence in &map.text_occurrences {
        if occurrence.normalized_value.contains(&query_lower)
            && let Some(path) = files.get(occurrence.file_id.as_str())
        {
            println!(
                "  {}:{} {} {}",
                path, occurrence.line, occurrence.kind, occurrence.context
            );
        }
    }
    println!("likely affected:");
    for relation in &map.relations {
        if relation.from.contains(query) || relation.to.contains(query) {
            println!(
                "  {} -> {} kind={} confidence={}",
                relation.from, relation.to, relation.kind, relation.confidence
            );
        }
    }
    let plan = plan_for_map(map, true);
    println!("tests:");
    for file in &map.files {
        let path = file.path.to_string_lossy().replace('\\', "/");
        if file.tags.iter().any(|tag| tag == "kind:test")
            && path.to_ascii_lowercase().contains(&query_lower)
        {
            println!("  {path}");
        }
    }
    for cmd in &plan.required {
        println!("  {cmd}");
    }
    println!("verify:");
    println!("  cargo build -p amigo-codemap");
    println!("  cargo test -p amigo-codemap");
    print_next(&[
        "read definitions",
        "migrate highest-risk groups",
        "run verify-plan",
    ]);
    Ok(())
}

fn files_by_id(map: &CodeMap) -> std::collections::BTreeMap<&str, String> {
    map.files
        .iter()
        .map(|file| {
            (
                file.id.as_str(),
                file.path.to_string_lossy().replace('\\', "/"),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::risk_for;

    #[test]
    fn impact_detects_reducer_risk() {
        assert_eq!(
            risk_for("src/app/store/editorReducer.ts", "x", Some("type")),
            Some("high: reducer/action compatibility")
        );
    }
}
