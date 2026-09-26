//! Load resolution: the file a host's load names, found without
//! following a symlink (`src/graph:V72`, `src/graph` §I).
//!
//! A load is relative to its site's runtime base, by default the host
//! file's directory (`languages/api/src/lens:V66`). The path is folded
//! lexically -- `..` allowed while it stays inside the root -- and then
//! walked from the root one component at a time with `symlink_metadata`,
//! so a symlink anywhere on the way is seen rather than followed: it may
//! point outside the repository, and the same bytes would then be judged
//! under two names (`src:V128`).

use std::fmt;
use std::fs;
use std::path::{Component, Path};

#[cfg(test)]
mod tests;

/// Why a load does not resolve: each a `dangling-load` (`src/graph:V7`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dangling {
    /// Nothing at this repo path.
    Missing(String),
    /// Absolute, or climbs above the root.
    Outside,
    /// This component on the way is a symlink.
    Symlink(String),
    /// There, but not a regular file.
    NotAFile(String),
}

impl fmt::Display for Dangling {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Dangling::Missing(path) => write!(f, "{path} does not exist"),
            Dangling::Outside => f.write_str("it points outside the repository root"),
            Dangling::Symlink(path) => write!(
                f,
                "{path} is a symlink, and graph never follows one (src/graph:V72)"
            ),
            Dangling::NotAFile(path) => write!(f, "{path} is not a regular file"),
        }
    }
}

/// The repo path the load `load` in host file `host` names, when it is
/// a regular file reached through no symlink.
///
/// # Errors
///
/// [`Dangling`], naming why the load does not resolve.
pub fn resolve(root: &Path, host: &str, load: &Path) -> Result<String, Dangling> {
    let mut parts: Vec<String> = Path::new(host)
        .parent()
        .into_iter()
        .flat_map(Path::components)
        .filter_map(|c| match c {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    for component in load.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop().ok_or(Dangling::Outside)?;
            }
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::RootDir | Component::Prefix(_) => return Err(Dangling::Outside),
        }
    }
    let mut at = root.to_path_buf();
    for (index, part) in parts.iter().enumerate() {
        at.push(part);
        let name = parts
            .iter()
            .take(index + 1)
            .cloned()
            .collect::<Vec<_>>()
            .join("/");
        let meta = fs::symlink_metadata(&at).map_err(|_| Dangling::Missing(parts.join("/")))?;
        if meta.file_type().is_symlink() {
            return Err(Dangling::Symlink(name));
        }
        if index + 1 == parts.len() && !meta.is_file() {
            return Err(Dangling::NotAFile(name));
        }
    }
    if parts.is_empty() {
        return Err(Dangling::NotAFile(".".to_owned()));
    }
    Ok(parts.join("/"))
}
