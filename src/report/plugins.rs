use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

const CANONICAL_FILES: &[&str] = &[
    "plugin.toml",
    "README.md",
    "src/lib.rs",
    "src/plugin.rs",
    "src/api/mod.rs",
    "src/scene/mod.rs",
    "src/participation/mod.rs",
    "src/runtime/mod.rs",
    "src/render_wgpu/mod.rs",
    "src/scripting/mod.rs",
    "src/diagnostics/mod.rs",
    "tests/waterfall_tests.rs",
    "docs/pipeline.md",
    "docs/contributions.md",
    "docs/diagnostics.md",
];

#[derive(Debug)]
struct PluginFolder {
    family: String,
    name: String,
    path: PathBuf,
    manifest: ManifestSummary,
    missing: Vec<&'static str>,
}

#[derive(Debug, Default)]
struct ManifestSummary {
    id: Option<String>,
    kind: Option<String>,
    provides: Vec<String>,
    requires: Vec<String>,
    implements_slots: Vec<String>,
    required_slots: Vec<String>,
    target_reads: Vec<String>,
    target_writes: Vec<String>,
    target_contributions: Vec<String>,
    emits: Vec<String>,
    consumes: Vec<String>,
    diagnostics: Vec<String>,
}

pub fn print_plugins_report(root: &Path, limit: usize) -> Result<()> {
    let plugins = discover_plugins(root)?;
    let limit = limit.max(1);

    println!("plugin graph:");
    println!("  families={}", family_count(&plugins));
    println!("  plugins={}", plugins.len());
    for plugin in plugins.iter().take(limit) {
        println!(
            "  {} -> {} ({})",
            plugin.family,
            plugin.manifest.id.as_deref().unwrap_or(&plugin.name),
            slash_path(&plugin.path)
        );
    }

    print_manifest_section(
        "capability graph",
        &plugins,
        |manifest| (&manifest.provides, &manifest.requires),
        limit,
    );
    print_slot_graph(&plugins, limit);
    print_target_graph(&plugins, limit);
    print_contribution_graph(&plugins, limit);
    print_diagnostics_graph(&plugins, limit);
    print_missing_canonical_files(&plugins, limit);
    print_orphans(&plugins);

    Ok(())
}

fn discover_plugins(root: &Path) -> Result<Vec<PluginFolder>> {
    let plugins_root = root.join("plugins");
    let mut plugins = Vec::new();

    if !plugins_root.exists() {
        return Ok(plugins);
    }

    for family_entry in fs::read_dir(&plugins_root)? {
        let family_entry = family_entry?;
        if !family_entry.file_type()?.is_dir() {
            continue;
        }
        let family = family_entry.file_name().to_string_lossy().to_string();

        for plugin_entry in fs::read_dir(family_entry.path())? {
            let plugin_entry = plugin_entry?;
            if !plugin_entry.file_type()?.is_dir() {
                continue;
            }
            let path = plugin_entry.path();
            let manifest_path = path.join("plugin.toml");
            if !manifest_path.exists() {
                continue;
            }

            let name = plugin_entry.file_name().to_string_lossy().to_string();
            let manifest_text = fs::read_to_string(&manifest_path).unwrap_or_default();
            let manifest = parse_manifest_summary(&manifest_text);
            let missing = CANONICAL_FILES
                .iter()
                .copied()
                .filter(|relative| !path.join(relative).exists())
                .collect();

            plugins.push(PluginFolder {
                family: family.clone(),
                name,
                path: path.strip_prefix(root).unwrap_or(&path).to_path_buf(),
                manifest,
                missing,
            });
        }
    }

    plugins.sort_by(|left, right| {
        left.family
            .cmp(&right.family)
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(plugins)
}

fn parse_manifest_summary(text: &str) -> ManifestSummary {
    let mut summary = ManifestSummary::default();
    let mut section = "";

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line.trim_matches(['[', ']']);
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();

        match (section, key) {
            ("", "id") => summary.id = scalar(value),
            ("", "kind") => summary.kind = scalar(value),
            ("capabilities", "provides") => summary.provides = array(value),
            ("capabilities", "requires") => summary.requires = array(value),
            ("slots", "implements") => summary.implements_slots = array(value),
            ("slots", "requires") => summary.required_slots = array(value),
            ("targets", "reads") => summary.target_reads = array(value),
            ("targets", "writes") => summary.target_writes = array(value),
            ("targets", "contributes") => summary.target_contributions = array(value),
            ("contributions", "emits") => summary.emits = array(value),
            ("contributions", "consumes") => summary.consumes = array(value),
            ("diagnostics", "channels") => summary.diagnostics = array(value),
            _ => {}
        }
    }

    summary
}

fn scalar(value: &str) -> Option<String> {
    let value = value.trim().trim_matches('"');
    (!value.is_empty()).then(|| value.to_owned())
}

fn array(value: &str) -> Vec<String> {
    value
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .filter_map(scalar)
        .collect()
}

fn family_count(plugins: &[PluginFolder]) -> usize {
    plugins
        .iter()
        .map(|plugin| plugin.family.as_str())
        .collect::<BTreeSet<_>>()
        .len()
}

fn print_manifest_section(
    title: &str,
    plugins: &[PluginFolder],
    values: impl Fn(&ManifestSummary) -> (&Vec<String>, &Vec<String>),
    limit: usize,
) {
    println!("{title}:");
    let mut emitted = 0usize;
    for plugin in plugins {
        let (left, right) = values(&plugin.manifest);
        for value in left {
            if emitted >= limit {
                return;
            }
            emitted += 1;
            println!("  {} provides {}", plugin_label(plugin), value);
        }
        for value in right {
            if emitted >= limit {
                return;
            }
            emitted += 1;
            println!("  {} requires {}", plugin_label(plugin), value);
        }
    }
    if emitted == 0 {
        println!("  none");
    }
}

fn print_slot_graph(plugins: &[PluginFolder], limit: usize) {
    println!("slot graph:");
    print_edges(
        plugins,
        limit,
        |manifest| &manifest.implements_slots,
        "implements",
    );
    print_edges(
        plugins,
        limit,
        |manifest| &manifest.required_slots,
        "requires",
    );
}

fn print_target_graph(plugins: &[PluginFolder], limit: usize) {
    println!("target graph:");
    print_edges(plugins, limit, |manifest| &manifest.target_reads, "reads");
    print_edges(plugins, limit, |manifest| &manifest.target_writes, "writes");
    print_edges(
        plugins,
        limit,
        |manifest| &manifest.target_contributions,
        "contributes",
    );
}

fn print_contribution_graph(plugins: &[PluginFolder], limit: usize) {
    println!("contribution graph:");
    print_edges(plugins, limit, |manifest| &manifest.emits, "emits");
    print_edges(plugins, limit, |manifest| &manifest.consumes, "consumes");
}

fn print_diagnostics_graph(plugins: &[PluginFolder], limit: usize) {
    println!("diagnostics graph:");
    print_edges(plugins, limit, |manifest| &manifest.diagnostics, "produces");
}

fn print_edges(
    plugins: &[PluginFolder],
    limit: usize,
    values: impl Fn(&ManifestSummary) -> &Vec<String>,
    verb: &str,
) {
    let mut emitted = 0usize;
    for plugin in plugins {
        for value in values(&plugin.manifest) {
            if emitted >= limit {
                return;
            }
            emitted += 1;
            println!("  {} {} {}", plugin_label(plugin), verb, value);
        }
    }
    if emitted == 0 {
        println!("  none");
    }
}

fn print_missing_canonical_files(plugins: &[PluginFolder], limit: usize) {
    println!("missing canonical files:");
    let mut emitted = 0usize;
    for plugin in plugins {
        for missing in &plugin.missing {
            if emitted >= limit {
                return;
            }
            emitted += 1;
            println!("  {} missing {}", plugin_label(plugin), missing);
        }
    }
    if emitted == 0 {
        println!("  none");
    }
}

fn print_orphans(plugins: &[PluginFolder]) {
    let mut readers = BTreeMap::<&str, usize>::new();
    let mut writers = BTreeMap::<&str, usize>::new();
    let mut contributions = BTreeMap::<&str, usize>::new();
    let mut consumers = BTreeMap::<&str, usize>::new();

    for plugin in plugins {
        for target in &plugin.manifest.target_reads {
            *readers.entry(target).or_default() += 1;
        }
        for target in &plugin.manifest.target_writes {
            *writers.entry(target).or_default() += 1;
        }
        for target in &plugin.manifest.target_contributions {
            *writers.entry(target).or_default() += 1;
        }
        for contribution in &plugin.manifest.emits {
            *contributions.entry(contribution).or_default() += 1;
        }
        for contribution in &plugin.manifest.consumes {
            *consumers.entry(contribution).or_default() += 1;
        }
    }

    let orphan_targets = readers
        .keys()
        .filter(|target| !writers.contains_key(**target))
        .copied()
        .collect::<Vec<_>>();
    let unused_contributions = contributions
        .keys()
        .filter(|contribution| !consumers.contains_key(**contribution))
        .copied()
        .collect::<Vec<_>>();

    println!("orphan targets:");
    if orphan_targets.is_empty() {
        println!("  none");
    } else {
        for target in orphan_targets {
            println!("  {target}");
        }
    }

    println!("unused contributions:");
    if unused_contributions.is_empty() {
        println!("  none");
    } else {
        for contribution in unused_contributions {
            println!("  {contribution}");
        }
    }
}

fn plugin_label(plugin: &PluginFolder) -> &str {
    plugin.manifest.id.as_deref().unwrap_or(&plugin.name)
}

fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::parse_manifest_summary;

    #[test]
    fn manifest_summary_reads_targets_and_diagnostics() {
        let summary = parse_manifest_summary(
            r#"
id = "amigo.postfx.composite"
kind = "target-consumer"

[capabilities]
provides = ["postfx.composite@1"]
requires = ["render.backend@1"]

[slots]
implements = ["postfx.composite"]
requires = ["render.backend"]

[targets]
reads = ["SceneColor", "CameraArtifactLayer"]
writes = ["FinalComposite"]
contributes = ["DiagnosticsSnapshot"]

[contributions]
emits = ["camera.optics"]
consumes = ["lighting.light"]

[diagnostics]
channels = ["postfx.composite"]
"#,
        );

        assert_eq!(summary.id.as_deref(), Some("amigo.postfx.composite"));
        assert_eq!(summary.kind.as_deref(), Some("target-consumer"));
        assert_eq!(summary.provides, ["postfx.composite@1"]);
        assert_eq!(summary.requires, ["render.backend@1"]);
        assert_eq!(summary.implements_slots, ["postfx.composite"]);
        assert_eq!(summary.required_slots, ["render.backend"]);
        assert_eq!(summary.target_reads, ["SceneColor", "CameraArtifactLayer"]);
        assert_eq!(summary.target_writes, ["FinalComposite"]);
        assert_eq!(summary.target_contributions, ["DiagnosticsSnapshot"]);
        assert_eq!(summary.emits, ["camera.optics"]);
        assert_eq!(summary.consumes, ["lighting.light"]);
        assert_eq!(summary.diagnostics, ["postfx.composite"]);
    }
}
