use std::path::Path;

use crate::model::FileEntry;

use super::model::SmellMetrics;

pub(super) fn is_rankable_file(path: &str) -> bool {
    !path.ends_with("Cargo.lock")
        && !path.ends_with("package-lock.json")
        && !path.ends_with(".png")
        && !path.ends_with(".jpg")
        && !path.ends_with(".webp")
}

pub(super) fn is_generated_file(file: &FileEntry, path: &str) -> bool {
    file.tags.iter().any(|tag| tag == "kind:generated")
        || path.contains("/gen/")
        || path.contains(".generated.")
        || path.ends_with("-schema.json")
}

pub(super) fn is_test_file(path: &str) -> bool {
    path.contains("/tests/")
        || path.ends_with("_test.rs")
        || path.ends_with(".test.ts")
        || path.ends_with(".test.tsx")
        || path.ends_with(".spec.ts")
        || path.ends_with(".spec.tsx")
}

pub(super) fn is_catalog_file(path: &str) -> bool {
    path.contains("descriptor")
        || path.contains("registry")
        || path.ends_with("dto.rs")
        || path.ends_with("dto.ts")
        || path.ends_with("-schema.json")
}

pub(super) fn is_stylesheet_file(path: &str) -> bool {
    path.ends_with(".css")
        || path.ends_with(".scss")
        || path.ends_with(".sass")
        || path.ends_with(".less")
}

pub(super) fn is_data_table_file(path: &str) -> bool {
    path.contains("/scenes/")
        && (path.ends_with(".yml") || path.ends_with(".yaml") || path.ends_with(".json"))
}

pub(super) fn file_role(path: &str) -> &'static str {
    if is_stylesheet_file(path) {
        "stylesheet"
    } else if path.contains("/src-tauri/src/commands/") || path.ends_with("_command.rs") {
        "command-handler"
    } else if path.contains("/src-tauri/src/commands.rs")
        || path.ends_with("/commands.rs")
        || path.contains("/app-tauri/src/commands/")
    {
        "command-handler"
    } else if path.contains("/features/") && path.contains("target") {
        "resolver"
    } else if path.contains("editor-targets") || path.contains("editor_target") {
        "editor-target"
    } else if path.contains("/main-window/") || path.contains("MainEditorWindow") {
        "editor-shell"
    } else if path.contains("/components/") || path.contains("editor-targets") {
        "ui-component"
    } else if is_catalog_file(path) {
        "catalog"
    } else if path.ends_with(".rs") {
        "service"
    } else if path.ends_with(".tsx") || path.ends_with(".jsx") {
        "component"
    } else if path.ends_with(".ts") || path.ends_with(".tsx") || path.ends_with(".js") {
        "frontend"
    } else {
        "misc"
    }
}

pub(super) fn role_confidence(_path: &str, role: &str) -> &'static str {
    match role {
        "component" | "editor-shell" => "high",
        "catalog" | "stylesheet" => "medium",
        "command-handler" | "service" => "high",
        _ => "low",
    }
}

pub(super) fn component_or_function_smell(path: &str, level: &str) -> &'static str {
    if path.ends_with(".tsx") && level == "critical" {
        "component.god_component"
    } else if path.ends_with(".tsx") {
        "component.too_large"
    } else if path.ends_with(".rs") {
        "method.too_long"
    } else {
        "function.too_long"
    }
}

pub(super) fn next_actions(path: &str, metrics: &SmellMetrics) -> Vec<String> {
    let mut next = vec![format!(
        "run: amigo-codemap open-set \"{path}\" --why --limit 10"
    )];
    if let Some(symbol) = &metrics.largest_block {
        next.push(format!("run: amigo-codemap slice {path} --symbol {symbol}"));
    }
    if path.contains("/src-tauri/src/commands/") || path.ends_with("_command.rs") {
        next.push(
            "dispatch commands by op kind and move each class/handler into its own module"
                .to_string(),
        );
    } else if path.contains("editor-targets") || path.contains("editor_target") {
        next.push("split resolver modules into one file per target kind".to_string());
    } else if path.ends_with(".tsx") {
        next.push("split shell/layout/actions/hooks/services".to_string());
    } else if path.ends_with(".rs") {
        next.push("split by responsibility: validation/apply/model/helpers".to_string());
    } else if is_stylesheet_file(path) {
        next.push("extract critical CSS blocks and shared tokens into theme files".to_string());
    } else if is_catalog_file(path) {
        next.push(
            "split descriptor catalog loaders from runtime logic into distinct modules".to_string(),
        );
    } else {
        next.push("split catalog/data and behavior into separate modules".to_string());
    }
    if metrics.symbols > 120 {
        next.push(format!(
            "extract {} symbols into focused submodules or interfaces",
            metrics.symbols
        ));
    }
    if metrics.largest_block_lines > 180 {
        next.push(format!(
            "consider splitting dominant symbol '{}' ({} lines)",
            metrics
                .largest_block
                .clone()
                .unwrap_or_else(|| "main-symbol".to_string()),
            metrics.largest_block_lines
        ));
    }
    next
}

pub(super) fn push(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|item| item == value) {
        values.push(value.to_string());
    }
}

pub(super) fn domain_for_path(path: &str) -> &str {
    if path.contains("amigo-codemap") {
        "codemap"
    } else if path.contains("amigo-editor") {
        "editor"
    } else if path.contains("/engine/") {
        "engine"
    } else if path.contains("/apps/app/") {
        "runtime-app"
    } else {
        "unknown"
    }
}

pub(super) fn package_for_path(path: &str) -> &str {
    path.split("/src/").next().unwrap_or(path)
}

pub(super) fn extension(path: &str) -> &str {
    path.rsplit('.').next().unwrap_or("unknown")
}

pub(super) fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
