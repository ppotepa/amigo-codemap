use super::model::{ExportEntry, ExportKind};
use std::path::Path;

pub fn parse_ts_exports(text: &str) -> Vec<ExportEntry> {
    let mut exports = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("export function ") {
            if let Some(name) = rest.split(['(', ':', '=', '<', '{']).next() {
                exports.push(ExportEntry {
                    source_file: Path::new("").to_path_buf(),
                    line: index + 1,
                    name: name.trim().to_string(),
                    kind: ExportKind::Named,
                });
            }
        } else if let Some(rest) = trimmed.strip_prefix("export const ") {
            if let Some(name) = rest.split(['=', ':', ';', ' ']).next() {
                exports.push(ExportEntry {
                    source_file: Path::new("").to_path_buf(),
                    line: index + 1,
                    name: name.trim().to_string(),
                    kind: ExportKind::Named,
                });
            }
        } else if let Some(rest) = trimmed.strip_prefix("export type ") {
            if let Some(name) = rest.split(['<', '=', ':', ' ']).next() {
                exports.push(ExportEntry {
                    source_file: Path::new("").to_path_buf(),
                    line: index + 1,
                    name: name.trim().to_string(),
                    kind: ExportKind::Named,
                });
            }
        } else if let Some(rest) = trimmed.strip_prefix("export interface ") {
            if let Some(name) = rest.split(['<', ' ']).next() {
                exports.push(ExportEntry {
                    source_file: Path::new("").to_path_buf(),
                    line: index + 1,
                    name: name.trim().to_string(),
                    kind: ExportKind::Named,
                });
            }
        } else if let Some(rest) = trimmed.strip_prefix("export default ") {
            if let Some(name) = rest.split([' ', '(']).next() {
                exports.push(ExportEntry {
                    source_file: Path::new("").to_path_buf(),
                    line: index + 1,
                    name: name.trim().to_string(),
                    kind: ExportKind::Default,
                });
            }
        } else if trimmed.starts_with("pub use ") {
            exports.push(ExportEntry {
                source_file: Path::new("").to_path_buf(),
                line: index + 1,
                name: "pub use".to_string(),
                kind: ExportKind::RustPubUse,
            });
        }
    }
    exports
}
