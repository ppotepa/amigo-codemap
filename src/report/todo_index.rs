use crate::model::CodeMap;

pub fn print_todo_index(map: &CodeMap, limit: usize) {
    println!("todo-index:");
    let markers = ["TODO", "FIXME", "HACK", "@codemap", "risk"];
    let mut emitted = 0usize;
    for occurrence in &map.text_occurrences {
        if markers
            .iter()
            .any(|marker| occurrence.context.contains(marker))
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
}
