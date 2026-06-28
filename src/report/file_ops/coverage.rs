use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use flate2::read::DeflateDecoder;

use crate::model::CodeMap;

pub fn print_coverage(root: &Path, map: &CodeMap, artifact: &Path, limit: usize) -> Result<()> {
    let artifact_path = if artifact.is_absolute() {
        artifact.to_path_buf()
    } else {
        root.join(artifact)
    };
    let text = read_artifact_text(&artifact_path, artifact)?;
    let artifact_files = extract_artifact_paths(&text);
    let repo_files = map
        .files
        .iter()
        .map(|file| normalize_path(&file.path))
        .collect::<BTreeSet<_>>();

    let missing = repo_files
        .difference(&artifact_files)
        .cloned()
        .collect::<Vec<_>>();
    let extra = artifact_files
        .difference(&repo_files)
        .cloned()
        .collect::<Vec<_>>();

    println!("coverage: {}", artifact.display());
    println!("  repo_files: {}", repo_files.len());
    println!("  artifact_files: {}", artifact_files.len());
    println!("  missing_from_artifact: {}", missing.len());
    println!("  extra_in_artifact: {}", extra.len());

    print_extension_summary("missing_by_ext", &missing);

    println!("missing:");
    for path in missing.iter().take(limit) {
        println!("  {path}");
    }
    if missing.is_empty() {
        println!("  none");
    }

    Ok(())
}

fn read_artifact_text(path: &Path, display_path: &Path) -> Result<String> {
    let bytes = fs::read(path)?;
    if path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
    {
        return read_zip_artifact_text(&bytes, display_path);
    }
    String::from_utf8(bytes).map_err(|_| {
        anyhow::anyhow!(
            "coverage expects UTF-8 text artifact; `{}` is binary or not UTF-8",
            display_path.display()
        )
    })
}

fn read_zip_artifact_text(bytes: &[u8], display_path: &Path) -> Result<String> {
    let entries = zip_entries(bytes)?;
    let mut text = String::new();

    for entry in entries {
        if looks_like_repo_path(&entry.name) {
            text.push_str("FILE: ");
            text.push_str(&entry.name);
            text.push('\n');
        }
        match entry.method {
            0 => {
                let data = zip_entry_data(bytes, &entry)?;
                append_utf8_entry_text(&mut text, data);
            }
            8 => {
                let data = zip_entry_data(bytes, &entry)?;
                let mut decoder = DeflateDecoder::new(data);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed)?;
                append_utf8_entry_text(&mut text, &decompressed);
            }
            _ => {}
        }
    }

    if text.trim().is_empty() {
        bail!(
            "coverage did not find UTF-8 concat text or repo-path entries in `{}`",
            display_path.display()
        );
    }
    Ok(text)
}

fn append_utf8_entry_text(out: &mut String, data: &[u8]) {
    if let Ok(entry_text) = std::str::from_utf8(data) {
        out.push_str(entry_text);
        if !entry_text.ends_with('\n') {
            out.push('\n');
        }
    }
}

#[derive(Debug)]
struct ZipEntry {
    name: String,
    method: u16,
    compressed_size: usize,
    local_header_offset: usize,
}

fn zip_entries(bytes: &[u8]) -> Result<Vec<ZipEntry>> {
    let eocd = find_zip_eocd(bytes)?;
    let entry_count = read_u16(bytes, eocd + 10)? as usize;
    let central_dir_size = read_u32(bytes, eocd + 12)? as usize;
    let central_dir_offset = read_u32(bytes, eocd + 16)? as usize;
    let central_dir_end = central_dir_offset
        .checked_add(central_dir_size)
        .filter(|end| *end <= bytes.len())
        .ok_or_else(|| anyhow::anyhow!("invalid zip central directory bounds"))?;

    let mut entries = Vec::new();
    let mut cursor = central_dir_offset;
    for _ in 0..entry_count {
        if cursor + 46 > central_dir_end || read_u32(bytes, cursor)? != 0x0201_4b50 {
            bail!("invalid zip central directory entry");
        }
        let method = read_u16(bytes, cursor + 10)?;
        let compressed_size = read_u32(bytes, cursor + 20)? as usize;
        let name_len = read_u16(bytes, cursor + 28)? as usize;
        let extra_len = read_u16(bytes, cursor + 30)? as usize;
        let comment_len = read_u16(bytes, cursor + 32)? as usize;
        let local_header_offset = read_u32(bytes, cursor + 42)? as usize;
        let name_start = cursor + 46;
        let name_end = name_start
            .checked_add(name_len)
            .filter(|end| *end <= central_dir_end)
            .ok_or_else(|| anyhow::anyhow!("invalid zip entry name bounds"))?;
        let name = String::from_utf8_lossy(&bytes[name_start..name_end]).replace('\\', "/");
        entries.push(ZipEntry {
            name,
            method,
            compressed_size,
            local_header_offset,
        });
        cursor = name_end
            .checked_add(extra_len)
            .and_then(|value| value.checked_add(comment_len))
            .filter(|next| *next <= central_dir_end)
            .ok_or_else(|| anyhow::anyhow!("invalid zip entry bounds"))?;
    }

    Ok(entries)
}

fn zip_entry_data<'a>(bytes: &'a [u8], entry: &ZipEntry) -> Result<&'a [u8]> {
    let offset = entry.local_header_offset;
    if offset + 30 > bytes.len() || read_u32(bytes, offset)? != 0x0403_4b50 {
        bail!("invalid zip local file header");
    }
    let name_len = read_u16(bytes, offset + 26)? as usize;
    let extra_len = read_u16(bytes, offset + 28)? as usize;
    let data_start = offset
        .checked_add(30)
        .and_then(|value| value.checked_add(name_len))
        .and_then(|value| value.checked_add(extra_len))
        .ok_or_else(|| anyhow::anyhow!("invalid zip local data offset"))?;
    let data_end = data_start
        .checked_add(entry.compressed_size)
        .filter(|end| *end <= bytes.len())
        .ok_or_else(|| anyhow::anyhow!("invalid zip local data bounds"))?;
    Ok(&bytes[data_start..data_end])
}

fn find_zip_eocd(bytes: &[u8]) -> Result<usize> {
    let min = bytes.len().saturating_sub(65_557);
    for index in (min..bytes.len().saturating_sub(3)).rev() {
        if bytes[index..].starts_with(&[0x50, 0x4b, 0x05, 0x06]) {
            return Ok(index);
        }
    }
    bail!("invalid zip archive: missing end of central directory")
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16> {
    let data = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| anyhow::anyhow!("unexpected end of zip data"))?;
    Ok(u16::from_le_bytes([data[0], data[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    let data = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| anyhow::anyhow!("unexpected end of zip data"))?;
    Ok(u32::from_le_bytes([data[0], data[1], data[2], data[3]]))
}

fn extract_artifact_paths(text: &str) -> BTreeSet<String> {
    text.lines()
        .filter_map(extract_artifact_path)
        .map(|path| normalize_text_path(&path))
        .filter(|path| !path.is_empty())
        .collect()
}

fn extract_artifact_path(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.starts_with("<<<FILE ") {
        return extract_codecat_file_path(trimmed).map(clean_marker_path);
    }
    if let Some((_, rest)) = trimmed.split_once("FILE:") {
        return Some(clean_marker_path(rest));
    }
    if looks_like_repo_path(trimmed) {
        return Some(clean_marker_path(trimmed));
    }
    None
}

fn extract_codecat_file_path(marker: &str) -> Option<&str> {
    let start = marker.find("path=\"")? + "path=\"".len();
    let rest = marker.get(start..)?;
    let end = rest.find('"')?;
    rest.get(..end)
}

fn clean_marker_path(value: &str) -> String {
    value
        .trim()
        .trim_matches('=')
        .trim_matches('-')
        .trim_matches('#')
        .trim()
        .trim_matches('`')
        .trim()
        .to_string()
}

fn looks_like_repo_path(value: &str) -> bool {
    let value = value.replace('\\', "/");
    matches!(
        value.split('/').next(),
        Some("crates" | "plugins" | "mods" | "apps" | "config" | "docs" | "assets" | "tools")
    ) && value.contains('.')
}

fn print_extension_summary(label: &str, paths: &[String]) {
    let mut counts = BTreeMap::<String, usize>::new();
    for path in paths {
        let ext = Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("<none>")
            .to_ascii_lowercase();
        *counts.entry(ext).or_default() += 1;
    }
    let mut counts = counts.into_iter().collect::<Vec<_>>();
    counts.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));

    println!("{label}:");
    if counts.is_empty() {
        println!("  none");
    } else {
        for (ext, count) in counts.into_iter().take(20) {
            println!("  .{ext}: {count}");
        }
    }
}

fn normalize_path(path: &Path) -> String {
    normalize_text_path(&path.to_string_lossy())
}

fn normalize_text_path(path: &str) -> String {
    PathBuf::from(path.replace('\\', "/"))
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_stored_zip_concat_entry() {
        let text = b"FILE: crates/tools/amigo-codemap/src/cache.rs\n";
        let name = b"concat.txt";
        let mut zip = Vec::new();

        let local_offset = zip.len() as u32;
        push_u32(&mut zip, 0x0403_4b50);
        push_u16(&mut zip, 20);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 0);
        push_u32(&mut zip, 0);
        push_u32(&mut zip, text.len() as u32);
        push_u32(&mut zip, text.len() as u32);
        push_u16(&mut zip, name.len() as u16);
        push_u16(&mut zip, 0);
        zip.extend_from_slice(name);
        zip.extend_from_slice(text);

        let central_offset = zip.len() as u32;
        push_u32(&mut zip, 0x0201_4b50);
        push_u16(&mut zip, 20);
        push_u16(&mut zip, 20);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 0);
        push_u32(&mut zip, 0);
        push_u32(&mut zip, text.len() as u32);
        push_u32(&mut zip, text.len() as u32);
        push_u16(&mut zip, name.len() as u16);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 0);
        push_u32(&mut zip, 0);
        push_u32(&mut zip, local_offset);
        zip.extend_from_slice(name);

        let central_size = zip.len() as u32 - central_offset;
        push_u32(&mut zip, 0x0605_4b50);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 0);
        push_u16(&mut zip, 1);
        push_u16(&mut zip, 1);
        push_u32(&mut zip, central_size);
        push_u32(&mut zip, central_offset);
        push_u16(&mut zip, 0);

        let artifact = read_zip_artifact_text(&zip, Path::new("concat.zip")).unwrap();
        assert!(artifact.contains("crates/tools/amigo-codemap/src/cache.rs"));
    }

    #[test]
    fn extracts_codecat_file_markers() {
        let text = r#"
CODECAT_VERSION: 1
<<<FILE path="crates/tools/amigo-codemap/src/cache.rs" plugin="rust" lang="rs">>>
fn example() {}
<<<END_FILE>>>
"#;
        let paths = extract_artifact_paths(text);
        assert!(paths.contains("crates/tools/amigo-codemap/src/cache.rs"));
    }

    fn push_u16(out: &mut Vec<u8>, value: u16) {
        out.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u32(out: &mut Vec<u8>, value: u32) {
        out.extend_from_slice(&value.to_le_bytes());
    }
}
