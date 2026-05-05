use anyhow::{Result, bail};

use crate::model::CodeMap;

use super::common::{print_next, symbols_matching, text_refs};

pub fn classify(defs: usize, refs: usize, changed_refs: usize) -> &'static str {
    match (defs, refs, changed_refs) {
        (0, 0, _) => "clean",
        (_, 0, _) => "safe-delete candidate",
        (_, refs, changed) if refs == changed => "cleanup in progress",
        _ => "migration incomplete",
    }
}

pub fn print_stale(
    root: &std::path::Path,
    map: &CodeMap,
    patterns: &[String],
    changed_only: bool,
    limit: usize,
) -> Result<()> {
    if patterns.is_empty() {
        bail!("stale requires --patterns a,b");
    }
    println!("task: stale");
    println!("patterns: {}", patterns.len());
    let mut clean = 0usize;
    let mut inspect = 0usize;
    let mut blocked = 0usize;
    for pattern in patterns {
        let defs = symbols_matching(map, pattern);
        let mut refs = text_refs(root, map, pattern, limit)?;
        refs.retain(|item| {
            !defs
                .iter()
                .any(|symbol| symbol.file_id == item.file_id && item.lines.iter().any(|(line, _)| *line == symbol.line))
        });
        if changed_only {
            refs.retain(|item| item.changed);
        }
        let changed_refs = refs.iter().filter(|item| item.changed).count();
        let status = classify(defs.len(), refs.len(), changed_refs);
        match status {
            "clean" | "safe-delete candidate" => clean += 1,
            "cleanup in progress" => inspect += 1,
            _ => blocked += 1,
        }
        println!();
        println!("{pattern}:");
        println!("  refs: {}", refs.len());
        println!("  definition: {}", if defs.is_empty() { "none" } else { "yes" });
        if !refs.is_empty() {
            println!("  files:");
            for item in refs.iter().take(limit) {
                let suffix = if item.changed { " changed" } else { "" };
                println!("    {}{}", item.path, suffix);
            }
        }
        println!("  status: {status}");
    }
    println!();
    println!("summary:");
    println!("  clean: {clean}");
    println!("  inspect: {inspect}");
    println!("  blocked: {blocked}");
    print_next(&["remove clean leftovers", "inspect unchanged references", "rerun stale after cleanup"]);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::classify;

    #[test]
    fn classifies_zero_refs_as_clean() {
        assert_eq!(classify(0, 0, 0), "clean");
    }

    #[test]
    fn classifies_refs_in_unchanged_files_as_blocked() {
        assert_eq!(classify(1, 2, 1), "migration incomplete");
    }

    #[test]
    fn classifies_refs_in_changed_files_as_in_progress() {
        assert_eq!(classify(1, 2, 2), "cleanup in progress");
    }
}
