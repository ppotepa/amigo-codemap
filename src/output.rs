use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use serde_json::{Value, json};

use crate::cli::Options;
use crate::model::CodeMap;

pub fn write_codemap(options: &Options, map: &CodeMap) -> Result<bool> {
    let value = to_json(map);
    let previous = fs::read(&options.out)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
    if previous
        .as_ref()
        .is_some_and(|previous| comparable(previous) == comparable(&value))
    {
        return Ok(false);
    }

    let bytes = if options.pretty {
        serde_json::to_vec_pretty(&value)?
    } else {
        serde_json::to_vec(&value)?
    };

    if let Some(parent) = options.out.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&options.out, bytes)?;
    append_delta(&options.out, previous.as_ref(), &value)?;
    Ok(true)
}

pub fn to_json(map: &CodeMap) -> Value {
    json!({
        "v": 1,
        "ts": unix_timestamp(),
        "root": map.root_name,
        "git": {
            "b": map.git.branch,
            "r": map.git.rev,
            "dirty": if map.git.dirty { 1 } else { 0 },
            "c": map.git.changed.iter().map(|change| {
                let target = change.file_id.as_deref().unwrap_or_else(|| change.path.to_str().unwrap_or(""));
                json!([change.status, target])
            }).collect::<Vec<_>>()
        },
        "st": map.stats,
        "f": map.files.iter().map(|file| {
            json!([file.id, slash_path(&file.path), file.language, file.lines, file.hash])
        }).collect::<Vec<_>>(),
        "pkg": map.packages.iter().map(|package| {
            json!([
                package.id,
                package.kind,
                package.name,
                slash_path(&package.manifest_path),
                package.version,
                package.dependencies,
                package.scripts,
            ])
        }).collect::<Vec<_>>(),
        "s": map.symbols.iter().map(|symbol| {
            json!([symbol.name, symbol.kind, symbol.file_id, symbol.line, symbol.visibility])
        }).collect::<Vec<_>>(),
        "d": map.dependencies.iter().map(|dep| {
            json!([dep.from, dep.to, dep.kind])
        }).collect::<Vec<_>>(),
        "areas": map.areas.iter().map(|area| {
            json!([area.name, area.files])
        }).collect::<Vec<_>>()
    })
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn comparable(value: &Value) -> Value {
    let mut value = value.clone();
    if let Value::Object(object) = &mut value {
        object.remove("ts");
    }
    value
}

fn append_delta(out: &Path, previous: Option<&Value>, current: &Value) -> Result<()> {
    let Some(parent) = out.parent() else {
        return Ok(());
    };
    let delta_path = parent.join("codemap.delta.jsonl");
    let previous_files = previous.map(files_by_path).unwrap_or_default();
    let current_files = files_by_path(current);
    let timestamp = current
        .get("ts")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    let mut lines = Vec::new();

    for (path, file) in &current_files {
        match previous_files.get(path) {
            Some(previous) if previous.hash != file.hash => {
                lines.push(
                    json!({"t": timestamp, "op": "mod", "f": file.id, "p": path, "h": file.hash}),
                );
            }
            None => {
                lines.push(
                    json!({"t": timestamp, "op": "add", "f": file.id, "p": path, "h": file.hash}),
                );
            }
            _ => {}
        }
    }

    for (path, file) in &previous_files {
        if !current_files.contains_key(path) {
            lines.push(json!({"t": timestamp, "op": "del", "f": file.id, "p": path}));
        }
    }

    if lines.is_empty() {
        lines.push(json!({"t": timestamp, "op": "scan"}));
    }

    let mut text = String::new();
    for line in lines {
        text.push_str(&serde_json::to_string(&line)?);
        text.push('\n');
    }
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(delta_path)?
        .write_all(text.as_bytes())?;
    Ok(())
}

#[derive(Debug, Clone)]
struct DeltaFile {
    id: String,
    hash: String,
}

fn files_by_path(value: &Value) -> BTreeMap<String, DeltaFile> {
    let mut files = BTreeMap::new();
    let Some(items) = value.get("f").and_then(Value::as_array) else {
        return files;
    };
    for item in items {
        let Some(array) = item.as_array() else {
            continue;
        };
        let (Some(id), Some(path), Some(hash)) = (
            array.first().and_then(Value::as_str),
            array.get(1).and_then(Value::as_str),
            array.get(4).and_then(Value::as_str),
        ) else {
            continue;
        };
        files.insert(
            path.to_string(),
            DeltaFile {
                id: id.to_string(),
                hash: hash.to_string(),
            },
        );
    }
    files
}
