use crate::model::CodeMap;

pub fn print_component_graph(map: &CodeMap, limit: usize) {
    println!("component-graph:");
    for symbol in map
        .symbols
        .iter()
        .filter(|symbol| {
            symbol.kind == "component" || symbol.tags.iter().any(|tag| tag == "kind:component")
        })
        .take(limit)
    {
        println!(
            "  component {} {}:{}-{}",
            symbol.name, symbol.file_id, symbol.line, symbol.line_end
        );
        if !symbol.params.is_empty() {
            println!("    params: {}", symbol.params.join(", "));
        }
    }
}
