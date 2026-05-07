use anyhow::{Result, bail};

use crate::model::CodeMap;

pub fn print_callsite_candidates(map: &CodeMap, query: &str, limit: usize) -> Result<()> {
    if query.trim().is_empty() {
        bail!("callsite-candidates requires a symbol query");
    }

    println!("callsite-candidates: {query}");
    let needle_1 = format!("{query}(");
    let needle_2 = format!(".{query}(");
    let mut emitted = 0usize;
    for occurrence in &map.text_occurrences {
        if occurrence.value.contains(&needle_1)
            || occurrence.value.contains(&needle_2)
            || occurrence.context.contains(&needle_1)
            || occurrence.context.contains(&needle_2)
        {
            println!(
                "  {}:{} {}",
                occurrence.file_id, occurrence.line, occurrence.context
            );
            emitted += 1;
            if emitted >= limit {
                break;
            }
        }
    }
    if emitted == 0 {
        println!("  none");
    }
    Ok(())
}
