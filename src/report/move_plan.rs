use std::collections::BTreeMap;
use std::fs;

use anyhow::{Result, bail};
use regex::Regex;

use super::common::{print_next, slash_path};
use super::tauri::extract_tauri_commands;

pub fn command_target(name: &str) -> &'static str {
    super::tauri::command_target(name)
}

pub fn print_move_plan(
    root: &std::path::Path,
    query: &str,
    by: Option<&str>,
    limit: usize,
) -> Result<()> {
    if query.is_empty() {
        bail!("move-plan requires a file or symbol");
    }
    let path = root.join(query);
    println!("task: move-plan {query}");
    println!("source:");
    if path.exists() {
        let text = fs::read_to_string(&path)?;
        println!("  lines: {}", text.lines().count());
        println!("  changed: unknown");
        if by == Some("tauri-command") || query.ends_with(".rs") {
            let mut groups = BTreeMap::<&str, Vec<String>>::new();
            for command in extract_tauri_commands(&slash_path(std::path::Path::new(query)), &text) {
                groups
                    .entry(command_target(&command.name))
                    .or_default()
                    .push(command.name);
            }
            println!("groups:");
            for (target, symbols) in groups.into_iter().take(limit) {
                println!(
                    "  {}:",
                    target
                        .trim_start_matches("commands/")
                        .trim_end_matches(".rs")
                );
                println!("    symbols: {}", symbols.join(", "));
                println!("    target: {target}");
            }
        } else {
            let re =
                Regex::new(r"\bexport\s+(?:function|const|type|interface|class)\s+([A-Za-z0-9_]+)")
                    .unwrap();
            let mut groups = BTreeMap::<&str, Vec<String>>::new();
            for caps in re.captures_iter(&text) {
                let name = caps[1].to_string();
                let lower = name.to_ascii_lowercase();
                let target = if lower.contains("selection") {
                    "selection"
                } else if lower.contains("workspace") {
                    "workspace"
                } else if lower.contains("asset") {
                    "assets"
                } else if lower.contains("project") {
                    "project"
                } else {
                    "shared"
                };
                groups.entry(target).or_default().push(name);
            }
            println!("groups:");
            for (target, symbols) in groups.into_iter().take(limit) {
                println!("  {target}:");
                println!("    symbols: {}", symbols.join(", "));
                println!("    target: src/{target}/");
            }
        }
    } else {
        println!("  file: not found, treating query as symbol");
    }
    println!("risk:");
    println!("  high: registration/import fallout");
    println!("  medium: helper visibility");
    print_next(&[
        "move shared helpers first",
        "move grouped symbols",
        "run verify-plan",
    ]);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::command_target;
    use crate::report::tauri::extract_tauri_commands;

    #[test]
    fn move_plan_suggests_target_modules() {
        assert_eq!(
            command_target("get_project_tree"),
            "commands/project_tree.rs"
        );
    }

    #[test]
    fn fixture_groups_tauri_commands_to_targets() {
        let text = include_str!("../../tests/fixtures/move_plan/tauri_commands_mod.rs");
        let commands = extract_tauri_commands("commands/mod.rs", text);
        let targets = commands
            .iter()
            .map(|command| (command.name.as_str(), command_target(&command.name)))
            .collect::<Vec<_>>();

        assert!(targets.contains(&("reveal_scene_document", "commands/project_tree.rs")));
        assert!(targets.contains(&("read_project_file", "commands/project_files.rs")));
        assert!(targets.contains(&("open_mod", "commands/session.rs")));
        assert!(targets.contains(&("open_settings_window", "commands/windows.rs")));
    }

    #[test]
    fn maps_reveal_scene_document_to_project_tree() {
        assert_eq!(
            command_target("reveal_scene_document"),
            "commands/project_tree.rs"
        );
    }

    #[test]
    fn maps_open_settings_window_to_windows() {
        assert_eq!(
            command_target("open_settings_window"),
            "commands/windows.rs"
        );
    }

    #[test]
    fn maps_cache_commands_to_cache() {
        assert_eq!(command_target("get_cache_info"), "commands/cache.rs");
    }

    #[test]
    fn maps_sheet_commands_to_sheets() {
        assert_eq!(command_target("load_sheet_resource"), "commands/sheets.rs");
    }
}
