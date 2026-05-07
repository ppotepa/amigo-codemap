use std::fs;
use std::path::Path;

use anyhow::{Result, bail};

use crate::model::CodeMap;
use crate::report::common::slash_path;

pub fn print_range_for_lines(
    root: &Path,
    map: &CodeMap,
    path: &Path,
    start_line: usize,
    end_line: usize,
    yaml_op: &str,
    context_radius: usize,
) -> Result<()> {
    let output = render_range_for_lines(
        root,
        map,
        path,
        start_line,
        end_line,
        yaml_op,
        context_radius,
    )?;
    print!("{output}");
    Ok(())
}

fn render_range_for_lines(
    root: &Path,
    map: &CodeMap,
    path: &Path,
    start_line: usize,
    end_line: usize,
    yaml_op: &str,
    context_radius: usize,
) -> Result<String> {
    if start_line == 0 || end_line < start_line {
        bail!("invalid range: {start_line}-{end_line}");
    }

    if yaml_op != "replace_range" && yaml_op != "delete_range" {
        bail!("unsupported --yaml-op `{yaml_op}`; use replace_range or delete_range");
    }

    let full = root.join(path);
    let text = fs::read_to_string(&full)?;
    let lines = text.lines().collect::<Vec<_>>();

    if end_line > lines.len() {
        bail!(
            "range outside file: {} has {} lines",
            path.display(),
            lines.len()
        );
    }

    let normalized_path = slash_path(path);
    let file_hash = map
        .files
        .iter()
        .find(|file| slash_path(&file.path) == normalized_path)
        .map(|file| file.hash.as_str())
        .unwrap_or_default();

    let before_start = start_line.saturating_sub(context_radius + 1);
    let before_end = start_line.saturating_sub(1);
    let after_start = end_line;
    let after_end = (end_line + context_radius).min(lines.len());

    let context_before = lines[before_start..before_end].join("\n");
    let context_after = lines[after_start..after_end].join("\n");
    let id_path = normalized_path
        .rsplit('/')
        .next()
        .unwrap_or("range")
        .replace('.', "-");

    let mut output = String::new();
    output.push_str(&format!(
        "- id: {}-{}-{}-{}\n",
        yaml_op.replace('_', "-"),
        id_path,
        start_line,
        end_line
    ));
    output.push_str(&format!("  kind: {yaml_op}\n"));
    output.push_str(&format!("  path: {normalized_path}\n"));
    output.push_str(&format!("  start_line: {start_line}\n"));
    output.push_str(&format!("  end_line: {end_line}\n"));
    if !file_hash.is_empty() {
        output.push_str(&format!("  expected_hash: \"{file_hash}\"\n"));
    }
    push_yaml_block(&mut output, "context_before", &context_before);
    push_yaml_block(&mut output, "context_after", &context_after);

    if yaml_op == "replace_range" {
        output.push_str("  content: |\n");
        output.push_str("    TODO\n");
    }

    Ok(output)
}

fn push_yaml_block(output: &mut String, key: &str, value: &str) {
    output.push_str(&format!("  {key}: |\n"));
    if value.is_empty() {
        output.push_str("    \n");
        return;
    }

    for line in value.lines() {
        output.push_str("    ");
        output.push_str(line);
        output.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::model::{CodeMap, FileEntry};

    use super::render_range_for_lines;

    static NEXT_TEMP_ID: AtomicUsize = AtomicUsize::new(0);

    fn temp_dir() -> PathBuf {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "amigo-codemap-range-for-lines-{}-{id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    #[test]
    fn range_for_lines_outputs_replace_range_yaml() {
        let temp = temp_dir();
        let path = std::path::Path::new("sample.ts");
        fs::write(
            temp.join(path),
            ["one", "two", "three", "four", "five", "six"].join("\n"),
        )
        .expect("write fixture");
        let map = CodeMap {
            files: vec![FileEntry {
                path: path.to_path_buf(),
                hash: "abc123".to_string(),
                ..FileEntry::default()
            }],
            ..CodeMap::default()
        };

        let output = render_range_for_lines(&temp, &map, path, 3, 4, "replace_range", 1)
            .expect("render yaml");

        assert!(output.contains("kind: replace_range"));
        assert!(output.contains("path: sample.ts"));
        assert!(output.contains("start_line: 3"));
        assert!(output.contains("end_line: 4"));
        assert!(output.contains("expected_hash: \"abc123\""));
        assert!(output.contains("context_before: |"));
        assert!(output.contains("    two"));
        assert!(output.contains("context_after: |"));
        assert!(output.contains("    five"));
        assert!(output.contains("content: |"));
    }

    #[test]
    fn range_for_lines_outputs_delete_range_yaml_without_content() {
        let temp = temp_dir();
        let path = std::path::Path::new("sample.ts");
        fs::write(temp.join(path), "one\ntwo\nthree\n").expect("write fixture");

        let output =
            render_range_for_lines(&temp, &CodeMap::default(), path, 1, 2, "delete_range", 1)
                .expect("render yaml");

        assert!(output.contains("kind: delete_range"));
        assert!(!output.contains("content: |"));
    }

    #[test]
    fn range_for_lines_rejects_invalid_range() {
        let temp = temp_dir();
        let path = std::path::Path::new("sample.ts");
        fs::write(temp.join(path), "one\ntwo\n").expect("write fixture");

        let error =
            render_range_for_lines(&temp, &CodeMap::default(), path, 0, 2, "replace_range", 1)
                .expect_err("invalid range should fail");

        assert!(error.to_string().contains("invalid range"));
    }
}
