use crate::model::CodeMap;

pub fn print_api_surface(map: &CodeMap, limit: usize) {
    println!("api-surface:");
    for symbol in map
        .symbols
        .iter()
        .filter(|symbol| matches!(symbol.visibility.as_str(), "pub" | "export"))
        .take(limit)
    {
        println!(
            "  {} {} {}:{}-{} {}",
            symbol.visibility,
            symbol.kind,
            symbol.file_id,
            symbol.line,
            symbol.line_end,
            symbol.signature
        );
    }
}
