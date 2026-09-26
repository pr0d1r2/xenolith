//! One writer at a time: the repo lock `--write` holds
//! (`src/extract:V127`, `src/extract:T126`).
//!
//! An OS advisory lock on `.xenolith.lock` at the repo root, not the
//! file's existence: the lock goes with its holder, so a writer that
//! died leaves a file nobody holds, which the next writer simply takes
//! -- a stale lock reclaimed without guessing whether a pid is alive.
//! The file's text names the holder (`pid <n> start <unix seconds>`),
//! so a refused second writer can say who is in the way, and the file
//! is removed on release while still held.

use std::fs::{self, File, TryLockError};
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::write::guard;

#[cfg(test)]
mod tests;

/// The lock file, at the repo root.
pub const FILE: &str = ".xenolith.lock";

/// A held lock; released, and its file removed, when dropped.
#[derive(Debug)]
pub struct Lock {
    file: File,
    path: PathBuf,
}

impl Lock {
    /// Take the lock at `root`, or say who holds it.
    ///
    /// # Errors
    ///
    /// Another writer holds it (named by pid and start time), the lock
    /// file is a symlink (`src/extract:V71`), or it cannot be opened.
    pub fn take(root: &Path) -> Result<Lock, String> {
        guard(root, FILE)?;
        let path = root.join(FILE);
        // A lock on a file another writer just removed would guard
        // nothing: take it again on the file now at the path.
        for _ in 0..3 {
            let mut file = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(&path)
                .map_err(|e| format!("{FILE}: {e}"))?;
            match file.try_lock() {
                Ok(()) => {}
                Err(TryLockError::WouldBlock) => {
                    let mut holder = String::new();
                    let _ = file.read_to_string(&mut holder);
                    return Err(format!(
                        "another `xnl extract --write` holds {FILE} ({}); one writer at a \
                         time (src/extract:V127)",
                        holder.trim()
                    ));
                }
                Err(TryLockError::Error(e)) => return Err(format!("{FILE}: {e}")),
            }
            if !same_file(&file, &path) {
                continue;
            }
            let started = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_secs());
            let text = format!("pid {} start {started}\n", std::process::id());
            file.set_len(0)
                .and_then(|()| file.seek(SeekFrom::Start(0)))
                .and_then(|_| file.write_all(text.as_bytes()))
                .and_then(|()| file.sync_all())
                .map_err(|e| format!("{FILE}: {e}"))?;
            return Ok(Lock { file, path });
        }
        Err(format!(
            "{FILE} kept changing under this run; another writer is busy (src/extract:V127)"
        ))
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        // Removed while still held, so no writer can take the old file
        // in the gap; the lock itself goes when `file` closes.
        let _ = fs::remove_file(&self.path);
        let _ = self.file.unlock();
    }
}

/// Whether `file` is still the file at `path`.
#[cfg(unix)]
fn same_file(file: &File, path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (file.metadata(), fs::symlink_metadata(path)) {
        (Ok(held), Ok(named)) => held.dev() == named.dev() && held.ino() == named.ino(),
        _ => false,
    }
}

/// Off unix there is no inode to compare; the file at the path is
/// taken as the one opened.
#[cfg(not(unix))]
fn same_file(_file: &File, path: &Path) -> bool {
    path.is_file()
}
