use std::fs;
use std::path::Path;

use anyhow::Result;
use regex::Regex;

use crate::model::{FileEntry, TextOccurrenceEntry};

pub fn scan_text_occurrences(root: &Path, files: &[FileEntry]) -> Result<Vec<TextOccurrenceEntry>> {
    let string_re = Regex::new(
        r#""(?P<double>[^"\\]*(?:\\.[^"\\]*)*)"|'(?P<single>[^'\\]*(?:\\.[^'\\]*)*)'|`(?P<tick>[^`]*)`"#,
    )?;
    let yaml_re = Regex::new(r#"^\s*(?P<key>[A-Za-z0-9_.:-]+)\s*:\s*(?P<value>.+)?$"#)?;
    let toml_re = Regex::new(r#"^\s*(?P<key>[A-Za-z0-9_.:-]+)\s*=\s*(?P<value>.+)?$"#)?;
    let css_re = Regex::new(r#"(?P<class>\.[A-Za-z_-][A-Za-z0-9_-]*)"#)?;

    let mut entries = Vec::new();
    let mut next_id = 1usize;

    for file in files {
        if !matches!(
            file.language.as_str(),
            "rs" | "ts" | "tsx" | "js" | "jsx" | "yaml" | "yml" | "toml" | "cargo" | "css"
        ) {
            continue;
        }

        let text = fs::read_to_string(root.join(&file.path))?;
        for (line_index, line) in text.lines().enumerate() {
            let line_number = line_index + 1;

            for caps in string_re.captures_iter(line) {
                let value = caps
                    .name("double")
                    .or_else(|| caps.name("single"))
                    .or_else(|| caps.name("tick"))
                    .map(|m| m.as_str())
                    .unwrap_or_default();
                if should_skip_value(value) {
                    continue;
                }
                let column = caps.get(0).map(|m| m.start() + 1).unwrap_or(1);
                push_occurrence(
                    &mut entries,
                    &mut next_id,
                    file,
                    value,
                    classify_value_kind(file, value),
                    line_number,
                    column,
                    line,
                    60,
                );
            }

            if matches!(file.language.as_str(), "yaml" | "yml")
                && let Some(caps) = yaml_re.captures(line)
            {
                let key = caps.name("key").map(|m| m.as_str()).unwrap_or_default();
                push_occurrence(
                    &mut entries,
                    &mut next_id,
                    file,
                    key,
                    "yaml-key",
                    line_number,
                    1,
                    line,
                    70,
                );

                if let Some(value) = caps.name("value").map(|m| clean_scalar(m.as_str()))
                    && !should_skip_value(&value)
                {
                    push_occurrence(
                        &mut entries,
                        &mut next_id,
                        file,
                        &value,
                        "yaml-value",
                        line_number,
                        line.find(&value).unwrap_or_default() + 1,
                        line,
                        65,
                    );
                }
            }

            if matches!(file.language.as_str(), "toml" | "cargo")
                && let Some(caps) = toml_re.captures(line)
            {
                let key = caps.name("key").map(|m| m.as_str()).unwrap_or_default();
                push_occurrence(
                    &mut entries,
                    &mut next_id,
                    file,
                    key,
                    "toml-key",
                    line_number,
                    1,
                    line,
                    70,
                );
            }

            if file.language == "css" {
                for caps in css_re.captures_iter(line) {
                    let value = caps.name("class").map(|m| m.as_str()).unwrap_or_default();
                    push_occurrence(
                        &mut entries,
                        &mut next_id,
                        file,
                        value,
                        "css-class",
                        line_number,
                        caps.get(0).map(|m| m.start() + 1).unwrap_or(1),
                        line,
                        70,
                    );
                }
            }
        }
    }

    Ok(entries)
}

fn push_occurrence(
    entries: &mut Vec<TextOccurrenceEntry>,
    next_id: &mut usize,
    file: &FileEntry,
    value: &str,
    kind: &str,
    line: usize,
    column: usize,
    context: &str,
    confidence: u8,
) {
    let value = value.trim();
    if value.is_empty() {
        return;
    }
    entries.push(TextOccurrenceEntry {
        id: format!("tx{}", *next_id),
        value: value.to_string(),
        normalized_value: normalize_value(value),
        kind: kind.to_string(),
        file_id: file.id.clone(),
        line,
        column,
        owner: None,
        context: trim_context(context),
        tags: vec![format!("kind:{kind}"), format!("lang:{}", file.language)],
        confidence,
    });
    *next_id += 1;
}

fn classify_value_kind(file: &FileEntry, value: &str) -> &'static str {
    let path = file.path.to_string_lossy().replace('\\', "/");
    if value.contains('.') && value.contains('-') {
        "id-like-string"
    } else if path.contains("dock") || value.ends_with(".panel") || value.ends_with(".inspector") {
        "dock-id"
    } else if path.contains("commands") {
        "command-name"
    } else if path.contains("scenes/") {
        "scene-id"
    } else if path.contains("assets") {
        "asset-id"
    } else {
        "string-literal"
    }
}

fn clean_scalar(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim_end_matches(',')
        .to_string()
}

fn normalize_value(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_ascii_lowercase()
}

fn trim_context(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.len() > 160 {
        format!("{}...", &trimmed[..160])
    } else {
        trimmed.to_string()
    }
}

fn should_skip_value(value: &str) -> bool {
    let value = value.trim();
    value.len() < 3 || value.len() > 180
}

#[cfg(test)]
mod tests {
    use super::normalize_value;

    #[test]
    fn normalizes_string_values() {
        assert_eq!(normalize_value("\"Entity.Inspector\""), "entity.inspector");
    }
}
