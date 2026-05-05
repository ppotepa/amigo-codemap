use std::collections::BTreeMap;
use std::fmt::Write as _;

use anyhow::Result;
use regex::Regex;

use super::common::{read_to_string, sorted_counts};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildError {
    pub file: Option<String>,
    pub code: String,
    pub kind: String,
    pub message: String,
}

pub fn parse_fallout(input: &str) -> Vec<BuildError> {
    let ts = Regex::new(
        r"(?:(?P<file>[^()\s]+\.(?:ts|tsx))\(\d+,\d+\):\s*)?error (?P<code>TS\d+): (?P<msg>.+)",
    )
    .unwrap();
    let rust = Regex::new(r"error\[(?P<code>E\d+)\]: (?P<msg>.+)").unwrap();
    let loc = Regex::new(r"-->\s+(?P<file>[^:]+):\d+:\d+").unwrap();
    let mut pending_rust: Option<BuildError> = None;
    let mut errors = Vec::new();
    for line in input.lines() {
        if let Some(caps) = ts.captures(line) {
            errors.push(BuildError {
                file: caps.name("file").map(|m| m.as_str().replace('\\', "/")),
                code: caps["code"].to_string(),
                kind: classify(&caps["code"]).to_string(),
                message: caps["msg"].to_string(),
            });
        } else if let Some(caps) = rust.captures(line) {
            if let Some(error) = pending_rust.take() {
                errors.push(error);
            }
            pending_rust = Some(BuildError {
                file: None,
                code: caps["code"].to_string(),
                kind: classify(&caps["code"]).to_string(),
                message: caps["msg"].to_string(),
            });
        } else if let (Some(caps), Some(error)) = (loc.captures(line), pending_rust.as_mut()) {
            error.file = Some(caps["file"].replace('\\', "/"));
        }
    }
    if let Some(error) = pending_rust {
        errors.push(error);
    }
    errors
}

pub fn classify(code: &str) -> &'static str {
    match code {
        "TS2304" => "missing name/import",
        "TS2305" => "missing export/import",
        "TS2322" | "E0308" => "type mismatch",
        "TS2339" => "missing property",
        "TS2345" => "argument mismatch",
        "E0432" => "unresolved import",
        "E0425" => "missing function/value",
        "E0412" => "missing type",
        "E0603" => "visibility",
        _ => "other",
    }
}

pub fn print_fallout(from: Option<&std::path::PathBuf>, limit: usize) -> Result<()> {
    let input = read_to_string(from)?;
    print!("{}", render_fallout(&input, limit));
    Ok(())
}

pub fn render_fallout(input: &str, limit: usize) -> String {
    let errors = parse_fallout(input);
    let mut output = String::new();
    writeln!(output, "task: fallout").unwrap();
    writeln!(output, "errors: {}", errors.len()).unwrap();
    let mut kinds = BTreeMap::new();
    let mut files = BTreeMap::new();
    for error in &errors {
        *kinds.entry(error.kind.clone()).or_default() += 1;
        *files
            .entry(error.file.clone().unwrap_or_else(|| "unknown".to_string()))
            .or_default() += 1;
    }
    writeln!(output, "groups:").unwrap();
    for (kind, count) in sorted_counts(kinds).into_iter().take(limit) {
        writeln!(output, "  {kind}: {count}").unwrap();
    }
    writeln!(output, "files:").unwrap();
    for (file, count) in sorted_counts(files).into_iter().take(limit) {
        writeln!(output, "  {file}: {count}").unwrap();
        writeln!(output, "    likely cause: {}", likely_cause(&file, &errors)).unwrap();
    }
    writeln!(output, "next:").unwrap();
    writeln!(output, "  1. fix missing imports/exports first").unwrap();
    writeln!(output, "  2. fix type-shape errors").unwrap();
    writeln!(output, "  3. rerun the failing command").unwrap();
    output
}

fn likely_cause(file: &str, errors: &[BuildError]) -> &'static str {
    if file.contains("commands/mod.rs") {
        "missing re-export after split"
    } else if errors
        .iter()
        .any(|e| e.message.contains("WorkspaceRuntimeServices"))
    {
        "service-shape mismatch"
    } else if errors
        .iter()
        .any(|e| e.message.contains("EditorSelectionRef"))
    {
        "selection migration fallout"
    } else {
        "inspect local compile error"
    }
}

#[cfg(test)]
mod tests {
    use super::{classify, likely_cause, parse_fallout, render_fallout};

    #[test]
    fn parses_tsc_missing_name() {
        let errors = parse_fallout("src/a.ts(1,2): error TS2304: Cannot find name 'X'.");
        assert_eq!(errors[0].kind, "missing name/import");
    }

    #[test]
    fn parses_rust_unresolved_import() {
        assert_eq!(classify("E0432"), "unresolved import");
    }

    #[test]
    fn fixture_parses_tsc_missing_import() {
        let errors = parse_fallout(include_str!(
            "../../tests/fixtures/fallout/tsc_missing_import.txt"
        ));
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].kind, "missing export/import");
        assert_eq!(
            errors[0].file.as_deref(),
            Some("src/main-window/MainEditorWindow.tsx")
        );
        assert_eq!(
            likely_cause("src/main-window/MainEditorWindow.tsx", &errors),
            "service-shape mismatch"
        );
    }

    #[test]
    fn fixture_parses_cargo_unresolved_import() {
        let errors = parse_fallout(include_str!(
            "../../tests/fixtures/fallout/cargo_unresolved_import.txt"
        ));
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, "unresolved import");
        assert_eq!(
            errors[0].file.as_deref(),
            Some("crates/apps/amigo-editor/src-tauri/src/commands/mod.rs")
        );
        assert_eq!(
            likely_cause(
                "crates/apps/amigo-editor/src-tauri/src/commands/mod.rs",
                &errors
            ),
            "missing re-export after split"
        );
    }

    #[test]
    fn fixture_parses_mixed_build_log() {
        let errors = parse_fallout(include_str!(
            "../../tests/fixtures/fallout/mixed_build_log.txt"
        ));
        assert_eq!(errors.len(), 3);
        assert_eq!(
            errors
                .iter()
                .filter(|error| error.kind == "missing property")
                .count(),
            1
        );
        assert_eq!(
            errors
                .iter()
                .filter(|error| error.kind == "visibility")
                .count(),
            1
        );
    }

    #[test]
    fn snapshot_fallout_tsc_missing_import() {
        assert_eq!(
            render_fallout(
                include_str!("../../tests/fixtures/fallout/tsc_missing_import.txt"),
                80
            )
            .trim(),
            include_str!("../../tests/snapshots/fallout_tsc_missing_import.snap").trim()
        );
    }
}
