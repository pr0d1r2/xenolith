//! Symlinks in discovery (`src:V128`): a candidate that is a link, or
//! that is reached through one, is never scanned.
//!
//! Two reasons, both about what a report means. A tracked link may point
//! outside the repository, and its bytes are then someone else's code
//! reported as this repository's. And a link inside the repository is the
//! same bytes under a second name, reported twice. The target, when it is
//! in the repository, is scanned under its own path already.
//!
//! How a skipped link is SAID depends on who chose it:
//!
//! * FOUND by discovery (`git ls-files`, a walked directory): skipped
//!   with a `symlink-skipped` warning. The user did not ask for that file
//!   by name, and a warning never changes the exit code.
//! * NAMED by the caller: exit 2. Skipping the one file somebody asked
//!   about and exiting 0 would read, in a gate, as that file being clean.
//!
//! Listing never follows a link in the first place: git records a linked
//! directory as one entry, and the walk does not enter one. This module
//! screens what the listing produced; `src/extract:V71` and
//! `src/graph:V72` are the write and graph halves of the same rule.

use std::fs;
use std::path::{Component, Path, PathBuf};

use super::{Candidates, DiscoverError, SYMLINK_SKIPPED};
use crate::model::Warning;

#[cfg(test)]
mod tests;

/// Split `listed` into the files to scan and a warning per skipped
/// link, refusing when a path the caller `named` is, or runs through, a
/// symlink.
pub(super) fn screen(
    root: &Path,
    named: &[PathBuf],
    listed: Vec<PathBuf>,
) -> Result<Candidates, DiscoverError> {
    let mut candidates = Candidates::default();
    for path in listed {
        let Some(link) = symlinked_component(root, &path) else {
            candidates.files.push(path);
            continue;
        };
        if named.contains(&path) {
            return Err(DiscoverError::Symlink { path, link });
        }
        let message = if link == path {
            format!(
                "{} is a symlink; not scanned, its target is scanned under its own path \
                 (src:V128)",
                path.display()
            )
        } else {
            format!(
                "{} lies under the symlink `{}`; not scanned (src:V128)",
                path.display(),
                link.display()
            )
        };
        candidates.warnings.push(Warning {
            code: SYMLINK_SKIPPED.to_owned(),
            file: Some(path),
            message,
        });
    }
    Ok(candidates)
}

/// The first prefix of `path` that is a symlink on disk, if any.
///
/// Only the components below `root` are judged: where the repository
/// itself lives (macOS keeps the temp dir under `/var` -> `/private/var`)
/// is not the repository's business. An absolute path outside `root` is
/// judged by its last component alone, for the same reason. A prefix
/// that does not exist ends the search: nothing below it can be a link.
pub(super) fn symlinked_component(root: &Path, path: &Path) -> Option<PathBuf> {
    let (base, below) = if !path.is_absolute() {
        (PathBuf::new(), path)
    } else if let Ok(rel) = path.strip_prefix(root) {
        (root.to_path_buf(), rel)
    } else {
        let is_link = fs::symlink_metadata(path).is_ok_and(|m| m.is_symlink());
        return is_link.then(|| path.to_path_buf());
    };
    let mut prefix = base;
    for component in below.components() {
        if matches!(component, Component::CurDir) {
            continue;
        }
        prefix.push(component);
        match fs::symlink_metadata(root.join(&prefix)) {
            Ok(meta) if meta.is_symlink() => return Some(prefix),
            Ok(_) => {}
            Err(_) => return None,
        }
    }
    None
}
