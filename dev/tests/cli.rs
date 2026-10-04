//! `xenolith-dev` end to end: the process boundary the unit tests in
//! `dev/src/tests.rs` cannot see -- argv, the environment, the working
//! directory and the exit status.
//!
//! Every spawn runs inside a scratch repository this file builds, and one
//! test asserts that it is not this repository: a binary whose job is to
//! rewrite a README must never be pointed at the real one by a test.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A scratch repository, removed on drop.
struct Repo {
    dir: PathBuf,
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn repo(name: &str) -> Repo {
    let dir = std::env::temp_dir().join(format!("xenolith-dev-e2e-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    for (rel, body) in [
        (".git/HEAD", "ref: refs/heads/main\n"),
        ("SPEC.md", "# SPEC\n"),
        (
            "Cargo.toml",
            "[workspace]\nmembers = []\n\n[workspace.package]\nedition = \"2024\"\n\
             rust-version = \"1.95\"\nlicense = \"MIT\"\n\n[workspace.lints.rust]\n\
             unsafe_code = \"forbid\"\n\n[features]\ndefault = []\n",
        ),
        (".coverage", "lines 97.96\n"),
        (".lint-debt", "density 0.0\n"),
        ("hk.pkl", ""),
        (
            "flake.lock",
            "{\n  \"nodes\": {\n    \"nixpkgs\": {\n      \"locked\": {\n        \
             \"lastModified\": 1789749394,\n        \"rev\": \"cf9d2fb3e50f\"\n      \
             }\n    }\n  }\n}\n",
        ),
        (".github/workflows/ci.yml", "        os: [ubuntu-latest]\n"),
        (
            "README.md",
            "# f\n<!-- BEGIN badges -->\n<!-- END badges -->\n\
             <!-- BEGIN langs -->\n<!-- END langs -->\n",
        ),
        ("src/deep/keep", ""),
        (
            "recorded/metadata.json",
            r#"{"packages": [], "workspace_members": [], "resolve": {"nodes": []}}"#,
        ),
        ("recorded/tools.json", "{}"),
    ] {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .unwrap_or_else(|e| panic!("mkdir {}: {e}", parent.display()));
        }
        fs::write(&path, body).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
    }
    Repo { dir }
}

fn run(cwd: &Path, root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_xenolith-dev"))
        .args(args)
        .current_dir(cwd)
        .env(
            "XENOLITH_DEV_CARGO_METADATA",
            root.join("recorded/metadata.json"),
        )
        .env(
            "XENOLITH_DEV_TOOL_LICENSES",
            root.join("recorded/tools.json"),
        )
        .output()
        .unwrap_or_else(|e| panic!("xenolith-dev did not run: {e}"))
}

/// The guard for every other test here.
#[test]
fn a_scratch_repository_is_not_this_one() {
    let r = repo("isolation");
    let ours = Path::new(env!("CARGO_MANIFEST_DIR"));
    assert!(!r.dir.starts_with(ours.parent().unwrap_or(ours)));
}

#[test]
fn a_usage_error_exits_two_and_prints_the_usage() {
    let r = repo("usage");
    let out = run(&r.dir, &r.dir, &["nonsense"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("xenolith-dev --check"));
}

/// From a subdirectory, with the owners' answers from the environment:
/// the check refuses, `--fix` repairs, and the check is then clean.
#[test]
fn check_fix_check_from_a_subdirectory() {
    let r = repo("roundtrip");
    let deep = r.dir.join("src/deep");
    let before = run(&deep, &r.dir, &["--check"]);
    assert_eq!(before.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&before.stderr).contains("readme failed"));
    assert_eq!(run(&deep, &r.dir, &["--fix"]).status.code(), Some(0));
    let after = run(&deep, &r.dir, &["--check"]);
    assert_eq!(
        after.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&after.stderr)
    );
    let readme = fs::read_to_string(r.dir.join("README.md")).unwrap_or_default();
    assert!(readme.contains("linux-5277C3?logo=intel"), "{readme}");
}
