use anyhow::Result;

use crate::model::CodeMap;

pub fn print_explain_file(map: &CodeMap, path_query: &str) -> Result<()> {
    let file = map
        .files
        .iter()
        .find(|file| file.path.to_string_lossy().replace('\\', "/") == path_query)
        .or_else(|| {
            map.files.iter().find(|file| {
                file.path
                    .to_string_lossy()
                    .replace('\\', "/")
                    .contains(path_query)
            })
        })
        .ok_or_else(|| anyhow::anyhow!("file not found: {path_query}"))?;

    println!("file: {}", file.path.to_string_lossy().replace('\\', "/"));
    println!("language: {}", file.language);
    println!("lines: {}", file.lines);
    println!("tags: {}", file.tags.join(","));

    println!("symbols:");
    for symbol in map
        .symbols
        .iter()
        .filter(|symbol| symbol.file_id == file.id)
        .take(20)
    {
        println!(
            "  {} {}:{}-{} {}",
            symbol.kind, symbol.name, symbol.line, symbol.line_end, symbol.signature
        );
    }

    println!("text occurrences:");
    for occurrence in map
        .text_occurrences
        .iter()
        .filter(|occurrence| occurrence.file_id == file.id)
        .take(20)
    {
        println!(
            "  {} {}:{} {}",
            occurrence.kind, occurrence.line, occurrence.column, occurrence.value
        );
    }
    Ok(())
}
