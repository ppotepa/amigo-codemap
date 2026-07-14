use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Result, bail};
use regex::Regex;

use crate::model::CodeMap;

use super::common::slash_path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PathClass {
    Live,
    TestSupport,
    DocsContent,
    Tooling,
}

impl PathClass {
    fn label(self) -> &'static str {
        match self {
            Self::Live => "live runtime/backend path",
            Self::TestSupport => "tests/support path",
            Self::DocsContent => "docs/content path",
            Self::Tooling => "tooling path",
        }
    }
}

#[derive(Debug, Clone)]
struct Violation {
    rule_id: &'static str,
    path: String,
    line: usize,
    class: PathClass,
    excerpt: String,
}

#[derive(Debug, Clone)]
struct Rule {
    id: &'static str,
    regexes: Vec<Regex>,
    applies_to: fn(&str) -> bool,
    allow: fn(&str) -> bool,
}

pub fn run(root: &Path, map: &CodeMap) -> Result<()> {
    let violations = collect_violations(root, map)?;
    print!("{}", render_report(map, &violations));
    if !violations.is_empty() {
        bail!("arch-guard found {} violation(s)", violations.len());
    }
    Ok(())
}

fn collect_violations(root: &Path, map: &CodeMap) -> Result<Vec<Violation>> {
    let mut violations =
        workspace_coverage_violations(map, &fs::read_to_string(root.join("plan.md"))?);
    let rules = rules()?;

    for file in &map.files {
        let path = slash_path(&file.path);
        if !is_text_path(&path) {
            continue;
        }
        let full_path = root.join(&file.path);
        let Ok(text) = fs::read_to_string(&full_path) else {
            continue;
        };
        for rule in &rules {
            if !(rule.applies_to)(&path) || (rule.allow)(&path) {
                continue;
            }
            for (line_index, line) in text.lines().enumerate() {
                if rule.regexes.iter().any(|regex| regex.is_match(line)) {
                    violations.push(Violation {
                        rule_id: rule.id,
                        path: path.clone(),
                        line: line_index + 1,
                        class: classify_path(&path),
                        excerpt: line.trim().to_owned(),
                    });
                }
            }
        }
    }

    violations.sort_by(|left, right| {
        left.rule_id
            .cmp(right.rule_id)
            .then_with(|| left.class.cmp(&right.class))
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line.cmp(&right.line))
    });
    Ok(violations)
}

fn render_report(map: &CodeMap, violations: &[Violation]) -> String {
    let mut out = String::new();
    use std::fmt::Write as _;

    writeln!(out, "task: arch-guard").unwrap();
    writeln!(out, "repo: {}", map.root_name).unwrap();
    writeln!(out, "workspace-packages: {}", map.packages.len()).unwrap();
    writeln!(out, "violations: {}", violations.len()).unwrap();

    if violations.is_empty() {
        writeln!(out, "status: ok").unwrap();
        return out;
    }

    writeln!(out, "status: fail").unwrap();
    let grouped = group_by_rule(violations);
    writeln!(out, "rules:").unwrap();
    for (rule_id, entries) in &grouped {
        let description = entries
            .first()
            .and_then(|entry| rule_description(entry.rule_id))
            .unwrap_or("-");
        let live = entries
            .iter()
            .filter(|entry| entry.class == PathClass::Live)
            .count();
        let test = entries
            .iter()
            .filter(|entry| entry.class == PathClass::TestSupport)
            .count();
        let docs = entries
            .iter()
            .filter(|entry| entry.class == PathClass::DocsContent)
            .count();
        let tooling = entries
            .iter()
            .filter(|entry| entry.class == PathClass::Tooling)
            .count();
        writeln!(
            out,
            "  {rule_id}: total={} live={} tests={} docs={} tooling={} :: {}",
            entries.len(),
            live,
            test,
            docs,
            tooling,
            description
        )
        .unwrap();
    }

    writeln!(out, "findings:").unwrap();
    for (rule_id, entries) in grouped {
        writeln!(out, "  [{rule_id}]").unwrap();
        let mut class_groups = BTreeMap::<PathClass, Vec<&Violation>>::new();
        for entry in &entries {
            class_groups.entry(entry.class).or_default().push(entry);
        }
        for (class, class_entries) in class_groups {
            writeln!(out, "    {}:", class.label()).unwrap();
            for entry in class_entries.into_iter().take(8) {
                writeln!(out, "      {}:{} {}", entry.path, entry.line, entry.excerpt).unwrap();
            }
        }
    }

    writeln!(out, "next:").unwrap();
    writeln!(out, "  1. fix live runtime/backend path violations first").unwrap();
    writeln!(
        out,
        "  2. update audits/architecture-scorecard.md with current counts"
    )
    .unwrap();
    writeln!(out, "  3. keep tests/docs/content exceptions explicit").unwrap();
    out
}

fn group_by_rule<'a>(violations: &'a [Violation]) -> BTreeMap<&'static str, Vec<&'a Violation>> {
    let mut grouped = BTreeMap::<&'static str, Vec<&Violation>>::new();
    for violation in violations {
        grouped
            .entry(violation.rule_id)
            .or_default()
            .push(violation);
    }
    grouped
}

fn workspace_coverage_violations(packages: &CodeMap, plan_text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    for package in packages
        .packages
        .iter()
        .filter(|package| package.manifest_path.ends_with("Cargo.toml"))
    {
        let package_path = slash_path(
            package
                .manifest_path
                .parent()
                .unwrap_or_else(|| package.manifest_path.as_path()),
        );
        let has_path = plan_text.contains(&package_path);
        let has_name = plan_text.contains(&package.name);
        if !has_path || !has_name {
            violations.push(Violation {
                rule_id: "workspace-coverage-matrix",
                path: package_path,
                line: 0,
                class: PathClass::DocsContent,
                excerpt: format!(
                    "missing workspace coverage row for package={} has_path={} has_name={}",
                    package.name, has_path, has_name
                ),
            });
        }
    }
    violations
}

fn rules() -> Result<Vec<Rule>> {
    Ok(vec![
        Rule {
            id: "scene-component-enum",
            regexes: regexes(&[
                r"\benum SceneComponentDocument\b",
                concat!(r"SceneComponentDocument", r"::"),
                r"\bis_builtin_type\b",
                r"\bis_rejected_legacy_type\b",
            ])?,
            applies_to: |path| path.starts_with("crates/") || path.starts_with("plugins/"),
            allow: is_migration_doc_or_summary,
        },
        Rule {
            id: "component-kind-central",
            regexes: regexes(&[
                r"\bComponentKind\b",
                r"\bbuiltin_renderable_2d_component_kinds\b",
                r"\bsupported_renderable_2d_component_kinds\b",
            ])?,
            applies_to: |path| path.starts_with("crates/") || path.starts_with("plugins/"),
            allow: is_migration_doc_or_summary,
        },
        Rule {
            id: "scene-command-queue",
            regexes: regexes(&[
                concat!(r"SceneCommand", r"::Queue"),
                r"Queue[A-Z][A-Za-z0-9_]*SceneCommand",
                r"\bPluginSceneCommandHandlerRegistry\b",
            ])?,
            applies_to: |path| path.starts_with("crates/") || path.starts_with("plugins/"),
            allow: is_migration_doc_or_summary,
        },
        Rule {
            id: "postfx-central-dispatch",
            regexes: regexes(&[
                r"\benum PostFx2d\b",
                concat!(r"PostFx2d", r"::"),
                r"\bdefault_post_fx_executor_registry\b",
                r"\bapply_cached_image_post_fx_rgba\b",
            ])?,
            applies_to: |path| path.starts_with("crates/") || path.starts_with("plugins/"),
            allow: is_migration_doc_or_summary,
        },
        Rule {
            id: "render-wgpu-scene-backedge",
            regexes: regexes(&[r"\bSceneService\b", r"\bamigo-scene\b", r"\bAssetCatalog\b"])?,
            applies_to: |path| path.starts_with("crates/engine/render-wgpu/"),
            allow: |_| false,
        },
        Rule {
            id: "app-render-request-boundary",
            regexes: regexes(&[
                r"\bWgpuFrameRenderRequest\b",
                r"\bWgpuWorld2dRenderInput\b",
                r"\bWgpuWorld3dRenderInput\b",
                r"\bcollect_camera_optical_candidates_from_light_sources_2d\b",
                r"\bbuild_render_scene_view\b",
            ])?,
            applies_to: |path| path.starts_with("crates/apps/app/src/"),
            allow: is_test_path,
        },
        Rule {
            id: "domain-string-guessing",
            regexes: regexes(&[
                r#""Sprite2D""#,
                r#""Text2D""#,
                r#""VectorShape2D""#,
                r#""ParticleEmitter2D""#,
                r#""TileMap2D""#,
                r#""lightmap:"#,
                r#"":lightmap:"#,
            ])?,
            applies_to: |path| {
                path.starts_with("crates/engine/")
                    || path.starts_with("crates/runtime/")
                    || path.starts_with("crates/apps/")
                    || path.starts_with("crates/platform/")
            },
            allow: is_test_path,
        },
        Rule {
            id: "migration-markers",
            regexes: regexes(&[r"\blegacy\b", r"\bv2\b", r"\bcompat\b", r"\bfallback\b"])?,
            applies_to: |path| path.starts_with("crates/") || path.starts_with("plugins/"),
            allow: |path| {
                is_test_path(path)
                    || path.ends_with("plan.md")
                    || path.ends_with("plan.summary.md")
                    || path.starts_with("docs/")
                    || path.starts_with("audits/")
            },
        },
    ])
}

fn regexes(patterns: &[&str]) -> Result<Vec<Regex>> {
    patterns
        .iter()
        .map(|pattern| Regex::new(pattern).map_err(Into::into))
        .collect()
}

fn rule_description(rule_id: &str) -> Option<&'static str> {
    match rule_id {
        "workspace-coverage-matrix" => {
            Some("every Cargo workspace member must appear in plan coverage")
        }
        "scene-component-enum" => {
            Some("central SceneComponentDocument enum and helpers should be removed")
        }
        "component-kind-central" => Some("central component kind lists should be removed"),
        "scene-command-queue" => {
            Some("domain Queue commands should not remain in central scene command path")
        }
        "postfx-central-dispatch" => {
            Some("central PostFx enum and executor registry should be removed")
        }
        "render-wgpu-scene-backedge" => {
            Some("render-wgpu should not depend on scene/runtime asset semantics directly")
        }
        "app-render-request-boundary" => {
            Some("apps/app should host render submission, not assemble renderer request internals")
        }
        "domain-string-guessing" => {
            Some("core/runtime/backend/app should not guess domain intent from component strings")
        }
        "migration-markers" => Some("old migration markers should not remain in live code"),
        _ => None,
    }
}

fn classify_path(path: &str) -> PathClass {
    if path.starts_with("docs/")
        || path.starts_with("audits/")
        || path.starts_with("mods/")
        || path.ends_with(".md")
        || path.ends_with(".txt")
    {
        PathClass::DocsContent
    } else if is_test_path(path) {
        PathClass::TestSupport
    } else if path.starts_with("crates/tools/") {
        PathClass::Tooling
    } else {
        PathClass::Live
    }
}

fn is_text_path(path: &str) -> bool {
    matches!(
        path.rsplit('.').next(),
        Some(
            "rs" | "md" | "toml" | "yml" | "yaml" | "rhai" | "txt" | "json" | "js" | "ts" | "html"
        )
    )
}

fn is_test_path(path: &str) -> bool {
    path.contains("/tests/")
        || path.ends_with("_tests.rs")
        || path.ends_with("tests.rs")
        || path.contains("/src/tests/")
}

fn is_migration_doc_or_summary(path: &str) -> bool {
    path == "plan.md"
        || path == "plan.summary.md"
        || path.starts_with("audits/")
        || path.starts_with("docs/")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::model::{CodeMap, PackageEntry};

    use super::{classify_path, is_test_path, workspace_coverage_violations};

    #[test]
    fn workspace_coverage_reports_missing_package_rows() {
        let mut map = CodeMap::default();
        map.packages.push(PackageEntry {
            name: "amigo-test".to_string(),
            manifest_path: PathBuf::from("crates/test/Cargo.toml"),
            ..PackageEntry::default()
        });

        let violations = workspace_coverage_violations(&map, "no package rows here");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule_id, "workspace-coverage-matrix");
    }

    #[test]
    fn classify_test_paths_before_live_paths() {
        assert!(is_test_path(
            "crates/engine/scene/tests/architecture_regressions.rs"
        ));
        assert_eq!(
            classify_path("crates/engine/scene/tests/architecture_regressions.rs").label(),
            "tests/support path"
        );
    }
}
