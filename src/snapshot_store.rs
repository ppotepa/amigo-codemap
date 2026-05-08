use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::cli::Options;
use crate::model::CodeMap;
use crate::{output, scan};

const SNAPSHOT_SCHEMA_VERSION: u16 = 1;
const SNAPSHOT_FILE_NAME: &str = "codemap.snapshot.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotEnvelope {
    pub version: u16,
    pub root: String,
    pub level: u8,
    pub generated_at_unix_ms: u128,
    pub map: CodeMap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapSource {
    Snapshot,
    Scan,
}

#[derive(Debug, Clone)]
pub struct LoadedMap {
    pub map: CodeMap,
    pub source: MapSource,
}

pub fn snapshot_path(root: &Path) -> PathBuf {
    root.join(".amigo").join(SNAPSHOT_FILE_NAME)
}

pub fn write_snapshot(options: &Options, map: &CodeMap) -> Result<()> {
    let path = snapshot_path(&options.root);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let envelope = SnapshotEnvelope {
        version: SNAPSHOT_SCHEMA_VERSION,
        root: normalize_path(&options.root),
        level: options.level,
        generated_at_unix_ms: now_ms(),
        map: map.clone(),
    };

    fs::write(path, serde_json::to_vec(&envelope)?)?;
    Ok(())
}

pub fn read_snapshot(options: &Options) -> Result<Option<SnapshotEnvelope>> {
    let path = snapshot_path(&options.root);

    if !path.exists() {
        return Ok(None);
    }

    let bytes = fs::read(path)?;
    Ok(Some(serde_json::from_slice::<SnapshotEnvelope>(&bytes)?))
}

pub fn load_or_scan(options: &Options) -> Result<LoadedMap> {
    if !options.no_cache {
        if let Some(envelope) = read_snapshot(options)? {
            if snapshot_is_usable(options, &envelope) {
                if snapshot_may_be_stale(options) {
                    eprintln!("codemap snapshot is stale; refreshing before running report");
                } else {
                    return Ok(LoadedMap {
                        map: envelope.map,
                        source: MapSource::Snapshot,
                    });
                }
            }
        }
    }

    let map = scan::scan_project(options)?;
    output::write_codemap(options, &map)?;
    write_snapshot(options, &map)?;

    Ok(LoadedMap {
        map,
        source: MapSource::Scan,
    })
}

pub fn refresh_snapshot(options: &Options) -> Result<bool> {
    let map = scan::scan_project(options)?;
    write_snapshot(options, &map)?;
    output::write_codemap(options, &map)
}

pub fn print_status(options: &Options) -> Result<()> {
    let path = snapshot_path(&options.root);

    println!("codemap status");
    println!("  root: {}", options.root.display());
    println!("  compact output: {}", options.out.display());
    println!("  snapshot cache: {}", path.display());

    if !path.exists() {
        println!("  snapshot: missing");
        println!("  next: amigo-codemap refresh");
        return Ok(());
    }

    let Some(envelope) = read_snapshot(options)? else {
        println!("  snapshot: unreadable");
        return Ok(());
    };

    println!("  snapshot: ok");
    println!("  schema: {}", envelope.version);
    println!("  level: {}", envelope.level);
    println!("  generated_at_unix_ms: {}", envelope.generated_at_unix_ms);
    println!("  files: {}", envelope.map.files.len());
    println!("  packages: {}", envelope.map.packages.len());
    println!("  symbols: {}", envelope.map.symbols.len());
    println!("  dependencies: {}", envelope.map.dependencies.len());
    println!("  areas: {}", envelope.map.areas.len());
    println!("  git_dirty: {}", envelope.map.git.dirty);

    if snapshot_is_usable(options, &envelope) {
        println!("  source: usable");
    } else {
        println!("  source: not usable for current options");
        println!("  next: amigo-codemap refresh");
    }

    if snapshot_may_be_stale(options) {
        println!("  stale: yes");
        println!("  next: amigo-codemap refresh or amigo-codemap watch --write");
    } else {
        println!("  stale: no");
    }

    Ok(())
}

pub fn snapshot_is_usable(options: &Options, envelope: &SnapshotEnvelope) -> bool {
    envelope.version == SNAPSHOT_SCHEMA_VERSION
        && envelope.level >= options.level
        && envelope.root == normalize_path(&options.root)
}

pub fn snapshot_may_be_stale(options: &Options) -> bool {
    let snapshot = snapshot_path(&options.root);

    let Ok(snapshot_meta) = fs::metadata(&snapshot) else {
        return true;
    };

    let Ok(snapshot_modified) = snapshot_meta.modified() else {
        return true;
    };

    let git_index = options.root.join(".git").join("index");
    if let Ok(git_meta) = fs::metadata(git_index) {
        if let Ok(git_modified) = git_meta.modified() {
            if git_modified > snapshot_modified {
                return true;
            }
        }
    }

    source_tree_modified_after(&options.root, snapshot_modified).unwrap_or(true)
}

fn source_tree_modified_after(root: &Path, snapshot_modified: SystemTime) -> Result<bool> {
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let read_dir = match fs::read_dir(&dir) {
            Ok(read_dir) => read_dir,
            Err(error) if is_permission_denied(&error) => continue,
            Err(error) => return Err(error.into()),
        };
        for entry in read_dir {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) if is_permission_denied(&error) => continue,
                Err(error) => return Err(error.into()),
            };
            let path = entry.path();
            if should_skip_snapshot_stale_path(root, &path) {
                continue;
            }
            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(error) if is_permission_denied(&error) => continue,
                Err(error) => return Err(error.into()),
            };
            if metadata.is_dir() {
                stack.push(path);
                continue;
            }
            if is_codemap_source_file(&path)
                && metadata
                    .modified()
                    .is_ok_and(|modified| modified > snapshot_modified)
            {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

fn is_permission_denied(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::PermissionDenied
}

fn should_skip_snapshot_stale_path(root: &Path, path: &Path) -> bool {
    let relative = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    relative.starts_with(".git/")
        || relative == ".git"
        || relative.starts_with(".amigo/")
        || relative == ".amigo"
        || relative.starts_with("target/")
        || relative == "target"
        || relative.contains("/node_modules/")
        || relative.ends_with("/node_modules")
        || relative.starts_with("node_modules/")
        || relative.starts_with("dist/")
        || relative.starts_with("build/")
        || relative.starts_with("coverage/")
}

fn is_codemap_source_file(path: &Path) -> bool {
    path.extension().is_some_and(|ext| {
        matches!(
            ext.to_string_lossy().to_ascii_lowercase().as_str(),
            "rs" | "ts" | "tsx" | "css" | "md" | "toml" | "yaml" | "yml" | "rhai"
        )
    })
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}
