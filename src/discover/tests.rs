//! Candidate discovery: the mirror of `src/discover.rs` (`src:C139`).
//!
//! Every case builds a throwaway tree under the system temp dir, and every
//! `git` these tests run -- the fixture's own `init` and `add`, and the one
//! the code under test spawns -- goes through [`Sandbox::git`]. That is not
//! tidiness. hk runs `cargo test` from a git hook, git exports `GIT_DIR`
//! and `GIT_INDEX_FILE` to hooks, and those beat `-C`: an unsandboxed
//! `git -C <tmp> add` in a test writes into the repository being committed
//! (`tests:B1`, `tests:V150`). The product code is meant to inherit that
//! environment -- inside a hook it SHOULD list the hook's repository -- so
//! the tests reach it through `discover_with` and hand it a sandboxed
//! command instead.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::{DiscoverError, discover, discover_with};

// ---------------------------------------------------------------------
// the sandbox (`tests:V150`, `tests:B1`)
// ---------------------------------------------------------------------

/// A temp directory removed on drop, and the one way to run git in it.
pub(crate) struct Sandbox {
    dir: PathBuf,
}

impl Sandbox {
    /// A fresh, empty directory. Canonical, because the temp dir itself
    /// sits under a symlink on macOS (`/var` -> `/private/var`) and git's
    /// ceiling comparison is textual.
    pub(crate) fn new() -> Sandbox {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let base = fs::canonicalize(std::env::temp_dir())
            .unwrap_or_else(|e| panic!("temp dir not canonicalizable: {e}"));
        let dir = base.join(format!(
            "xenolith-discover-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("mkdir {}: {e}", dir.display()));
        fs::write(dir.join("gitconfig"), "")
            .unwrap_or_else(|e| panic!("write gitconfig in {}: {e}", dir.display()));
        Sandbox { dir }
    }

    /// The sandbox root; fixtures live in subdirectories of it.
    pub(crate) fn path(&self) -> &Path {
        &self.dir
    }

    /// `git`, cut off from everything outside the sandbox: every `GIT_*`
    /// variable this process inherited is removed (a hook's `GIT_DIR`
    /// among them), the global config is an empty file here, the system
    /// config is off, and repository discovery stops at the sandbox.
    pub(crate) fn git(&self) -> Command {
        let mut cmd = Command::new("git");
        for (key, _) in std::env::vars_os() {
            if key.to_string_lossy().starts_with("GIT_") {
                cmd.env_remove(key);
            }
        }
        cmd.env("GIT_CONFIG_GLOBAL", self.dir.join("gitconfig"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CEILING_DIRECTORIES", &self.dir);
        cmd
    }

    /// Run a sandboxed git in `cwd`, identity passed as flags (never
    /// `git config`, which is the write that corrupted `.git/config` in
    /// `tests:B1`), and panic with its stderr if it fails.
    pub(crate) fn run_git(&self, cwd: &Path, args: &[&str]) {
        let out = self
            .git()
            .arg("-c")
            .arg("user.name=xenolith-test")
            .arg("-c")
            .arg("user.email=test@example.invalid")
            .arg("-c")
            .arg("init.defaultBranch=main")
            .arg("-C")
            .arg(cwd)
            .args(args)
            .output()
            .unwrap_or_else(|e| panic!("git {args:?} did not run: {e}"));
        assert!(
            out.status.success(),
            "git {args:?} in {} failed: {}",
            cwd.display(),
            String::from_utf8_lossy(&out.stderr)
        );
    }

    /// A new directory `name` in the sandbox, `git init`ed.
    pub(crate) fn repo(&self, name: &str) -> PathBuf {
        let root = self.dir.join(name);
        fs::create_dir_all(&root).unwrap_or_else(|e| panic!("mkdir {}: {e}", root.display()));
        self.run_git(&root, &["init", "-q"]);
        root
    }

    /// A new plain directory `name` in the sandbox, no repository.
    pub(crate) fn plain(&self, name: &str) -> PathBuf {
        let root = self.dir.join(name);
        fs::create_dir_all(&root).unwrap_or_else(|e| panic!("mkdir {}: {e}", root.display()));
        root
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// Write `rel` under `root`, creating parent directories.
pub(crate) fn write(root: &Path, rel: &str, contents: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap_or_else(|e| panic!("mkdir {}: {e}", parent.display()));
    }
    fs::write(&path, contents).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
}

/// Paths as `&str`s, the shape assertions compare against.
pub(super) fn names(files: &[PathBuf]) -> Vec<String> {
    files.iter().map(|p| p.display().to_string()).collect()
}

/// `rel`s as the `paths` argument.
pub(super) fn paths(rels: &[&str]) -> Vec<PathBuf> {
    rels.iter().map(PathBuf::from).collect()
}

/// Discover through the sandboxed git, expecting success.
pub(super) fn found(sandbox: &Sandbox, root: &Path, rels: &[&str]) -> super::Candidates {
    match discover_with(root, &paths(rels), &|| sandbox.git()) {
        Ok(candidates) => candidates,
        Err(e) => panic!("expected candidates, got: {e}"),
    }
}

/// Discover through the sandboxed git, expecting a refusal.
pub(super) fn refused(sandbox: &Sandbox, root: &Path, rels: &[&str]) -> DiscoverError {
    match discover_with(root, &paths(rels), &|| sandbox.git()) {
        Ok(candidates) => panic!("expected a refusal, got: {candidates:?}"),
        Err(e) => e,
    }
}

#[test]
fn the_sandbox_strips_every_inherited_git_variable() {
    // The regression guard for `tests:B1` on the Rust side. Under hk the
    // test step runs with the hook's `GIT_DIR` and `GIT_INDEX_FILE` set,
    // and this is the test that notices if the sandbox stops removing
    // them; outside a hook it still pins the three variables it sets.
    let sandbox = Sandbox::new();
    let cmd = sandbox.git();
    let envs: Vec<(&OsStr, Option<&OsStr>)> = cmd.get_envs().collect();
    let own = [
        "GIT_CONFIG_GLOBAL",
        "GIT_CONFIG_NOSYSTEM",
        "GIT_CEILING_DIRECTORIES",
    ];
    for (key, _) in std::env::vars_os() {
        let name = key.to_string_lossy().into_owned();
        if !name.starts_with("GIT_") || own.contains(&name.as_str()) {
            continue;
        }
        assert!(
            envs.contains(&(key.as_os_str(), None)),
            "{name} is inherited by the sandboxed git"
        );
    }
    let set = |k: &str| envs.iter().find(|(key, _)| *key == k).and_then(|(_, v)| *v);
    assert_eq!(
        set("GIT_CONFIG_GLOBAL"),
        Some(sandbox.path().join("gitconfig").as_os_str())
    );
    assert_eq!(set("GIT_CONFIG_NOSYSTEM"), Some(OsStr::new("1")));
    assert_eq!(
        set("GIT_CEILING_DIRECTORIES"),
        Some(sandbox.path().as_os_str())
    );
}

// ---------------------------------------------------------------------
// no paths: `git ls-files` (`src:V57`)
// ---------------------------------------------------------------------

#[test]
fn without_paths_the_candidates_are_the_tracked_files_sorted() {
    let sandbox = Sandbox::new();
    let root = sandbox.repo("r");
    write(&root, "z.sh", "echo z\n");
    write(&root, "a/b.nix", "{ }\n");
    write(&root, "a.pkl", "x = 1\n");
    sandbox.run_git(&root, &["add", "."]);
    let got = found(&sandbox, &root, &[]);
    // Path order, component by component -- `a/b.nix` before `a.pkl` --
    // the same order `Report` sorts violations by `file` in (`src:V11`),
    // not git's byte order.
    assert_eq!(names(&got.files), ["a/b.nix", "a.pkl", "z.sh"]);
    assert!(got.warnings.is_empty(), "{:?}", got.warnings);
}

#[test]
fn untracked_and_ignored_files_are_not_candidates() {
    // The fixture `src:T58` names: tracked only, so `.gitignore` is
    // honoured for free and a stray build output is never scanned.
    let sandbox = Sandbox::new();
    let root = sandbox.repo("r");
    write(&root, ".gitignore", "*.log\nbuild/\n");
    write(&root, "tracked.sh", "echo hi\n");
    sandbox.run_git(&root, &["add", "."]);
    write(&root, "debug.log", "noise\n");
    write(&root, "build/out.sh", "echo built\n");
    write(&root, "untracked.sh", "echo new\n");
    let got = found(&sandbox, &root, &[]);
    assert_eq!(names(&got.files), [".gitignore", "tracked.sh"]);
}

#[test]
fn a_tracked_file_deleted_from_the_work_tree_is_not_a_candidate() {
    // `ls-files` still lists it from the index, but there are no bytes to
    // scan, and reporting a read error for a deletion the user is about
    // to commit would fail the very commit that removes the file.
    let sandbox = Sandbox::new();
    let root = sandbox.repo("r");
    write(&root, "kept.sh", "echo kept\n");
    write(&root, "gone.sh", "echo gone\n");
    sandbox.run_git(&root, &["add", "."]);
    fs::remove_file(root.join("gone.sh")).unwrap_or_else(|e| panic!("rm: {e}"));
    let got = found(&sandbox, &root, &[]);
    assert_eq!(names(&got.files), ["kept.sh"]);
}

#[test]
fn a_tracked_name_with_spaces_and_a_newline_survives_the_listing() {
    // `-z`, not line splitting and not git's quoted-path escaping: a
    // filename is bytes, and the candidate must be the file on disk.
    let sandbox = Sandbox::new();
    let root = sandbox.repo("r");
    write(&root, "with space.sh", "echo 1\n");
    write(&root, "new\nline.sh", "echo 2\n");
    write(&root, "calf\u{e9}.sh", "echo 3\n");
    sandbox.run_git(&root, &["add", "."]);
    let got = found(&sandbox, &root, &[]);
    assert_eq!(
        names(&got.files),
        ["calf\u{e9}.sh", "new\nline.sh", "with space.sh"]
    );
    for file in &got.files {
        assert!(
            root.join(file).is_file(),
            "{} is not on disk",
            file.display()
        );
    }
}

#[test]
fn a_root_below_the_top_level_lists_only_its_own_subtree_relative_to_itself() {
    let sandbox = Sandbox::new();
    let top = sandbox.repo("r");
    write(&top, "outside.sh", "echo o\n");
    write(&top, "sub/inside.sh", "echo i\n");
    sandbox.run_git(&top, &["add", "."]);
    let got = found(&sandbox, &top.join("sub"), &[]);
    assert_eq!(names(&got.files), ["inside.sh"]);
}

#[test]
fn an_empty_repository_has_no_candidates_and_is_not_an_error() {
    let sandbox = Sandbox::new();
    let root = sandbox.repo("r");
    let got = found(&sandbox, &root, &[]);
    assert!(got.files.is_empty(), "{:?}", got.files);
}

#[test]
fn outside_git_without_paths_is_refused_with_exit_2() {
    // `src:V57`: with neither a repository nor paths there is no honest
    // answer to "which files", and scanning nothing then exiting 0 is
    // indistinguishable, in a gate, from a clean tree.
    let sandbox = Sandbox::new();
    let root = sandbox.plain("p");
    write(&root, "a.sh", "echo a\n");
    let e = refused(&sandbox, &root, &[]);
    assert!(matches!(e, DiscoverError::NotARepo { .. }), "{e:?}");
    assert_eq!(e.exit_code(), 2);
    let text = e.to_string();
    assert!(text.contains("not a git repository"), "{text}");
    assert!(text.contains(&root.display().to_string()), "{text}");
    assert!(text.contains("src:V57"), "{text}");
}

#[test]
fn a_git_that_cannot_run_is_refused_with_exit_2_not_treated_as_empty() {
    let sandbox = Sandbox::new();
    let root = sandbox.repo("r");
    write(&root, "a.sh", "echo a\n");
    sandbox.run_git(&root, &["add", "."]);
    let missing = sandbox.path().join("no-such-git");
    let e = match discover_with(&root, &[], &|| Command::new(&missing)) {
        Ok(c) => panic!("expected a refusal, got: {c:?}"),
        Err(e) => e,
    };
    assert!(matches!(e, DiscoverError::Git { .. }), "{e:?}");
    assert_eq!(e.exit_code(), 2);
    assert!(e.to_string().contains("git"), "{e}");
}

#[test]
fn a_git_that_fails_for_another_reason_is_a_git_error_not_not_a_repo() {
    // `false` exits 1 with nothing on stderr: whatever went wrong, it was
    // not "no repository", and saying so would send the user looking in
    // the wrong place.
    let sandbox = Sandbox::new();
    let root = sandbox.repo("r");
    let e = match discover_with(&root, &[], &|| Command::new("false")) {
        Ok(c) => panic!("expected a refusal, got: {c:?}"),
        Err(e) => e,
    };
    assert!(matches!(e, DiscoverError::Git { .. }), "{e:?}");
    assert_eq!(e.exit_code(), 2);
}

// ---------------------------------------------------------------------
// explicit paths (`src:V57`: the hk file list)
// ---------------------------------------------------------------------

#[test]
fn explicit_files_are_the_candidates_even_untracked() {
    // hk hands over the file list it decided on; second-guessing it with
    // the index would drop a file hk means to check.
    let sandbox = Sandbox::new();
    let root = sandbox.repo("r");
    write(&root, "tracked.sh", "echo t\n");
    sandbox.run_git(&root, &["add", "."]);
    write(&root, "untracked.sh", "echo u\n");
    let got = found(&sandbox, &root, &["untracked.sh"]);
    assert_eq!(names(&got.files), ["untracked.sh"]);
}

#[test]
fn explicit_files_outside_git_are_honoured() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("p");
    write(&root, "b.sh", "echo b\n");
    write(&root, "a.nix", "{ }\n");
    let got = found(&sandbox, &root, &["b.sh", "a.nix"]);
    assert_eq!(names(&got.files), ["a.nix", "b.sh"]);
}

#[test]
fn explicit_paths_come_out_sorted_and_once() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("p");
    write(&root, "b.sh", "echo b\n");
    write(&root, "a.sh", "echo a\n");
    let got = found(&sandbox, &root, &["b.sh", "a.sh", "b.sh"]);
    assert_eq!(names(&got.files), ["a.sh", "b.sh"]);
}

#[test]
fn an_explicit_path_that_does_not_exist_is_refused_with_exit_2() {
    let sandbox = Sandbox::new();
    let root = sandbox.repo("r");
    let e = refused(&sandbox, &root, &["nope.sh"]);
    assert_eq!(
        e,
        DiscoverError::Missing {
            path: PathBuf::from("nope.sh")
        }
    );
    assert_eq!(e.exit_code(), 2);
    assert!(e.to_string().contains("nope.sh"), "{e}");
}

#[test]
fn an_explicit_directory_in_a_repository_means_its_tracked_files() {
    let sandbox = Sandbox::new();
    let root = sandbox.repo("r");
    write(&root, "d/tracked.sh", "echo t\n");
    write(&root, "d/deep/tracked.nix", "{ }\n");
    write(&root, "other.sh", "echo o\n");
    sandbox.run_git(&root, &["add", "."]);
    write(&root, "d/untracked.sh", "echo u\n");
    let got = found(&sandbox, &root, &["d"]);
    assert_eq!(names(&got.files), ["d/deep/tracked.nix", "d/tracked.sh"]);
}

#[test]
fn an_explicit_directory_with_no_tracked_file_is_refused_with_exit_2() {
    // A directory named in a repository means its tracked files, so one
    // holding only untracked and ignored files yields nothing -- and an
    // empty scan exiting 0 reads, in a gate, as that directory clean.
    let sandbox = Sandbox::new();
    let root = sandbox.repo("r");
    write(&root, ".gitignore", "*.log\n");
    write(&root, "top.sh", "echo t\n");
    sandbox.run_git(&root, &["add", "."]);
    write(&root, "d/untracked.sh", "echo u\n");
    write(&root, "d/debug.log", "noise\n");
    let e = refused(&sandbox, &root, &["top.sh", "d"]);
    assert_eq!(e.exit_code(), 2);
    let text = e.to_string();
    assert!(text.starts_with("d: "), "{text}");
    assert!(text.contains("src:V57"), "{text}");
}

#[test]
fn an_explicit_empty_directory_outside_git_is_refused_with_exit_2() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("p");
    write(&root, "a.sh", "echo a\n");
    fs::create_dir_all(root.join("d/e")).unwrap_or_else(|e| panic!("mkdir: {e}"));
    let e = refused(&sandbox, &root, &["a.sh", "d"]);
    assert_eq!(e.exit_code(), 2);
    assert!(e.to_string().starts_with("d: "), "{e}");
}

#[test]
fn an_explicit_directory_is_a_literal_path_not_a_glob() {
    // A directory named `d*` must not also pull in `dx/`: git reads
    // pathspecs as globs unless told otherwise.
    let sandbox = Sandbox::new();
    let root = sandbox.repo("r");
    write(&root, "d*/star.sh", "echo s\n");
    write(&root, "dx/x.sh", "echo x\n");
    sandbox.run_git(&root, &["add", "."]);
    let got = found(&sandbox, &root, &["d*"]);
    assert_eq!(names(&got.files), ["d*/star.sh"]);
}

#[test]
fn an_explicit_directory_outside_git_is_walked() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("p");
    write(&root, "d/a.sh", "echo a\n");
    write(&root, "d/e/b.nix", "{ }\n");
    write(&root, "other.sh", "echo o\n");
    let got = found(&sandbox, &root, &["d"]);
    assert_eq!(names(&got.files), ["d/a.sh", "d/e/b.nix"]);
}

#[test]
fn explicit_files_and_directories_mix_and_merge() {
    let sandbox = Sandbox::new();
    let root = sandbox.repo("r");
    write(&root, "d/a.sh", "echo a\n");
    write(&root, "top.sh", "echo t\n");
    sandbox.run_git(&root, &["add", "."]);
    let got = found(&sandbox, &root, &["top.sh", "d", "d/a.sh"]);
    assert_eq!(names(&got.files), ["d/a.sh", "top.sh"]);
}

// ---------------------------------------------------------------------
// only regular files: a FIFO, socket or device has no source to judge
// ---------------------------------------------------------------------

/// A named pipe at `rel` under `root`. Opening one for reading blocks
/// until a writer comes, which is what makes it dangerous as a candidate.
#[cfg(unix)]
fn fifo(root: &Path, rel: &str) {
    let path = root.join(rel);
    let out = Command::new("mkfifo")
        .arg(&path)
        .output()
        .unwrap_or_else(|e| panic!("mkfifo did not run: {e}"));
    assert!(out.status.success(), "mkfifo {}", path.display());
}

/// `job`'s result, or a panic if it is not back within ten seconds: a
/// candidate the engine blocks on must fail the test, not hang the run.
#[cfg(unix)]
fn within<T: Send + 'static>(job: impl FnOnce() -> T + Send + 'static) -> T {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(job());
    });
    rx.recv_timeout(std::time::Duration::from_secs(10))
        .unwrap_or_else(|e| panic!("discovery + reading the candidates hung: {e}"))
}

/// Discover as `found` does, then read every candidate as the engine
/// does (`src:V152`), all under [`within`].
#[cfg(unix)]
fn found_and_read(sandbox: Sandbox, root: PathBuf, rels: &'static [&'static str]) -> Vec<String> {
    within(move || {
        let got = found(&sandbox, &root, rels);
        for file in &got.files {
            let _ = fs::read(root.join(file));
        }
        names(&got.files)
    })
}

#[cfg(unix)]
#[test]
fn a_fifo_in_a_walked_directory_is_not_a_candidate() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("p");
    write(&root, "d/a.sh", "echo a\n");
    fifo(&root, "d/pipe");
    assert_eq!(found_and_read(sandbox, root, &["d"]), ["d/a.sh"]);
}

#[cfg(unix)]
#[test]
fn a_tracked_file_replaced_by_a_fifo_is_not_a_candidate() {
    let sandbox = Sandbox::new();
    let root = sandbox.repo("r");
    write(&root, "a.sh", "echo a\n");
    write(&root, "b.sh", "echo b\n");
    sandbox.run_git(&root, &["add", "."]);
    fs::remove_file(root.join("b.sh")).unwrap_or_else(|e| panic!("rm: {e}"));
    fifo(&root, "b.sh");
    assert_eq!(found_and_read(sandbox, root, &[]), ["a.sh"]);
}

#[cfg(unix)]
#[test]
fn an_explicit_fifo_is_refused_with_exit_2() {
    // Named, it is refused rather than skipped, as a named symlink is
    // (`src:V128`): skipping the one path asked about reads as clean.
    let sandbox = Sandbox::new();
    let root = sandbox.plain("p");
    fifo(&root, "pipe");
    let e = refused(&sandbox, &root, &["pipe"]);
    assert_eq!(e.exit_code(), 2);
    let text = e.to_string();
    assert!(text.starts_with("pipe: "), "{text}");
    assert!(text.contains("not a regular file"), "{text}");
}

// ---------------------------------------------------------------------
// the public entry point
// ---------------------------------------------------------------------

#[test]
fn discover_takes_explicit_files_without_consulting_git() {
    // The one call of the real `discover` here: explicit files never
    // spawn git, so the test is safe under a hook's environment.
    let sandbox = Sandbox::new();
    let root = sandbox.plain("p");
    write(&root, "a.sh", "echo a\n");
    let got = match discover(&root, &paths(&["a.sh"])) {
        Ok(c) => c,
        Err(e) => panic!("expected candidates, got: {e}"),
    };
    assert_eq!(names(&got.files), ["a.sh"]);
}

#[test]
fn every_refusal_names_its_spec_rule() {
    let cases = [
        DiscoverError::NotARepo {
            root: PathBuf::from("/x"),
        },
        DiscoverError::Git {
            detail: "boom".to_owned(),
        },
        DiscoverError::Missing {
            path: PathBuf::from("a"),
        },
        DiscoverError::EmptyDir {
            path: PathBuf::from("d"),
        },
        DiscoverError::Io {
            path: PathBuf::from("a"),
            detail: "denied".to_owned(),
        },
    ];
    for e in cases {
        assert_eq!(e.exit_code(), 2, "{e:?}");
        assert!(e.to_string().contains("src:V57"), "{e}");
    }
}
