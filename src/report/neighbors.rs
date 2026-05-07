use anyhow::Result;

use crate::model::CodeMap;

pub fn print_neighbors(map: &CodeMap, path_query: &str, limit: usize) -> Result<()> {
    let Some(file) = map.files.iter().find(|file| {
        file.path
            .to_string_lossy()
            .replace('\\', "/")
            .contains(path_query)
    }) else {
        anyhow::bail!("file not found: {path_query}");
    };

    println!(
        "neighbors: {}",
        file.path.to_string_lossy().replace('\\', "/")
    );
    println!("dependencies:");
    for dep in map
        .dependencies
        .iter()
        .filter(|dep| dep.from == file.id || dep.to == file.id)
        .take(limit)
    {
        println!("  {} -> {} kind={}", dep.from, dep.to, dep.kind);
    }

    println!("relations:");
    for rel in map
        .relations
        .iter()
        .filter(|rel| rel.from == file.id || rel.to == file.id)
        .take(limit)
    {
        println!(
            "  {} -> {} kind={} confidence={}",
            rel.from, rel.to, rel.kind, rel.confidence
        );
    }

    println!("same-domain files:");
    for tag in file.tags.iter().filter(|tag| tag.starts_with("domain:")) {
        for other in map
            .files
            .iter()
            .filter(|other| other.id != file.id && other.tags.contains(tag))
            .take(limit)
        {
            println!("  {}", other.path.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(())
}
