use crate::model::CodeMap;

pub fn print_tauri_graph(map: &CodeMap, limit: usize) {
    println!("tauri-graph:");
    for occurrence in map
        .text_occurrences
        .iter()
        .filter(|occurrence| {
            occurrence.kind == "command-name"
                || occurrence.value.contains("invoke")
                || occurrence.context.contains("#[tauri::command]")
        })
        .take(limit)
    {
        println!(
            "  {} {}:{} {}",
            occurrence.kind, occurrence.file_id, occurrence.line, occurrence.context
        );
    }

    println!("backend commands:");
    for symbol in map
        .symbols
        .iter()
        .filter(|symbol| symbol.tags.iter().any(|tag| tag == "domain:tauri-commands"))
        .take(limit)
    {
        println!("  {} {} {}", symbol.kind, symbol.name, symbol.signature);
    }
}
