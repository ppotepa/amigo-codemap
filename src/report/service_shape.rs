use std::collections::BTreeMap;
use std::fs;

use anyhow::{Result, bail};
use regex::Regex;

use crate::model::CodeMap;

use super::common::{feature_group, files_by_id, print_next, symbols_matching, text_refs};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldUsageClass {
    Unused,
    CandidateFeatureService,
    Infrastructure,
    CandidateBridgeService,
    CrossFeature,
}

pub fn extract_ts_fields(text: &str, type_name: &str) -> Vec<String> {
    let Some(start) = text
        .find(&format!("interface {type_name}"))
        .or_else(|| text.find(&format!("type {type_name}")))
    else {
        return Vec::new();
    };
    let rest = &text[start..];
    let Some(open) = rest.find('{') else {
        return Vec::new();
    };
    let mut depth = 0i32;
    let mut body = String::new();
    for ch in rest[open..].chars() {
        if ch == '{' {
            depth += 1;
            if depth == 1 {
                continue;
            }
        }
        if ch == '}' {
            depth -= 1;
            if depth == 0 {
                break;
            }
        }
        body.push(ch);
    }
    let field_re = Regex::new(r"^\s*([A-Za-z0-9_]+)\??\s*:").unwrap();
    body.lines()
        .filter_map(|line| field_re.captures(line).map(|caps| caps[1].to_string()))
        .collect()
}

pub fn print_service_shape(
    root: &std::path::Path,
    map: &CodeMap,
    query: &str,
    limit: usize,
) -> Result<()> {
    if query.is_empty() {
        bail!("service-shape requires a type/interface name");
    }
    let defs = symbols_matching(map, query);
    let files = files_by_id(map);
    let mut fields = Vec::new();
    if let Some(def) = defs.first() {
        if let Some(file) = files.get(def.file_id.as_str()) {
            let text = fs::read_to_string(root.join(&file.path)).unwrap_or_default();
            fields.extend(extract_ts_fields(&text, query));
        }
    }
    if fields.is_empty() {
        for file in &map.files {
            if !(file
                .path
                .extension()
                .is_some_and(|ext| ext == "ts" || ext == "tsx"))
            {
                continue;
            }
            let text = fs::read_to_string(root.join(&file.path)).unwrap_or_default();
            fields.extend(extract_ts_fields(&text, query));
        }
    }
    fields.sort();
    fields.dedup();
    println!("task: service-shape {query}");
    if let Some(def) = defs.first() {
        if let Some(file) = files.get(def.file_id.as_str()) {
            println!("definition: {}:{}", file.path.display(), def.line);
        }
    }
    println!("fields: {}", fields.len());
    println!("field usage:");
    let mut candidates = BTreeMap::<String, Vec<String>>::new();
    let definition_file_id = defs.first().map(|def| def.file_id.as_str());
    for field in fields.iter().take(limit) {
        let refs = text_refs(root, map, field, limit)?
            .into_iter()
            .filter(|item| {
                item.path.starts_with("crates/apps/amigo-editor/src/")
                    && (item.path.ends_with(".ts") || item.path.ends_with(".tsx"))
            })
            .filter(|item| {
                definition_file_id != Some(item.file_id.as_str())
                    && item
                        .lines
                        .iter()
                        .any(|(_, line)| is_likely_field_usage(line, field))
            })
            .collect::<Vec<_>>();
        let mut features = refs
            .iter()
            .map(|item| feature_group(&item.path))
            .collect::<Vec<_>>();
        features.sort();
        features.dedup();
        let class = classify_field_usage(&features);
        println!("  {field}:");
        println!("    files: {}", refs.len());
        println!(
            "    feature: {}",
            if features.is_empty() {
                "unused".to_string()
            } else {
                features.join(", ")
            }
        );
        println!("    class: {}", class_name(class));
        if features.len() == 1 {
            candidates
                .entry(features[0].clone())
                .or_default()
                .push(field.clone());
        }
    }
    println!("candidates:");
    for (feature, fields) in candidates {
        println!("  {feature}: {}", fields.join(", "));
    }
    println!("risk:");
    println!("  high: prop drilling through ComponentHost");
    println!("  medium: inspector/properties bridge");
    print_next(&[
        "remove unused fields",
        "split one-feature fields",
        "run npm run build",
    ]);
    Ok(())
}

pub fn is_likely_field_usage(line: &str, field: &str) -> bool {
    line.contains(&format!(".{field}"))
        || line.contains(&format!("{{ {field}"))
        || line.contains(&format!("{field} }}"))
        || line.contains(&format!("{field},"))
}

pub fn classify_field_usage(features: &[String]) -> FieldUsageClass {
    match features {
        [] => FieldUsageClass::Unused,
        [only] if only == "main-window" => FieldUsageClass::Infrastructure,
        [..] if features.len() == 1 => FieldUsageClass::CandidateFeatureService,
        [left, right] if left == "main-window" || right == "main-window" => {
            FieldUsageClass::CandidateBridgeService
        }
        _ => FieldUsageClass::CrossFeature,
    }
}

fn class_name(class: FieldUsageClass) -> &'static str {
    match class {
        FieldUsageClass::Unused => "unused",
        FieldUsageClass::CandidateFeatureService => "candidate feature service",
        FieldUsageClass::Infrastructure => "infrastructure",
        FieldUsageClass::CandidateBridgeService => "candidate bridge service",
        FieldUsageClass::CrossFeature => "cross-feature",
    }
}

#[cfg(test)]
mod tests {
    use super::{FieldUsageClass, classify_field_usage, extract_ts_fields, is_likely_field_usage};

    #[test]
    fn extracts_ts_interface_fields() {
        assert_eq!(
            extract_ts_fields("interface X {\n a: string\n b?: number\n}", "X"),
            vec!["a", "b"]
        );
    }

    #[test]
    fn classifies_main_window_only_as_infrastructure() {
        assert_eq!(
            classify_field_usage(&["main-window".to_string()]),
            FieldUsageClass::Infrastructure
        );
    }

    #[test]
    fn detects_likely_field_usage() {
        assert!(is_likely_field_usage("services.details", "details"));
        assert!(!is_likely_field_usage(
            "details: EditorModDetailsDto",
            "details"
        ));
    }
}
