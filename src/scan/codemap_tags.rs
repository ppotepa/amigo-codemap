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
        for (line_index, line) in text.lines().enumerate() {
            let Some(caps) = marker_re.captures(line) else {
                continue;
            };
            let body = caps.name("body").map(|m| m.as_str()).unwrap_or_default();
            let values = parse_values(body);
            let name = values
                .get("anchor")
                .or_else(|| values.get("name"))
                .or_else(|| values.get("domain"))
                .cloned()
                .unwrap_or_else(|| "codemap".to_string());
            let target = values
                .get("target")
                .cloned()
                .unwrap_or_else(|| "line".to_string());

            tags.push(CodemapTagEntry {
                name,
                file_id: file.id.clone(),
                line: line_index + 1,
                target,
                values,
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
        let values = parse_values("anchor:workspace-dock domain:workspace role:registry");
        assert_eq!(
            values.get("anchor").map(String::as_str),
            Some("workspace-dock")
        );
        assert_eq!(values.get("domain").map(String::as_str), Some("workspace"));
        assert_eq!(values.get("role").map(String::as_str), Some("registry"));
    }
}
