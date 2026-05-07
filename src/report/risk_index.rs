use crate::model::CodeMap;

pub fn print_risk_index(map: &CodeMap, limit: usize) {
    println!("risk-index:");
    for file in map
        .files
        .iter()
        .filter(|file| {
            file.tags.iter().any(|tag| tag.starts_with("risk:"))
                || file.lines > 500
                || file.tags.iter().any(|tag| tag == "state:changed")
        })
        .take(limit)
    {
        println!(
            "  {} lines={} tags={}",
            file.path.to_string_lossy().replace('\\', "/"),
            file.lines,
            file.tags.join(",")
        );
    }
}
