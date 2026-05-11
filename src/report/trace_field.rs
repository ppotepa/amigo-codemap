use std::path::Path;

use anyhow::{Result, bail};

use crate::model::CodeMap;

pub fn print_trace_field(root: &Path, map: &CodeMap, access: &str, limit: usize) -> Result<()> {
    if access.trim().is_empty() {
        bail!("trace-field requires a field access query");
    }

    println!("trace-field: {access}");
    println!(
        "snapshot: files={} symbols={} text_occurrences={}",
        map.files.len(),
        map.symbols.len(),
        map.text_occurrences.len()
    );

    let mut emitted = 0usize;
    for file in &map.files {
        if emitted >= limit {
            break;
        }
        if !matches!(file.language.as_str(), "rs" | "ts" | "tsx") {
            continue;
        }
        let full = root.join(&file.path);
        let Ok(text) = std::fs::read_to_string(&full) else {
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            if !line.contains(access) {
                continue;
            }
            emitted += 1;
            println!("{}\t{}\t{}", file.path.display(), index + 1, line.trim());
            if emitted >= limit {
                break;
            }
        }
    }

    if emitted == 0 {
        println!("no field access matches");
    }

    Ok(())
}
