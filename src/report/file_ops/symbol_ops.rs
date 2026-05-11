use std::fs;
use std::path::Path;

use anyhow::{Result, bail};

use crate::model::{CodeMap, SymbolEntry};

pub fn replace_symbol(
    root: &Path,
    map: &CodeMap,
    path: &Path,
    symbol_name: &str,
    content: &str,
    write: bool,
) -> Result<()> {
    let symbol = resolve_symbol(map, path, symbol_name)?;
    replace_line_range(
        root,
        path,
        symbol.line,
        symbol.line_end,
        Some(content),
        write,
    )
}

pub fn delete_symbol(
    root: &Path,
    map: &CodeMap,
    path: &Path,
    symbol_name: &str,
    write: bool,
) -> Result<()> {
    let symbol = resolve_symbol(map, path, symbol_name)?;
    replace_line_range(root, path, symbol.line, symbol.line_end, None, write)
}

pub fn insert_before_symbol(
    root: &Path,
    map: &CodeMap,
    path: &Path,
    symbol_name: &str,
    content: &str,
    write: bool,
) -> Result<()> {
    let symbol = resolve_symbol(map, path, symbol_name)?;
    insert_at_line(root, path, symbol.line, content, write)
}

pub fn insert_after_symbol(
    root: &Path,
    map: &CodeMap,
    path: &Path,
    symbol_name: &str,
    content: &str,
    write: bool,
) -> Result<()> {
    let symbol = resolve_symbol(map, path, symbol_name)?;
    insert_at_line(
        root,
        path,
        symbol.line_end.saturating_add(1),
        content,
        write,
    )
}

pub fn replace_method_body(
    root: &Path,
    map: &CodeMap,
    path: &Path,
    symbol_name: &str,
    content: &str,
    write: bool,
) -> Result<()> {
    let symbol = resolve_symbol(map, path, symbol_name)?;
    let text = fs::read_to_string(root.join(path))?;
    let lines = text.lines().collect::<Vec<_>>();

    let start_index = symbol.line.saturating_sub(1);
    let end_index = symbol.line_end.min(lines.len());
    let open_line = (start_index..end_index)
        .find(|index| lines[*index].contains('{'))
        .map(|index| index + 1);
    let close_line = (start_index..end_index)
        .rev()
        .find(|index| lines[*index].contains('}'))
        .map(|index| index + 1);

    let Some(open_line) = open_line else {
        bail!("method body open brace not found for {symbol_name}");
    };
    let Some(close_line) = close_line else {
        bail!("method body close brace not found for {symbol_name}");
    };
    if close_line <= open_line {
        bail!("invalid method body range for {symbol_name}");
    }

    replace_line_range(
        root,
        path,
        open_line + 1,
        close_line - 1,
        Some(content),
        write,
    )
}

pub fn replace_field_access(
    root: &Path,
    map: &CodeMap,
    path: Option<&Path>,
    find: &str,
    replace: &str,
    scope: Option<&str>,
    expected_matches: Option<usize>,
    write: bool,
) -> Result<()> {
    let files: Vec<&Path> = if let Some(path) = path {
        vec![path]
    } else if scope == Some("changed") {
        map.git.changed.iter().map(|change| change.path.as_path()).collect()
    } else {
        map.files.iter().map(|file| file.path.as_path()).collect()
    };

    let mut matches = 0usize;
    for file_path in &files {
        let full = root.join(file_path);
        let text = fs::read_to_string(&full)?;
        matches += text.matches(find).count();
    }

    if let Some(expected) = expected_matches {
        if matches != expected {
            bail!(
                "replace_field_access expected {} matches for {}, got {}",
                expected,
                find,
                matches
            );
        }
    } else if matches == 0 {
        bail!("replace_field_access locator not found: {find}");
    }

    for file_path in files {
        let full = root.join(file_path);
        let text = fs::read_to_string(&full)?;
        if !text.contains(find) {
            continue;
        }
        let next = text.replacen(find, replace, expected_matches.unwrap_or(1));
        if write {
            fs::write(full, next)?;
        } else {
            println!("{next}");
        }
    }

    Ok(())
}

fn resolve_symbol<'a>(map: &'a CodeMap, path: &Path, symbol_name: &str) -> Result<&'a SymbolEntry> {
    Ok(super::symbol_locator::resolve_symbol_in_file(map, path, symbol_name)?.symbol)
}

fn replace_line_range(
    root: &Path,
    path: &Path,
    start_line: usize,
    end_line: usize,
    content: Option<&str>,
    write: bool,
) -> Result<()> {
    if start_line == 0 || end_line < start_line {
        bail!(
            "invalid range {}:{}-{}",
            path.display(),
            start_line,
            end_line
        );
    }

    let full = root.join(path);
    let text = fs::read_to_string(&full)?;
    let mut output = String::new();
    for (index, line) in text.lines().enumerate() {
        let line_no = index + 1;
        if line_no == start_line
            && let Some(content) = content
        {
            output.push_str(content.trim_end());
            output.push('\n');
        }
        if line_no < start_line || line_no > end_line {
            output.push_str(line);
            output.push('\n');
        }
    }

    if write {
        fs::write(full, output)?;
    } else {
        println!("{output}");
    }
    Ok(())
}

fn insert_at_line(root: &Path, path: &Path, line: usize, content: &str, write: bool) -> Result<()> {
    let full = root.join(path);
    let text = fs::read_to_string(&full)?;
    let mut output = String::new();
    for (index, existing) in text.lines().enumerate() {
        let line_no = index + 1;
        if line_no == line {
            output.push_str(content.trim_end());
            output.push('\n');
        }
        output.push_str(existing);
        output.push('\n');
    }
    if line > text.lines().count() {
        output.push_str(content.trim_end());
        output.push('\n');
    }

    if write {
        fs::write(full, output)?;
    } else {
        println!("{output}");
    }
    Ok(())
}




