//! `xnl extract --write`: an [`Edit`] carried out (`src/extract:C15`).
//!
//! Per host, its extracts first and the host last, so a run that stops
//! half way leaves an extract nothing loads yet -- an orphan `xnl graph`
//! reports -- rather than a load of a file that is not there
//! (`src/extract:V84`). Each file is written whole or not at all: into
//! a temp file beside it, synced, then renamed over it, so a reader
//! never sees half an extract or half a host.

use std::fs;
use std::io::{self, Write as _};
use std::path::Path;

use super::Edit;
use super::lock::Lock;

#[cfg(test)]
mod tests;

/// Write every change in `edit` under `root`; returns the paths
/// written, in order.
///
/// # Errors
///
/// The first write that failed or was refused, naming its path.
pub fn apply(root: &Path, edit: &Edit) -> Result<Vec<String>, String> {
    apply_with(root, edit, &mut |_| Ok(()))
}

/// [`apply`], with `before` called on each path right before it is
/// written -- the seam a test kills a run through (`src/extract:T85`).
pub(crate) fn apply_with(
    root: &Path,
    edit: &Edit,
    before: &mut dyn FnMut(&str) -> io::Result<()>,
) -> Result<Vec<String>, String> {
    if edit.hosts.is_empty() {
        return Ok(Vec::new());
    }
    // One writer (`src/extract:V127`), and the plan still true under the
    // lock: a host another hand changed since would lose that change.
    let _lock = Lock::take(root)?;
    let changed: Vec<&str> = edit
        .hosts
        .iter()
        .filter(|host| {
            fs::read(root.join(&host.path)).ok().as_deref() != Some(host.before.as_bytes())
        })
        .map(|host| host.path.as_str())
        .collect();
    if !changed.is_empty() {
        return Err(format!(
            "{} changed since the plan was made; nothing was written, run xnl extract again",
            changed.join(", ")
        ));
    }
    let mut written = Vec::new();
    for host in &edit.hosts {
        for extract in host.extracts.iter().filter(|e| !e.present) {
            let fail = |e: io::Error| format!("{}: {e}", extract.path);
            guard(root, &extract.path)?;
            before(&extract.path).map_err(fail)?;
            let path = root.join(&extract.path);
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir).map_err(fail)?;
            }
            atomic(&path, &extract.text, mode(extract.executable)).map_err(fail)?;
            written.push(extract.path.clone());
        }
        let fail = |e: io::Error| format!("{}: {e}", host.path);
        guard(root, &host.path)?;
        before(&host.path).map_err(fail)?;
        let path = root.join(&host.path);
        // The host keeps its own mode: a rename puts the temp file's in
        // its place otherwise.
        let kept = fs::metadata(&path).map(|m| m.permissions()).ok();
        atomic(&path, &host.after, kept).map_err(fail)?;
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

/// `text` into `path` atomically (`src/extract:V84`): a temp file in
/// the same directory -- a rename across filesystems is a copy -- then
/// fsync, then rename, then fsync of the directory so the rename itself
/// survives a crash. The temp file is removed when any step fails.
fn atomic(path: &Path, text: &str, perms: Option<fs::Permissions>) -> io::Result<()> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let temp = dir.join(format!(".{name}.xnl-{}.tmp", std::process::id()));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        file.write_all(text.as_bytes())?;
        if let Some(perms) = perms {
            file.set_permissions(perms)?;
        }
        file.sync_all()?;
        fs::rename(&temp, path)?;
        sync_dir(dir)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

/// `0755` for an executable extract, `0644` otherwise.
#[cfg(unix)]
#[allow(clippy::unnecessary_wraps)] // the twin off unix has no mode to give
fn mode(executable: bool) -> Option<fs::Permissions> {
    use std::os::unix::fs::PermissionsExt;
    Some(fs::Permissions::from_mode(if executable {
        0o755
    } else {
        0o644
    }))
}

/// No mode bits off unix.
#[cfg(not(unix))]
fn mode(_executable: bool) -> Option<fs::Permissions> {
    None
}

/// Make a rename in `dir` durable.
#[cfg(unix)]
fn sync_dir(dir: &Path) -> io::Result<()> {
    fs::File::open(dir)?.sync_all()
}

/// Directories cannot be opened for syncing off unix.
#[cfg(not(unix))]
#[allow(clippy::unnecessary_wraps)] // the unix twin can fail
fn sync_dir(_dir: &Path) -> io::Result<()> {
    Ok(())
}
