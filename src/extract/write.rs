//! `xnl extract --write`: an [`Edit`] carried out (`src/extract:C15`).
//!
//! Per host, its extracts first and the host last, so a run that stops
//! half way leaves an extract nothing loads yet -- an orphan `xnl graph`
//! reports -- rather than a load of a file that is not there
//! (`src/extract:V84`).

use std::fs;
use std::io;
use std::path::Path;

use super::{Edit, NewFile};

#[cfg(test)]
mod tests;

/// Write every change in `edit` under `root`; returns the paths
/// written, in order.
///
/// # Errors
///
/// The first write that failed or was refused, naming its path.
pub fn apply(root: &Path, edit: &Edit) -> Result<Vec<String>, String> {
    let mut written = Vec::new();
    for host in &edit.hosts {
        for extract in host.extracts.iter().filter(|e| !e.present) {
            guard(root, &extract.path)?;
            create(root, extract).map_err(|e| format!("{}: {e}", extract.path))?;
            written.push(extract.path.clone());
        }
        guard(root, &host.path)?;
        fs::write(root.join(&host.path), &host.after).map_err(|e| format!("{}: {e}", host.path))?;
        written.push(host.path.clone());
    }
    Ok(written)
}

/// Refuse to write `rel` when any part of it that exists is a symlink,
/// or when its deepest existing ancestor resolves outside `root`
/// (`src/extract:V71`): a write through a link lands wherever the link
/// points, and a directory created through one is created there too.
///
/// # Errors
///
/// The component that is a link, or the path that leaves the root.
pub fn guard(root: &Path, rel: &str) -> Result<(), String> {
    let refuse = |why: String| Err(format!("{rel}: {why} (src/extract:V71)"));
    let mut at = root.to_path_buf();
    let mut deepest = root.to_path_buf();
    for part in rel.split('/').filter(|p| !p.is_empty()) {
        at.push(part);
        match fs::symlink_metadata(&at) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return refuse(format!(
                    "{} is a symlink, and extract never writes through one",
                    at.strip_prefix(root).unwrap_or(&at).display()
                ));
            }
            Ok(_) => deepest.clone_from(&at),
            Err(_) => break,
        }
    }
    let inside = match (fs::canonicalize(root), fs::canonicalize(&deepest)) {
        (Ok(root), Ok(deepest)) => deepest.starts_with(root),
        _ => false,
    };
    if inside {
        Ok(())
    } else {
        refuse("it resolves outside the repository".to_owned())
    }
}

/// Create one extract, with its directory and mode.
fn create(root: &Path, file: &NewFile) -> io::Result<()> {
    let path = root.join(&file.path);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(&path, &file.text)?;
    set_mode(&path, file.executable)
}

/// `0755` for an executable extract, `0644` otherwise.
#[cfg(unix)]
fn set_mode(path: &Path, executable: bool) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mode = if executable { 0o755 } else { 0o644 };
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
}

/// No mode bits to set off unix.
#[cfg(not(unix))]
#[allow(clippy::unnecessary_wraps)] // the unix twin can fail
fn set_mode(_path: &Path, _executable: bool) -> io::Result<()> {
    Ok(())
}
