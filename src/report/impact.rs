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
    let plan = plan_for_map(map, true);
    println!("tests:");
    for cmd in &plan.required {
        println!("  {cmd}");
    }
    print_next(&[
        "read definitions",
        "migrate highest-risk groups",
        "run verify-plan",
    ]);
    Ok(())
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
