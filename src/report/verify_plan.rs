use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;

use crate::model::CodeMap;

use super::common::{is_codemap, is_docs, is_editor_frontend, is_editor_tauri, slash_path};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct VerifyPlan {
    pub frontend: bool,
    pub tauri: bool,
    pub codemap: bool,
    pub engine: bool,
    pub docs_only: bool,
    pub required: BTreeSet<String>,
    pub optional: BTreeSet<String>,
    pub skip: BTreeSet<String>,
    pub reason: Vec<String>,
}

pub fn plan_for_paths(paths: impl IntoIterator<Item = std::path::PathBuf>) -> VerifyPlan {
    let paths = paths.into_iter().collect::<Vec<_>>();
    let mut plan = VerifyPlan::default();
    let mut non_docs = false;

    for path in &paths {
        non_docs |= !is_docs(path);

        if is_editor_frontend(path) {
            plan.frontend = true;
            plan.required.insert("npm test".to_string());
            plan.required.insert("npm run build".to_string());
            plan.reason
                .push("TS/TSX changed under amigo-editor".to_string());
        } else if is_editor_tauri(path) {
            plan.tauri = true;
            plan.required
                .insert("cargo test -p amigo-editor --lib".to_string());
            plan.reason.push("Rust changed under src-tauri".to_string());
        } else if is_codemap(path) {
            plan.codemap = true;
            plan.required
                .insert("cargo test -p amigo-codemap".to_string());
            plan.required
                .insert("cargo build -p amigo-codemap".to_string());
            plan.reason.push("amigo-codemap source changed".to_string());
        } else if let Some(crate_name) = engine_test_crate(path) {
            plan.engine = true;
            plan.required.insert(format!("cargo test -p {crate_name}"));
            plan.reason
                .push(format!("engine crate changed: {crate_name}"));
        }
    }

    plan.docs_only = !paths.is_empty() && !non_docs;
    if plan.docs_only {
        plan.reason.push("markdown/docs only".to_string());
    }
    if !plan.codemap {
        plan.optional.insert("amigo-codemap compact".to_string());
    }
    plan.skip.insert("full cargo test workspace".to_string());
    if plan.reason.is_empty() {
        plan.reason
            .push("no known high-risk path matched".to_string());
    }
    plan.reason.sort();
    plan.reason.dedup();
    plan
}

fn engine_test_crate(path: &Path) -> Option<&'static str> {
    let text = slash_path(path);
    if text.starts_with("crates/engine/scene/") {
        Some("amigo-scene")
    } else if text.starts_with("crates/engine/render-wgpu/") {
        Some("amigo-render-wgpu")
    } else if text.starts_with("crates/engine/assets/") {
        Some("amigo-assets")
    } else if text.starts_with("crates/ui/core/") {
        Some("amigo-ui")
    } else {
        None
    }
}

pub fn plan_for_map(map: &CodeMap, changed_only: bool) -> VerifyPlan {
    let paths = if changed_only || !map.git.changed.is_empty() {
        map.git
            .changed
            .iter()
            .map(|change| change.path.clone())
            .collect::<Vec<_>>()
    } else {
        map.files
            .iter()
            .map(|file| file.path.clone())
            .collect::<Vec<_>>()
    };
    plan_for_paths(paths)
}

pub fn print_verify_plan(map: &CodeMap, changed_only: bool) {
    let plan = plan_for_map(map, changed_only);
    print!("{}", render_verify_plan(&plan));
}

pub fn render_verify_plan(plan: &VerifyPlan) -> String {
    let mut output = String::new();
    writeln!(output, "task: verify-plan").unwrap();
    writeln!(output, "changed:").unwrap();
    writeln!(output, "  frontend: {}", yes(plan.frontend)).unwrap();
    writeln!(output, "  tauri: {}", yes(plan.tauri)).unwrap();
    writeln!(output, "  codemap: {}", yes(plan.codemap)).unwrap();
    writeln!(output, "  engine: {}", yes(plan.engine)).unwrap();
    writeln!(output, "  docs: {}", yes(plan.docs_only)).unwrap();
    write_set(&mut output, "required", &plan.required);
    write_set(&mut output, "optional", &plan.optional);
    write_set(&mut output, "skip", &plan.skip);
    writeln!(output, "reason:").unwrap();
    for reason in &plan.reason {
        writeln!(output, "  {reason}").unwrap();
    }
    writeln!(output, "next:").unwrap();
    writeln!(output, "  1. run required checks").unwrap();
    writeln!(output, "  2. use fallout if build output is noisy").unwrap();
    output
}

fn write_set(output: &mut String, title: &str, values: &BTreeSet<String>) {
    writeln!(output, "{title}:").unwrap();
    if values.is_empty() {
        writeln!(output, "  none").unwrap();
    }
    for value in values {
        writeln!(output, "  {value}").unwrap();
    }
}

fn yes(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{plan_for_paths, render_verify_plan};

    #[test]
    fn plans_npm_build_for_editor_ts_change() {
        let plan = plan_for_paths([PathBuf::from("crates/apps/amigo-editor/src/app/a.ts")]);
        assert!(plan.required.contains("npm run build"));
        assert!(plan.required.contains("npm test"));
    }

    #[test]
    fn plans_editor_cargo_test_for_src_tauri_change() {
        let plan = plan_for_paths([PathBuf::from(
            "crates/apps/amigo-editor/src-tauri/src/lib.rs",
        )]);
        assert!(plan.required.contains("cargo test -p amigo-editor --lib"));
    }

    #[test]
    fn plans_codemap_tests_for_codemap_change() {
        let plan = plan_for_paths([PathBuf::from("crates/tools/amigo-codemap/src/main.rs")]);
        assert!(plan.required.contains("cargo test -p amigo-codemap"));
    }

    #[test]
    fn docs_only_for_markdown_change() {
        let plan = plan_for_paths([PathBuf::from("README.md")]);
        assert!(plan.docs_only);
    }

    #[test]
    fn snapshot_verify_plan_codemap_change() {
        let plan = plan_for_paths([PathBuf::from("crates/tools/amigo-codemap/src/main.rs")]);
        assert_eq!(
            render_verify_plan(&plan).trim(),
            include_str!("../../tests/snapshots/verify_plan_codemap_change.snap").trim()
        );
    }
}
