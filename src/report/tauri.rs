use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use anyhow::Result;
use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TauriCommand {
    pub name: String,
    pub path: String,
    pub group: String,
}

pub fn extract_tauri_commands(path: &str, text: &str) -> Vec<TauriCommand> {
    let fn_re = Regex::new(r"\bfn\s+([A-Za-z0-9_]+)").unwrap();
    let mut commands = Vec::new();
    let mut after_attr = false;
    for line in text.lines() {
        if line.contains("#[tauri::command]") {
            after_attr = true;
            continue;
        }
        if after_attr {
            if let Some(caps) = fn_re.captures(line) {
                commands.push(TauriCommand {
                    name: caps[1].to_string(),
                    path: path.to_string(),
                    group: command_group(&caps[1]).to_string(),
                });
                after_attr = false;
            }
        }
    }
    commands
}

pub fn extract_generate_handler_entries(text: &str) -> Vec<String> {
    let Some(start) = text.find("generate_handler![") else {
        return Vec::new();
    };
    let rest = &text[start + "generate_handler![".len()..];
    let Some(end) = rest.find(']') else {
        return Vec::new();
    };
    rest[..end]
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| entry.rsplit("::").next().unwrap_or(entry).to_string())
        .collect()
}

pub fn print_tauri_commands(root: &Path, limit: usize) -> Result<()> {
    let base = root.join("crates/apps/amigo-editor/src-tauri/src");
    let mut commands = Vec::new();
    if base.exists() {
        for entry in walk_rs(&base)? {
            let rel = entry.strip_prefix(root).unwrap_or(&entry).to_string_lossy().replace('\\', "/");
            let text = fs::read_to_string(&entry).unwrap_or_default();
            commands.extend(extract_tauri_commands(&rel, &text));
        }
    }
    let lib = fs::read_to_string(base.join("lib.rs")).unwrap_or_default();
    print!("{}", render_tauri_commands(&commands, &lib, limit));
    Ok(())
}

pub fn render_tauri_commands(commands: &[TauriCommand], handler_text: &str, limit: usize) -> String {
    let registered = extract_generate_handler_entries(handler_text).into_iter().collect::<BTreeSet<_>>();
    let names = commands.iter().map(|cmd| cmd.name.clone()).collect::<Vec<_>>();
    let mut seen = BTreeSet::new();
    let duplicates = names
        .iter()
        .filter(|name| !seen.insert((*name).clone()))
        .cloned()
        .collect::<BTreeSet<_>>();
    let missing = names
        .iter()
        .filter(|name| !registered.contains(*name))
        .cloned()
        .collect::<BTreeSet<_>>();
    let stale_registered = registered
        .iter()
        .filter(|name| !names.contains(name))
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut output = String::new();
    writeln!(output, "task: tauri-commands").unwrap();
    writeln!(output, "commands: {}", names.len()).unwrap();
    writeln!(output, "registered: {}", registered.len()).unwrap();
    writeln!(output, "missing registration: {}", missing.len()).unwrap();
    writeln!(output, "registered without definition: {}", stale_registered.len()).unwrap();
    writeln!(output, "duplicates: {}", duplicates.len()).unwrap();
    writeln!(output, "groups:").unwrap();
    let mut groups = BTreeMap::<String, usize>::new();
    for command in commands {
        *groups.entry(command.group.clone()).or_default() += 1;
    }
    for (group, count) in groups.into_iter().take(limit) {
        writeln!(output, "  {group}: {count}").unwrap();
    }
    writeln!(output, "risk:").unwrap();
    if missing.is_empty() && duplicates.is_empty() && stale_registered.is_empty() {
        writeln!(output, "  none").unwrap();
    } else {
        for item in missing {
            writeln!(output, "  high: missing registration {item}").unwrap();
        }
        for item in stale_registered {
            writeln!(output, "  medium: registered without definition {item}").unwrap();
        }
        for item in duplicates {
            writeln!(output, "  high: duplicate command {item}").unwrap();
        }
    }
    writeln!(output, "next:").unwrap();
    writeln!(output, "  1. fix missing registrations").unwrap();
    writeln!(output, "  2. remove duplicate wrappers").unwrap();
    writeln!(output, "  3. run cargo test -p amigo-editor --lib").unwrap();
    output
}

pub fn command_group(name: &str) -> &'static str {
    if name.contains("window") {
        "windows"
    } else if name == "open_mod" || name == "open_mod_workspace" || name.contains("session") {
        "session"
    } else if name.contains("scene_document")
        || name.contains("tree")
        || name.contains("hierarchy")
        || name.contains("structure")
    {
        "project_tree"
    } else if name.contains("expected_project_folder")
        || name.contains("project_file")
        || name.contains("read_project")
        || name.contains("write_project")
    {
        "project_files"
    } else if name.contains("mod") || name.contains("validate") {
        "mods"
    } else if name.contains("asset") {
        "assets"
    } else if name.contains("sheet") || name.contains("tile") {
        "sheets"
    } else if name.contains("preview") {
        "preview"
    } else if name.contains("cache") {
        "cache"
    } else if name.contains("setting") || name.contains("theme") || name.contains("font") {
        "settings"
    } else {
        "shared"
    }
}

pub fn command_target_for_group(group: &str) -> &'static str {
    match group {
        "assets" => "commands/assets.rs",
        "cache" => "commands/cache.rs",
        "mods" => "commands/mods.rs",
        "preview" => "commands/preview.rs",
        "project_files" => "commands/project_files.rs",
        "project_tree" => "commands/project_tree.rs",
        "session" => "commands/session.rs",
        "settings" => "commands/settings.rs",
        "sheets" => "commands/sheets.rs",
        "windows" => "commands/windows.rs",
        _ => "commands/shared.rs",
    }
}

pub fn command_target(name: &str) -> &'static str {
    command_target_for_group(command_group(name))
}

fn walk_rs(root: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(walk_rs(&path)?);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::{command_group, command_target, extract_generate_handler_entries, extract_tauri_commands, render_tauri_commands};

    #[test]
    fn detects_tauri_command_attribute() {
        let commands = extract_tauri_commands("x.rs", "#[tauri::command]\nfn open_mod() {}");
        assert_eq!(commands[0].name, "open_mod");
    }

    #[test]
    fn extracts_generate_handler_entries() {
        let entries = extract_generate_handler_entries("generate_handler![commands::open_mod, close]");
        assert_eq!(entries, vec!["open_mod", "close"]);
    }

    #[test]
    fn groups_commands_by_domain() {
        assert_eq!(command_group("get_project_tree"), "project_tree");
        assert_eq!(command_group("clear_preview_cache"), "preview");
    }

    #[test]
    fn maps_reveal_scene_document_to_project_tree() {
        assert_eq!(command_target("reveal_scene_document"), "commands/project_tree.rs");
    }

    #[test]
    fn maps_open_settings_window_to_windows() {
        assert_eq!(command_target("open_settings_window"), "commands/windows.rs");
    }

    #[test]
    fn maps_cache_commands_to_cache() {
        assert_eq!(command_target("get_cache_info"), "commands/cache.rs");
    }

    #[test]
    fn maps_sheet_commands_to_sheets() {
        assert_eq!(command_target("load_sheet_resource"), "commands/sheets.rs");
    }

    #[test]
    fn snapshot_tauri_commands() {
        let commands = extract_tauri_commands(
            "commands/mod.rs",
            include_str!("../../tests/fixtures/move_plan/tauri_commands_mod.rs"),
        );
        let handler = "generate_handler![commands::reveal_scene_document, commands::read_project_file, commands::open_mod, commands::open_settings_window]";
        assert_eq!(
            render_tauri_commands(&commands, handler, 80).trim(),
            include_str!("../../tests/snapshots/tauri_commands.snap").trim()
        );
    }
}
