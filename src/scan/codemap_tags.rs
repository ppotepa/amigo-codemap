use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::Result;
use regex::Regex;

use crate::model::{CodemapTagEntry, FileEntry};

pub fn scan_codemap_tags(root: &Path, files: &[FileEntry]) -> Result<Vec<CodemapTagEntry>> {
    let marker_re = Regex::new(r#"@codemap\s+(?P<body>.+)$"#)?;
    let mut tags = Vec::new();

    for file in files {
        let text = fs::read_to_string(root.join(&file.path))?;
        let mut in_markdown_fence = false;
        let is_markdown = file
            .path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| matches!(extension, "md" | "mdx"));
        for (line_index, line) in text.lines().enumerate() {
            if is_markdown && line.trim_start().starts_with("```") {
                in_markdown_fence = !in_markdown_fence;
                continue;
            }

            if in_markdown_fence {
                continue;
            }

            let Some(caps) = marker_re.captures(line) else {
                continue;
            };
            let body = caps.name("body").map(|m| m.as_str()).unwrap_or_default();
            let values = parse_values(body);
            let Some(anchor) = values
                .get("anchor")
                .or_else(|| values.get("name"))
                .or_else(|| values.get("domain"))
                .cloned()
            else {
                continue;
            };
            let codemap_tags = values
                .get("tags")
                .map(|value| {
                    value
                        .split(',')
                        .map(str::trim)
                        .filter(|tag| !tag.is_empty())
                        .map(ToOwned::to_owned)
                        .collect()
                })
                .unwrap_or_default();
            let target = values
                .get("target")
                .cloned()
                .unwrap_or_else(|| "line".to_string());

            tags.push(CodemapTagEntry {
                name: anchor.clone(),
                anchor,
                file_id: file.id.clone(),
                line: line_index + 1,
                target,
                domain: values.get("domain").cloned(),
                role: values.get("role").cloned(),
                priority: values.get("priority").cloned(),
                layer: values.get("layer").cloned(),
                status: values.get("status").cloned(),
                risk: values.get("risk").cloned(),
                owner: values.get("owner").cloned(),
                tags: codemap_tags,
                values,
                raw: line.trim().to_string(),
                generated: false,
                confidence: 100,
            });
        }
    }

    Ok(tags)
}

fn parse_values(body: &str) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    for token in body.split_whitespace() {
        let Some((key, value)) = token.split_once(':') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim().trim_matches(',').to_string();
        if !key.is_empty() && !value.is_empty() {
            values.insert(key, value);
        }
    }
    values
}

#[cfg(test)]
mod tests {
    use super::parse_values;

    #[test]
    fn parses_codemap_values() {
        let values = parse_values(
            "anchor:workspace-dock domain:workspace role:registry priority:P0 tags:dock,registry",
        );
        assert_eq!(
            values.get("anchor").map(String::as_str),
            Some("workspace-dock")
        );
        assert_eq!(values.get("domain").map(String::as_str), Some("workspace"));
        assert_eq!(values.get("role").map(String::as_str), Some("registry"));
        assert_eq!(values.get("priority").map(String::as_str), Some("P0"));
        assert_eq!(
            values.get("tags").map(String::as_str),
            Some("dock,registry")
        );
    }
}
