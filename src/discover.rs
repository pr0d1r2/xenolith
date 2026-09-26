//! Candidate discovery: which files a verb looks at (`src:V57`).
//!
//! The first stage of the check engine (`src:V152`): everything after it
//! -- hosts claiming files, sites, guests -- sees only what this module
//! returns, so a file dropped here is a file never judged, and a file
//! added here is a file judged that nobody asked about.
//!
//! Two sources, never mixed:
//!
//! * NO PATHS: `git ls-files`, tracked only, so `.gitignore` is honoured
//!   without re-implementing it and a stray build output is never
//!   scanned. Outside a repository that question has no answer, and the
//!   answer is a refusal (exit 2), not an empty scan that a gate would
//!   read as clean.
//! * EXPLICIT PATHS: what hk (or a person) named. A file is taken as
//!   given, tracked or not -- hk already decided. A directory means its
//!   tracked files inside a repository, and a walk outside one; one that
//!   yields no file at all is refused, since scanning nothing and exiting
//!   0 would read, in a gate, as that directory clean.
//!
//! Whichever source, a symlink is never scanned (`src:V128`): found, it
//! is skipped with a warning; named, it is refused. That rule and its
//! reasons are `symlink`'s.
//!
//! git runs in the user's repository and INHERITS the environment on
//! purpose: inside a hook, git exports `GIT_DIR` and `GIT_INDEX_FILE`,
//! and the listing should be the hook's view of the repository. Only the
//! tests isolate git (`tests:V150`), through `discover_with`.

use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use crate::cli::EXIT_USAGE;
use crate::model::Warning;

mod symlink;

#[cfg(test)]
mod tests;

/// The git sandbox every test that builds a repository runs git through
/// (`tests:V150`, `tests:B1`), shared rather than copied.
#[cfg(test)]
pub(crate) use self::tests::{Sandbox, write};

/// The warning code for a candidate skipped because it is, or lies under,
/// a symlink (`src:V128`). Stable, matched like a rule id.
pub const SYMLINK_SKIPPED: &str = "symlink-skipped";

/// What discovery hands the engine: the files to scan, repo-root
/// relative and sorted (`src:V11`), and what it has to say about the
/// ones it left out.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Candidates {
    /// The files to scan, sorted, each once.
    pub files: Vec<PathBuf>,
    /// Files seen and deliberately not scanned. Never an exit code
    /// (`src/cli` §I).
    pub warnings: Vec<Warning>,
}

/// Why discovery refused. Every variant is exit 2 (`src/cli:V24`): the
/// request could not be carried out, which is neither "clean" nor
/// "found something".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoverError {
    /// No paths were named and `root` is not inside a git work tree.
    NotARepo {
        /// The directory discovery ran from.
        root: PathBuf,
    },
    /// git could not be run, or failed for a reason other than "not a
    /// repository".
    Git {
        /// What git (or the attempt to spawn it) said.
        detail: String,
    },
    /// A path named explicitly does not exist.
    Missing {
        /// The path as it was named.
        path: PathBuf,
    },
    /// A directory named explicitly holds no file to scan: inside a
    /// repository, no tracked one.
    EmptyDir {
        /// The directory as it was named.
        path: PathBuf,
    },
    /// A path named explicitly is neither a directory nor a regular file
    /// (nor a symlink, refused as [`DiscoverError::Symlink`]): a FIFO, a
    /// socket, a device. It has no source to judge, and opening a FIFO
    /// blocks until something writes to it.
    NotAFile {
        /// The path as it was named.
        path: PathBuf,
    },
    /// A path could not be read while listing it.
    Io {
        /// The path being read.
        path: PathBuf,
        /// The operating system's reason.
        detail: String,
    },
    /// A path named explicitly is, or runs through, a symlink
    /// (`src:V128`).
    Symlink {
        /// The path as it was named.
        path: PathBuf,
        /// The part of it that is the link: the path itself, or a
        /// directory it runs through.
        link: PathBuf,
    },
}

impl DiscoverError {
    /// The process exit code this refusal maps to: always 2.
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        EXIT_USAGE
    }
}

impl fmt::Display for DiscoverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiscoverError::NotARepo { root } => write!(
                f,
                "{} is not a git repository and no paths were given: name the files \
                 to check, or run inside a repository (src:V57)",
                root.display()
            ),
            DiscoverError::Git { detail } => {
                write!(f, "git ls-files failed: {detail} (src:V57)")
            }
            DiscoverError::Missing { path } => {
                write!(f, "{}: no such file or directory (src:V57)", path.display())
            }
            DiscoverError::EmptyDir { path } => write!(
                f,
                "{}: named directory holds no file to scan; inside a repository only \
                 its tracked files are candidates, untracked & ignored ones are not \
                 (src:V57)",
                path.display()
            ),
            DiscoverError::NotAFile { path } => write!(
                f,
                "{}: named explicitly but is not a regular file (a FIFO, socket or \
                 device); it has no source to scan (src:V57)",
                path.display()
            ),
            DiscoverError::Io { path, detail } => {
                write!(f, "{}: {detail} (src:V57)", path.display())
            }
            DiscoverError::Symlink { path, link } if path == link => write!(
                f,
                "{}: named explicitly but is a symlink; xenolith does not read \
                 through symlinks, name its target instead (src:V128)",
                path.display()
            ),
            DiscoverError::Symlink { path, link } => write!(
                f,
                "{}: named explicitly but runs through the symlink `{}`; xenolith \
                 does not read through symlinks, name the real path instead (src:V128)",
                path.display(),
                link.display()
            ),
        }
    }
}

impl std::error::Error for DiscoverError {}

/// The candidates for a verb run from `root` over `paths` (`src:V57`).
///
/// `paths` empty means "the repository": its tracked files. Otherwise
/// each path is taken relative to `root`, as named. The engine calls
/// this once per run.
///
/// # Errors
///
/// [`DiscoverError`], every variant exit 2: outside git with no paths,
/// git failing, or a named path that does not exist, cannot be read, or
/// is a symlink (`src:V128`) or not a regular file, or a named directory
/// with no file to scan.
/// A symlink FOUND rather than named is
/// skipped with a [`SYMLINK_SKIPPED`] warning instead.
pub fn discover(root: &Path, paths: &[PathBuf]) -> Result<Candidates, DiscoverError> {
    discover_with(root, paths, &|| Command::new("git"))
}

/// [`discover`], with the `git` command supplied by the caller -- the
/// seam the tests use to run git sandboxed (`tests:V150`), here and
/// through the check engine (`src:V152`).
pub(crate) fn discover_with(
    root: &Path,
    paths: &[PathBuf],
    git: &dyn Fn() -> Command,
) -> Result<Candidates, DiscoverError> {
    let named: Vec<PathBuf> = paths.iter().map(|p| normalise(root, p)).collect();
    symlink::screen(root, &named, list(root, paths, git)?)
}

/// `path` as the one name its file is reported under: relative to
/// `root` when it lies below it, `.` components and doubled separators
/// gone, and `x/..` folded away when `x` is a real directory. Named as
/// `./a.nix`, `a.nix` and by its absolute path, a file would otherwise
/// be scanned, and reported, once per spelling.
///
/// `..` is folded only past a directory, never past a symlink: through
/// a link, `link/..` is the link target's parent, and the unfolded path
/// is what `symlink::screen` refuses (`src:V128`). A path outside `root`
/// keeps its absolute form.
fn normalise(root: &Path, path: &Path) -> PathBuf {
    let below = match path.strip_prefix(root) {
        Ok(rel) if path.is_absolute() => rel,
        _ if path.is_absolute() => return path.components().collect(),
        _ => path,
    };
    let mut out = PathBuf::new();
    for component in below.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir
                if matches!(out.components().next_back(), Some(Component::Normal(_)))
                    && fs::symlink_metadata(root.join(&out)).is_ok_and(|m| m.is_dir()) =>
            {
                out.pop();
            }
            other => out.push(other),
        }
    }
    if out.as_os_str().is_empty() {
        out.push(Component::CurDir);
    }
    out
}

/// Every path discovery would scan, sorted and deduplicated.
fn list(
    root: &Path,
    paths: &[PathBuf],
    git: &dyn Fn() -> Command,
) -> Result<Vec<PathBuf>, DiscoverError> {
    let mut files = BTreeSet::new();
    if paths.is_empty() {
        files.extend(ls_files(root, None, git)?);
    }
    for named in paths {
        let path = &normalise(root, named);
        let meta = fs::symlink_metadata(root.join(path)).map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => DiscoverError::Missing {
                path: named.clone(),
            },
            _ => DiscoverError::Io {
                path: named.clone(),
                detail: e.to_string(),
            },
        })?;
        if meta.is_dir() {
            let mut found = BTreeSet::new();
            match ls_files(root, Some(path), git) {
                Ok(listed) => found.extend(listed),
                Err(DiscoverError::NotARepo { .. }) => walk(root, path, &mut found)?,
                Err(e) => return Err(e),
            }
            if found.is_empty() {
                return Err(DiscoverError::EmptyDir {
                    path: named.clone(),
                });
            }
            files.append(&mut found);
        } else if scannable(meta.file_type()) {
            files.insert(path.clone());
        } else {
            return Err(DiscoverError::NotAFile {
                path: named.clone(),
            });
        }
    }
    Ok(files.into_iter().collect())
}

/// `git ls-files` from `root`, optionally limited to one directory.
///
/// `-z` because a filename is bytes and git otherwise quotes the unusual
/// ones; `--literal-pathspecs` because a directory named `d*` means that
/// directory and not every one starting with `d`. `LC_ALL=C` so "not a
/// git repository" can be told apart from every other failure in any
/// locale. Entries the work tree no longer has, that are directories (a
/// submodule), or that are no longer regular files (a FIFO where a file
/// was) have no bytes to scan and are dropped.
fn ls_files(
    root: &Path,
    within: Option<&Path>,
    git: &dyn Fn() -> Command,
) -> Result<Vec<PathBuf>, DiscoverError> {
    let mut cmd = git();
    cmd.arg("--literal-pathspecs")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-z", "--"])
        .env("LC_ALL", "C");
    if let Some(dir) = within {
        cmd.arg(dir);
    }
    let out = cmd.output().map_err(|e| DiscoverError::Git {
        detail: format!("could not run git: {e}"),
    })?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_owned();
        if stderr.contains("not a git repository") {
            return Err(DiscoverError::NotARepo {
                root: root.to_path_buf(),
            });
        }
        return Err(DiscoverError::Git {
            detail: if stderr.is_empty() {
                format!("exited with {}", out.status)
            } else {
                stderr
            },
        });
    }
    let mut files = Vec::new();
    for entry in out.stdout.split(|&b| b == 0).filter(|e| !e.is_empty()) {
        let path = path_from_bytes(entry)?;
        match fs::symlink_metadata(root.join(&path)) {
            Ok(meta) if scannable(meta.file_type()) => files.push(path),
            _ => {}
        }
    }
    Ok(files)
}

/// A path from git's raw bytes, exactly: on unix a path IS bytes.
#[cfg(unix)]
#[allow(clippy::unnecessary_wraps)] // the non-unix twin can fail
fn path_from_bytes(bytes: &[u8]) -> Result<PathBuf, DiscoverError> {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    Ok(PathBuf::from(OsStr::from_bytes(bytes)))
}

/// A path from git's raw bytes, which elsewhere must be UTF-8.
#[cfg(not(unix))]
fn path_from_bytes(bytes: &[u8]) -> Result<PathBuf, DiscoverError> {
    String::from_utf8(bytes.to_vec())
        .map(PathBuf::from)
        .map_err(|_| DiscoverError::Git {
            detail: format!("non-UTF-8 path: {}", String::from_utf8_lossy(bytes)),
        })
}

/// Whether an entry of this kind can be a candidate: a regular file, or
/// a symlink for `symlink::screen` to judge (`src:V128`). A FIFO, socket
/// or device has no source to scan, and the engine opening a FIFO would
/// block until something wrote to it -- a gate that hangs.
fn scannable(kind: fs::FileType) -> bool {
    kind.is_file() || kind.is_symlink()
}

/// Every regular file and symlink under `dir` (relative to `root`), into
/// `files`.
///
/// Outside a repository there is no index to ask, so the directory named
/// is read as it stands. `DirEntry::file_type` does not follow links: a
/// symlinked directory is an entry, not a subtree to enter. Anything else
/// found -- a FIFO, a socket, a device -- is passed over, as git itself
/// never lists one.
fn walk(root: &Path, dir: &Path, files: &mut BTreeSet<PathBuf>) -> Result<(), DiscoverError> {
    let io_error = |e: io::Error| DiscoverError::Io {
        path: dir.to_path_buf(),
        detail: e.to_string(),
    };
    for entry in fs::read_dir(root.join(dir)).map_err(io_error)? {
        let entry = entry.map_err(io_error)?;
        // `.` (the root, named) adds nothing to a name: `a.sh`, not `./a.sh`.
        let rel = if dir == Path::new(".") {
            PathBuf::from(entry.file_name())
        } else {
            dir.join(entry.file_name())
        };
        let kind = entry.file_type().map_err(io_error)?;
        if kind.is_dir() {
            walk(root, &rel, files)?;
        } else if scannable(kind) {
            files.insert(rel);
        }
    }
    Ok(())
}
