use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::Result;

use super::classify::{domain_for_path, extension, package_for_path};
use super::model::{CodeSmellFinding, SmellOptions};

pub(super) fn print_code_smells_output(
    root: &Path,
    findings: &[CodeSmellFinding],
    options: &SmellOptions,
) -> Result<()> {
    if options.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "code_smells": findings,
            }))?
        );
        if options.report {
            let output_path = options
                .report_file
                .clone()
                .unwrap_or_else(|| root.join(".amigo").join("code-smells-report.json"));
            fs::create_dir_all(output_path.parent().unwrap_or(Path::new(".")))?;
            fs::write(
                &output_path,
                serde_json::to_string_pretty(&serde_json::json!({
                    "code_smells": findings,
                    "code_smells_report": true,
                }))?,
            )?;
            println!("report saved: {}", output_path.display());
        }
        return Ok(());
    }

    if let Some(group) = options.group.as_deref() {
        print_grouped_smells(findings, group);
        if options.report {
            let output_path = options
                .report_file
                .clone()
                .unwrap_or_else(|| root.join(".amigo").join("code-smells-report.json"));
            fs::create_dir_all(output_path.parent().unwrap_or(Path::new(".")))?;
            let serialized = serde_json::to_string_pretty(&serde_json::json!({
                "grouped_by": group,
                "groups": find_smell_groups(findings, group),
                "findings": findings,
            }))?;
            fs::write(&output_path, serialized)?;
            println!("report saved: {}", output_path.display());
        }
        return Ok(());
    }

    if options.report {
        print_report_summary(findings);
        let output_path = options
            .report_file
            .clone()
            .unwrap_or_else(|| root.join(".amigo").join("code-smells-report.json"));
        fs::create_dir_all(output_path.parent().unwrap_or(Path::new(".")))?;
        let serialized = serde_json::to_string_pretty(&serde_json::json!({
            "summary": {
                "total": findings.len(),
            },
            "findings": findings,
        }))?;
        fs::write(&output_path, serialized)?;
        println!("report saved: {}", output_path.display());
    }

    println!("code-smells:");
    let limit = if options.report {
        findings.len()
    } else {
        options.top
    };
    for (index, finding) in findings.iter().take(limit).enumerate() {
        println!("  {}. {}", index + 1, finding.path);
        if finding.line > 0 {
            println!("     line: {}", finding.line);
        }
        println!("     score: {}", finding.score);
        println!("     severity: {}", finding.severity);
        println!("     confidence: {}", finding.confidence);
        if let Some(symbol) = &finding.symbol {
            println!("     symbol: {symbol}");
        }
        println!("     smells:");
        for smell in &finding.smells {
            println!("       - {smell}");
        }
        println!("     metrics:");
        println!("       lines: {}", finding.metrics.lines);
        println!("       symbols: {}", finding.metrics.symbols);
        println!("       branch_tokens: {}", finding.metrics.branch_tokens);
        println!(
            "       branch_density: {:.1} per 100 LOC",
            finding.metrics.branch_density
        );
        if let Some(block) = &finding.metrics.largest_block {
            println!(
                "       largest_block: {}, {} lines",
                block, finding.metrics.largest_block_lines
            );
        }
        println!("       todo_markers: {}", finding.metrics.todo_markers);
        println!("       fixme_markers: {}", finding.metrics.fixme_markers);
        println!("       hack_markers: {}", finding.metrics.hack_markers);
        println!("       unwrap_markers: {}", finding.metrics.unwrap_markers);
        println!("       expect_markers: {}", finding.metrics.expect_markers);
        println!("       outgoing_deps: {}", finding.metrics.outgoing_deps);
        println!("       incoming_deps: {}", finding.metrics.incoming_deps);
        if options.why {
            println!("     reasons:");
            for reason in &finding.reasons {
                println!("       - {reason}");
            }
            println!("     next:");
            for next in &finding.next_actions {
                println!("       - {next}");
            }
        }
    }
    Ok(())
}

fn print_grouped_smells(findings: &[CodeSmellFinding], group: &str) {
    let mut groups = BTreeMap::<String, Vec<&CodeSmellFinding>>::new();
    for finding in findings {
        for key in group_keys(finding, group) {
            groups.entry(key).or_default().push(finding);
        }
    }
    println!("code-smells:{group}");
    for (name, mut items) in groups {
        items.sort_by(|left, right| right.score.cmp(&left.score));
        let max_score = items
            .iter()
            .map(|item| item.score)
            .max()
            .unwrap_or_default();
        println!("  {name}: count={} max_score={}", items.len(), max_score);
        for item in items.into_iter().take(3) {
            println!("    - {} score={}", item.path, item.score);
        }
    }
}

fn print_report_summary(findings: &[CodeSmellFinding]) {
    println!("code-smells-report:");
    println!("  total: {}", findings.len());
    for group in ["severity", "domain", "smell"] {
        let counts = find_smell_groups(findings, group);
        println!("  by_{group}:");
        for (key, count) in counts {
            println!("    {key}: {count}");
        }
    }
}

fn group_keys(finding: &CodeSmellFinding, group: &str) -> Vec<String> {
    match group {
        "severity" => vec![finding.severity.to_string()],
        "language" => vec![extension(&finding.path).to_string()],
        "smell" => finding.smells.clone(),
        "path" => vec![
            finding
                .path
                .split('/')
                .take(3)
                .collect::<Vec<_>>()
                .join("/"),
        ],
        "domain" => vec![domain_for_path(&finding.path).to_string()],
        "package" => vec![package_for_path(&finding.path).to_string()],
        _ => vec!["all".to_string()],
    }
}

fn find_smell_groups(
    findings: &[CodeSmellFinding],
    group: &str,
) -> std::collections::BTreeMap<String, usize> {
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for finding in findings {
        for key in group_keys(finding, group) {
            *counts.entry(key).or_default() += 1;
        }
    }
    counts
}
