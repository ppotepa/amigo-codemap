use std::path::Path;

use regex::Regex;

use super::model::{ImportEntry, ImportKind};

pub fn parse_ts_imports(file: &Path, text: &str) -> Vec<ImportEntry> {
    let re = Regex::new(r#"^\s*(?:import|export)\s+.*?\s+from\s+["']([^"']+)["']"#).unwrap();
    let side_effect_re = Regex::new(r#"^\s*import\s+["']([^"']+)["']"#).unwrap();

    parse_named_imports(file, text, &re, &side_effect_re, ImportKind::TypeScript)
}

pub fn parse_rust_imports(file: &Path, text: &str) -> Vec<ImportEntry> {
    let use_re = Regex::new(r#"^\s*use\s+([^;\n]+);"#).unwrap();
    let mod_re = Regex::new(r#"^\s*(?:pub\s+)?mod\s+([A-Za-z0-9_]+)"#).unwrap();
    let mut imports = Vec::new();

    for (index, line) in text.lines().enumerate() {
        if let Some(capture) = use_re.captures(line) {
            imports.push(ImportEntry {
                source_file: file.to_path_buf(),
                line: index + 1,
                raw: line.trim().to_string(),
                specifier: capture[1].trim().to_string(),
                resolved: None,
                exists: false,
                kind: ImportKind::RustUse,
            });
            continue;
        }

        if let Some(capture) = mod_re.captures(line) {
            imports.push(ImportEntry {
                source_file: file.to_path_buf(),
                line: index + 1,
                raw: line.trim().to_string(),
                specifier: capture[1].trim().to_string(),
                resolved: None,
                exists: false,
                kind: ImportKind::RustMod,
            });
        }
    }
    imports
}

pub fn parse_imports(file: &Path, text: &str) -> Vec<ImportEntry> {
    let mut imports = parse_ts_imports(file, text);
    imports.extend(parse_rust_imports(file, text));
    imports
}

fn parse_named_imports(
    file: &Path,
    text: &str,
    import_re: &Regex,
    side_effect_re: &Regex,
    kind: ImportKind,
) -> Vec<ImportEntry> {
    let mut imports = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if let Some(capture) = import_re.captures(line) {
            imports.push(ImportEntry {
                source_file: file.to_path_buf(),
                line: index + 1,
                raw: line.trim().to_string(),
                specifier: capture[1].trim().to_string(),
                resolved: None,
                exists: false,
                kind,
            });
            continue;
        }
        if let Some(capture) = side_effect_re.captures(line) {
            let specifier = capture[1].trim().to_string();
            if !specifier.is_empty() {
                imports.push(ImportEntry {
                    source_file: file.to_path_buf(),
                    line: index + 1,
                    raw: line.trim().to_string(),
                    specifier,
                    resolved: None,
                    exists: false,
                    kind,
                });
            }
        }
    }
    imports
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::parse_ts_imports;

    #[test]
    fn parse_imports_from_ts() {
        let path = PathBuf::from("crates/apps/amigo-editor/src/app/store/editorStore.ts");
        let imports = parse_ts_imports(
            &path,
            r#"import { a } from "../x";
import "../styles.css";
export { y } from "./y";"#,
        );
        assert_eq!(imports.len(), 3);
        assert_eq!(imports[0].specifier, "../x");
    }
}
