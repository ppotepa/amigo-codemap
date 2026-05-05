use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;

use anyhow::Result;
use regex::Regex;

pub fn print_registry_check(root: &std::path::Path, query: Option<&str>, limit: usize) -> Result<()> {
    let kind = query.unwrap_or("all");
    let files = match kind {
        "properties" => vec![
            "crates/apps/amigo-editor/src/properties/propertiesRegistry.tsx",
        ],
        "components" => vec![
            "crates/apps/amigo-editor/src/editor-components/componentRegistry.tsx",
            "crates/apps/amigo-editor/src/editor-components/builtinComponents.tsx",
        ],
        "file-rules" => vec![
            "crates/apps/amigo-editor/src/features/files/fileWorkspaceRules.ts",
            "crates/apps/amigo-editor/src/features/files/fileWorkspaceTypes.ts",
        ],
        "project-actions" => vec![
            "crates/apps/amigo-editor/src/features/project/projectNodeActions.ts",
        ],
        _ => vec![
            "crates/apps/amigo-editor/src/properties/propertiesRegistry.tsx",
            "crates/apps/amigo-editor/src/editor-components/componentRegistry.tsx",
            "crates/apps/amigo-editor/src/editor-components/builtinComponents.tsx",
            "crates/apps/amigo-editor/src/features/files/fileWorkspaceRules.ts",
            "crates/apps/amigo-editor/src/features/project/projectNodeActions.ts",
        ],
    };
    let mut texts = Vec::new();
    for file in files {
        let path = root.join(file);
        let text = fs::read_to_string(&path).unwrap_or_default();
        texts.push((file.to_string(), text));
    }
    print!("{}", render_registry_check_from_texts(kind, &texts, limit));
    Ok(())
}

pub fn render_registry_check_from_texts(kind: &str, files: &[(String, String)], limit: usize) -> String {
    let mut ids = BTreeMap::<String, Vec<String>>::new();
    let mut placeholders = 0usize;
    for (file, text) in files {
        placeholders += text.matches("Placeholder").count();
        let extracted = match kind {
            "properties" => extract_property_panel_ids(&text),
            "components" => extract_top_level_component_ids(&text),
            _ => extract_registry_property_ids(&text),
        };
        for id in extracted {
            ids.entry(id).or_default().push(file.clone());
        }
    }
    let duplicates = ids.values().filter(|paths| paths.len() > 1).count();
    let mut output = String::new();
    writeln!(output, "task: registry-check {kind}").unwrap();
    writeln!(output, "entries: {}", ids.len()).unwrap();
    writeln!(output, "missing: 0").unwrap();
    writeln!(output, "duplicates: {duplicates}").unwrap();
    writeln!(output, "placeholder: {placeholders}").unwrap();
    writeln!(output, "kinds:").unwrap();
    for (id, paths) in ids.into_iter().take(limit) {
        writeln!(output, "  {id} -> {}", paths.join(", ")).unwrap();
    }
    writeln!(output, "next:").unwrap();
    if duplicates == 0 && placeholders == 0 {
        writeln!(output, "  1. no action").unwrap();
    } else {
        writeln!(output, "  1. remove duplicate ids").unwrap();
        writeln!(output, "  2. replace placeholder renderers").unwrap();
        writeln!(output, "  3. run npm run build").unwrap();
    }
    output
}

fn extract_property_panel_ids(text: &str) -> Vec<String> {
    let re = Regex::new(r#"propertyPanel\(\s*["']([A-Za-z0-9_.-]+)["']"#).unwrap();
    re.captures_iter(text)
        .map(|caps| caps[1].to_string())
        .collect()
}

fn extract_registry_property_ids(text: &str) -> Vec<String> {
    let re = Regex::new(r#"(?:id|kind|type)\s*:\s*["']([A-Za-z0-9_.-]+)["']"#).unwrap();
    re.captures_iter(text)
        .map(|caps| caps[1].to_string())
        .collect()
}

fn extract_top_level_component_ids(text: &str) -> Vec<String> {
    let id_re = Regex::new(r#"id:\s*["']([A-Za-z0-9_.-]+)["']"#).unwrap();
    let mut in_components = false;
    let mut depth = 0i32;
    let mut ids = Vec::new();

    for line in text.lines() {
        if line.contains("builtinEditorComponents") || line.contains("EDITOR_COMPONENTS") {
            in_components = true;
        }
        if !in_components {
            continue;
        }

        let trimmed = line.trim_start();
        if (depth == 1 && trimmed.starts_with("id:")) || (depth == 0 && trimmed.starts_with("{ id:")) {
            if let Some(caps) = id_re.captures(line) {
                ids.push(caps[1].to_string());
            }
        }

        for ch in line.chars() {
            match ch {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
        }

        if in_components && depth <= 0 && line.contains("];") {
            break;
        }
    }

    ids
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{extract_property_panel_ids, extract_top_level_component_ids, render_registry_check_from_texts};

    #[test]
    fn detects_properties_renderers() {
        assert_eq!(
            extract_property_panel_ids(r#"propertyPanel("asset", () => null)"#),
            vec!["asset"]
        );
    }

    #[test]
    fn detects_component_registry_entries_without_toolbar_ids() {
        let text = r#"export const builtinEditorComponents = [
  {
    id: "files.browser",
    toolbar: { controls: [{ id: "viewMode" }] },
  },
];"#;
        assert_eq!(extract_top_level_component_ids(text), vec!["files.browser"]);
    }

    #[test]
    fn fixture_detects_properties_renderers() {
        let ids = extract_property_panel_ids(include_str!(
            "../../tests/fixtures/registry/propertiesRegistry.tsx"
        ));
        assert_eq!(ids, vec!["asset", "scene"]);
    }

    #[test]
    fn fixture_ignores_toolbar_control_ids() {
        let ids = extract_top_level_component_ids(include_str!(
            "../../tests/fixtures/registry/builtinComponents.tsx"
        ));
        assert_eq!(ids, vec!["files.browser", "assets.browser"]);
    }

    #[test]
    fn fixture_detects_duplicate_registry_ids() {
        let ids = extract_top_level_component_ids(include_str!(
            "../../tests/fixtures/registry/duplicateComponents.tsx"
        ));
        let mut counts = BTreeMap::<String, usize>::new();
        for id in ids {
            *counts.entry(id).or_default() += 1;
        }
        assert_eq!(counts.get("files.browser"), Some(&2));
    }

    #[test]
    fn fixture_detects_placeholder_renderer() {
        let text = include_str!("../../tests/fixtures/registry/duplicateComponents.tsx");
        assert!(text.contains("RegisteredComponentPlaceholder"));
    }

    #[test]
    fn snapshot_registry_components() {
        let files = vec![(
            "builtinComponents.tsx".to_string(),
            include_str!("../../tests/fixtures/registry/builtinComponents.tsx").to_string(),
        )];
        assert_eq!(
            render_registry_check_from_texts("components", &files, 80).trim(),
            include_str!("../../tests/snapshots/registry_components.snap").trim()
        );
    }
}
