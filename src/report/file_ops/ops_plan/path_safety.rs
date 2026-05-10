use std::path::{Component, Path, PathBuf};

use anyhow::{Result, bail};

pub(super) fn repo_path(root: &Path, path: &Path) -> Result<PathBuf> {
    validate_repo_relative_path(path)?;
    Ok(root.join(path))
}

pub(super) fn validate_repo_relative_path(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() {
        bail!("path must not be empty");
    }
    if path.is_absolute() {
        bail!("path must be repo-relative: {}", path.display());
    }
    for component in path.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                bail!("path must not escape repo root: {}", path.display());
            }
        }
    }
    Ok(())
}

pub(super) fn path_key(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
