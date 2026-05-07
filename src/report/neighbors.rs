use anyhow::Result;

use crate::model::CodeMap;
use crate::report::anchors::anchor_priority_score;

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

    let file_anchors = map
        .tags
        .iter()
        .filter(|tag| tag.file_id == file.id)
        .collect::<Vec<_>>();

    println!("anchors:");
    if file_anchors.is_empty() {
        println!("  none");
    } else {
        for tag in file_anchors.iter().take(limit) {
            println!(
                "  {} {} domain={} role={} line={}",
                tag.priority.as_deref().unwrap_or("P2"),
                tag.anchor,
                tag.domain.as_deref().unwrap_or("unknown"),
                tag.role.as_deref().unwrap_or("file"),
                tag.line
            );
        }
    }

    let mut anchor_neighbors = map
        .tags
        .iter()
        .filter(|candidate| candidate.file_id != file.id)
        .filter(|candidate| {
            file_anchors.iter().any(|tag| {
                tag.domain.is_some() && tag.domain == candidate.domain
                    || tag.role.is_some() && tag.role == candidate.role
            })
        })
        .collect::<Vec<_>>();

    anchor_neighbors.sort_by(|left, right| {
        anchor_priority_score(right.priority.as_deref().unwrap_or("P2"))
            .cmp(&anchor_priority_score(
                left.priority.as_deref().unwrap_or("P2"),
            ))
            .then_with(|| left.anchor.cmp(&right.anchor))
    });

    println!("anchor-neighbors:");
    if anchor_neighbors.is_empty() {
        println!("  none");
    } else {
        for tag in anchor_neighbors.into_iter().take(limit) {
            println!(
                "  {} {} domain={} role={} file_id={}:{}",
                tag.priority.as_deref().unwrap_or("P2"),
                tag.anchor,
                tag.domain.as_deref().unwrap_or("unknown"),
                tag.role.as_deref().unwrap_or("file"),
                tag.file_id,
                tag.line
            );
        }
    }
    Ok(())
}
