use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use regex::Regex;

use crate::model::{DependencyEntry, FileEntry, SymbolEntry};

pub fn scan_symbols(root: &Path, files: &[FileEntry], level: u8) -> Result<Vec<SymbolEntry>> {
    let rust_patterns = RustPatterns::new()?;
    let ts_patterns = TsPatterns::new()?;
    let yaml_patterns = YamlPatterns::new()?;
    let mut symbols = Vec::new();

    for file in files {
        match file.language.as_str() {
            "rs" => symbols.extend(scan_rust(root, file, level, &rust_patterns)?),
            "ts" | "tsx" => symbols.extend(scan_ts(root, file, level, &ts_patterns)?),
            "css" if level >= 2 => symbols.extend(scan_css(root, file)?),
            "yaml" | "yml" => symbols.extend(scan_yaml(root, file, &yaml_patterns)?),
            "rhai" => symbols.extend(scan_rhai(root, file)?),
            _ => {}
        }
    }

    Ok(symbols)
}

pub fn scan_dependencies(
    root: &Path,
    files: &[FileEntry],
    file_ids: &BTreeMap<PathBuf, String>,
) -> Result<Vec<DependencyEntry>> {
    let mut deps = Vec::new();
    for file in files {
        match file.language.as_str() {
            "ts" | "tsx" => deps.extend(scan_ts_imports(root, file, file_ids)?),
            "rs" => deps.extend(scan_rust_mods(root, file, file_ids)?),
            _ => {}
        }
    }
    deps.sort_by(|a, b| (&a.from, &a.to, &a.kind).cmp(&(&b.from, &b.to, &b.kind)));
    deps.dedup();
    Ok(deps)
}

pub fn scan_ai_relations(
    root: &Path,
    files: &[FileEntry],
    file_ids: &BTreeMap<PathBuf, String>,
) -> Result<Vec<DependencyEntry>> {
    let mut deps = scan_test_candidate_relations(root, files, file_ids)?;
    deps.sort_by(|a, b| (&a.from, &a.to, &a.kind).cmp(&(&b.from, &b.to, &b.kind)));
    deps.dedup();
    Ok(deps)
}

fn scan_rust(
    root: &Path,
    file: &FileEntry,
    level: u8,
    patterns: &RustPatterns,
) -> Result<Vec<SymbolEntry>> {
    let text = fs::read_to_string(root.join(&file.path))?;
    let mut symbols = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            continue;
        }
        for regex in &patterns.items {
            if let Some(caps) = regex.captures(trimmed) {
                let visibility = caps
                    .name("vis")
                    .map(|m| m.as_str().trim().to_string())
                    .filter(|vis| vis == "pub")
                    .unwrap_or_else(|| "local".to_string());
                if level == 1 && visibility != "pub" {
                    continue;
                }
                symbols.push(SymbolEntry {
                    name: caps["name"].to_string(),
                    kind: caps["kind"].to_string(),
                    file_id: file.id.clone(),
                    line: line_index + 1,
                    visibility,
                });
                break;
            }
        }
    }
    Ok(symbols)
}

fn scan_ts(
    root: &Path,
    file: &FileEntry,
    level: u8,
    patterns: &TsPatterns,
) -> Result<Vec<SymbolEntry>> {
    let text = fs::read_to_string(root.join(&file.path))?;
    let mut symbols = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            continue;
        }
        for regex in &patterns.items {
            if let Some(caps) = regex.captures(trimmed) {
                let visibility = if caps.name("export").is_some() {
                    "export"
                } else {
                    "local"
                };
                if level == 1 && visibility != "export" {
                    continue;
                }
                let name = caps["name"].to_string();
                let mut kind = caps["kind"].to_string();
                if kind == "const" || kind == "function" {
                    kind = classify_ts_name(&name, &file.language);
                }
                symbols.push(SymbolEntry {
                    name,
                    kind,
                    file_id: file.id.clone(),
                    line: line_index + 1,
                    visibility: visibility.to_string(),
                });
                break;
            }
        }
    }
    Ok(symbols)
}

fn scan_yaml(root: &Path, file: &FileEntry, patterns: &YamlPatterns) -> Result<Vec<SymbolEntry>> {
    let text = fs::read_to_string(root.join(&file.path))?;
    let mut symbols = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        for (kind, regex) in &patterns.items {
            if let Some(caps) = regex.captures(line.trim()) {
                symbols.push(SymbolEntry {
                    name: caps["name"].to_string(),
                    kind: kind.to_string(),
                    file_id: file.id.clone(),
                    line: line_index + 1,
                    visibility: "yaml".to_string(),
                });
                break;
            }
        }
    }
    Ok(symbols)
}

fn scan_rhai(root: &Path, file: &FileEntry) -> Result<Vec<SymbolEntry>> {
    let text = fs::read_to_string(root.join(&file.path))?;
    let regex = Regex::new(r"^fn\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*\(")?;
    let mut symbols = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        if let Some(caps) = regex.captures(line.trim_start()) {
            symbols.push(SymbolEntry {
                name: caps["name"].to_string(),
                kind: "fn".to_string(),
                file_id: file.id.clone(),
                line: line_index + 1,
                visibility: "rhai".to_string(),
            });
        }
    }
    Ok(symbols)
}

fn scan_css(root: &Path, file: &FileEntry) -> Result<Vec<SymbolEntry>> {
    let text = fs::read_to_string(root.join(&file.path))?;
    let mut symbols = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let Some((selector_text, _)) = line.split_once('{') else {
            continue;
        };
        let selector_text = selector_text.trim();
        if selector_text.is_empty() || selector_text.starts_with('@') {
            continue;
        }
        for selector in selector_text.split(',') {
            let selector = selector.trim();
            if selector.is_empty() {
                continue;
            }
            symbols.push(SymbolEntry {
                name: selector.to_string(),
                kind: "css-selector".to_string(),
                file_id: file.id.clone(),
                line: line_index + 1,
                visibility: "css".to_string(),
            });
        }
    }
    Ok(symbols)
}

fn scan_ts_imports(
    root: &Path,
    file: &FileEntry,
    file_ids: &BTreeMap<PathBuf, String>,
) -> Result<Vec<DependencyEntry>> {
    let text = fs::read_to_string(root.join(&file.path))?;
    let import_re = Regex::new(
        r#"from\s+["'](?P<path>[^"']+)["']|import\s*\(\s*["'](?P<dynamic>[^"']+)["']\s*\)"#,
    )?;
    let mut deps = Vec::new();
    for caps in import_re.captures_iter(&text) {
        let import_path = caps
            .name("path")
            .or_else(|| caps.name("dynamic"))
            .map(|m| m.as_str())
            .unwrap_or_default();
        if !import_path.starts_with('.') {
            continue;
        }
        if let Some(target) = resolve_ts_import(&file.path, import_path, file_ids) {
            deps.push(DependencyEntry {
                from: file.id.clone(),
                to: target,
                kind: "imports".to_string(),
            });
        }
    }
    Ok(deps)
}

fn scan_rust_mods(
    root: &Path,
    file: &FileEntry,
    file_ids: &BTreeMap<PathBuf, String>,
) -> Result<Vec<DependencyEntry>> {
    let text = fs::read_to_string(root.join(&file.path))?;
    let mod_re = Regex::new(r"^\s*(?:pub\s+)?mod\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*;")?;
    let mut deps = Vec::new();
    for caps in mod_re.captures_iter(&text) {
        let name = &caps["name"];
        if let Some(target) = resolve_rust_mod(&file.path, name, file_ids) {
            deps.push(DependencyEntry {
                from: file.id.clone(),
                to: target,
                kind: "declares".to_string(),
            });
        }
    }
    Ok(deps)
}

fn scan_test_candidate_relations(
    root: &Path,
    files: &[FileEntry],
    file_ids: &BTreeMap<PathBuf, String>,
) -> Result<Vec<DependencyEntry>> {
    let mut deps = Vec::new();
    for file in files {
        let path = slash_path(&file.path);
        if file.language == "rs" {
            let text = fs::read_to_string(root.join(&file.path))?;
            if text.contains("#[cfg(test)]") || text.contains("#[test]") {
                deps.push(DependencyEntry {
                    from: file.id.clone(),
                    to: file.id.clone(),
                    kind: "test-candidate:in-file".to_string(),
                });
            }
            continue;
        }
        if !(path.ends_with(".test.ts") || path.ends_with(".test.tsx")) {
            continue;
        }
        for source in test_source_candidates(&file.path) {
            if let Some(source_id) = file_ids.get(&source) {
                deps.push(DependencyEntry {
                    from: source_id.clone(),
                    to: file.id.clone(),
                    kind: "test-candidate".to_string(),
                });
            }
        }
    }
    Ok(deps)
}

fn test_source_candidates(test_path: &Path) -> Vec<PathBuf> {
    let path = slash_path(test_path);
    let source_paths = [
        path.strip_suffix(".test.ts")
            .map(|base| format!("{base}.ts")),
        path.strip_suffix(".test.ts")
            .map(|base| format!("{base}.tsx")),
        path.strip_suffix(".test.tsx")
            .map(|base| format!("{base}.ts")),
        path.strip_suffix(".test.tsx")
            .map(|base| format!("{base}.tsx")),
    ];
    source_paths
        .into_iter()
        .flatten()
        .map(PathBuf::from)
        .collect()
}

fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn resolve_ts_import(
    from: &Path,
    import_path: &str,
    file_ids: &BTreeMap<PathBuf, String>,
) -> Option<String> {
    let parent = from.parent().unwrap_or_else(|| Path::new(""));
    let base = normalize(parent.join(import_path));
    let candidates = [
        base.clone(),
        base.with_extension("ts"),
        base.with_extension("tsx"),
        base.with_extension("js"),
        base.with_extension("jsx"),
        base.join("index.ts"),
        base.join("index.tsx"),
    ];
    candidates
        .iter()
        .find_map(|candidate| file_ids.get(candidate).cloned())
}

fn resolve_rust_mod(
    from: &Path,
    name: &str,
    file_ids: &BTreeMap<PathBuf, String>,
) -> Option<String> {
    let parent = from.parent().unwrap_or_else(|| Path::new(""));
    let candidates = [
        parent.join(format!("{name}.rs")),
        parent.join(name).join("mod.rs"),
    ];
    candidates
        .iter()
        .map(|candidate| normalize(candidate.clone()))
        .find_map(|candidate| file_ids.get(&candidate).cloned())
}

fn normalize(path: PathBuf) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::CurDir => {}
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn classify_ts_name(name: &str, language: &str) -> String {
    if name.starts_with("use") && name.chars().nth(3).is_some_and(char::is_uppercase) {
        return "hook".to_string();
    }
    if language == "tsx" && name.chars().next().is_some_and(char::is_uppercase) {
        return "component".to_string();
    }
    "fn".to_string()
}

struct RustPatterns {
    items: Vec<Regex>,
}

impl RustPatterns {
    fn new() -> Result<Self> {
        Ok(Self {
            items: vec![
                Regex::new(
                    r"^(?P<vis>pub\s+)?(?:const\s+)?(?P<kind>fn)\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
                )?,
                Regex::new(
                    r"^(?P<vis>pub\s+)?(?P<kind>struct|enum|trait|mod|type|const)\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
                )?,
                Regex::new(
                    r"^(?P<vis>pub\s+)?(?P<kind>impl)\s+(?:<[^>]+>\s+)?(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
                )?,
                Regex::new(
                    r"^(?P<vis>pub\s+)?(?P<kind>macro_rules!)\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
                )?,
            ],
        })
    }
}

struct TsPatterns {
    items: Vec<Regex>,
}

impl TsPatterns {
    fn new() -> Result<Self> {
        Ok(Self {
            items: vec![
                Regex::new(
                    r"^(?P<export>export\s+)?(?P<kind>function)\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
                )?,
                Regex::new(
                    r"^(?P<export>export\s+)?(?P<kind>const|let|var)\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*=",
                )?,
                Regex::new(
                    r"^(?P<export>export\s+)?(?P<kind>type|interface|enum|class)\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
                )?,
            ],
        })
    }
}

struct YamlPatterns {
    items: Vec<(&'static str, Regex)>,
}

impl YamlPatterns {
    fn new() -> Result<Self> {
        Ok(Self {
            items: vec![
                (
                    "scene",
                    Regex::new(r#"^(?:id|scene|scene_id):\s*['"]?(?P<name>[A-Za-z0-9_.:-]+)"#)?,
                ),
                (
                    "mod",
                    Regex::new(r#"^(?:mod|mod_id|package):\s*['"]?(?P<name>[A-Za-z0-9_.:-]+)"#)?,
                ),
                (
                    "asset",
                    Regex::new(
                        r#"^(?:asset|asset_id|source):\s*['"]?(?P<name>[A-Za-z0-9_./:-]+)"#,
                    )?,
                ),
            ],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::classify_ts_name;

    #[test]
    fn classifies_tsx_components_and_hooks() {
        assert_eq!(classify_ts_name("StartupDialog", "tsx"), "component");
        assert_eq!(classify_ts_name("useEditorStore", "tsx"), "hook");
        assert_eq!(classify_ts_name("scanMods", "ts"), "fn");
    }
}
