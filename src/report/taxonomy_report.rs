use std::path::Path;

use anyhow::Result;

use crate::taxonomy::CodemapTaxonomy;

pub fn print_taxonomy(root: &Path) -> Result<()> {
    let taxonomy = CodemapTaxonomy::load(root)?;
    let name = taxonomy
        .metadata
        .as_ref()
        .map(|metadata| metadata.name.as_str())
        .unwrap_or("codemap taxonomy");

    println!("taxonomy: {name}");
    println!("version: {}", taxonomy.version);
    println!();

    println!("priorities:");
    for (name, priority) in &taxonomy.priorities {
        println!("  {name}: {} score={}", priority.label, priority.score);
    }

    println!();
    println!("layers:");
    for (name, layer) in &taxonomy.layers {
        println!("  {name}: {}", layer.label);
    }

    println!();
    println!("domains:");
    for (name, domain) in &taxonomy.domains {
        println!("  {name}: layer={} {}", domain.layer, domain.label);
    }

    println!();
    println!("roles:");
    for (name, role) in &taxonomy.roles {
        println!("  {name}: score={} {}", role.score, role.description);
    }

    Ok(())
}
