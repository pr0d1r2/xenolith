//! Load resolution: the mirror of `src/graph/resolve.rs` (`src:C139`).
//!
//! Each case builds a plain tree in a [`Sandbox`]; none runs git.

use std::path::{Path, PathBuf};

use super::{Dangling, resolve};
use crate::discover::{Sandbox, write};

fn tree(sandbox: &Sandbox, files: &[&str]) -> PathBuf {
    let root = sandbox.plain("t");
    for rel in files {
        write(&root, rel, "echo hi\n");
    }
    root
}

#[test]
fn a_load_resolves_from_the_host_files_directory() {
    // `languages/api/src/lens:V66`: the default runtime base is the
    // host file's directory, never the working directory.
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &["scripts/a.sh", "sub/x.sh", "top.sh"]);
    assert_eq!(
        resolve(&root, "hk.pkl", Path::new("scripts/a.sh")),
        Ok("scripts/a.sh".to_owned())
    );
    assert_eq!(
        resolve(&root, "sub/h.pkl", Path::new("./x.sh")),
        Ok("sub/x.sh".to_owned())
    );
    assert_eq!(
        resolve(&root, "sub/h.pkl", Path::new("../top.sh")),
        Ok("top.sh".to_owned())
    );
}

#[test]
fn a_missing_dot_load_is_retried_from_the_repository_root() {
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &["scripts/lib.sh"]);
    assert_eq!(
        resolve(&root, "sub/a.sh", Path::new("./scripts/lib.sh")),
        Ok("scripts/lib.sh".to_owned())
    );
}

#[test]
fn a_root_retry_keeps_the_host_missing_error_when_it_escapes_root() {
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &[]);
    assert_eq!(
        resolve(&root, "sub/h.sh", Path::new("./../missing.sh")),
        Err(Dangling::Missing("missing.sh".to_owned()))
    );
}

#[test]
fn a_missing_file_is_dangling_and_named() {
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &[]);
    let got = resolve(&root, "sub/h.pkl", Path::new("gone.sh"));
    assert_eq!(got, Err(Dangling::Missing("sub/gone.sh".to_owned())));
    let why = got.err().map(|d| d.to_string()).unwrap_or_default();
    assert!(why.contains("sub/gone.sh"), "{why}");
}

#[test]
fn a_substitution_placeholder_is_unknown_at_runtime() {
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &[]);
    assert_eq!(
        resolve(&root, "h.nix", Path::new("@stateDir@/token.env")),
        Err(Dangling::Unknown)
    );
}

#[test]
fn a_placeholder_after_an_empty_at_pair_is_unknown_at_runtime() {
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &[]);
    assert_eq!(
        resolve(&root, "h.nix", Path::new("@@stateDir@/token.env")),
        Err(Dangling::Unknown)
    );
}

#[test]
fn at_signs_in_different_path_components_are_static() {
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &["a@v1/b@v2.sh"]);
    assert_eq!(
        resolve(&root, "h.nix", Path::new("a@v1/b@v2.sh")),
        Ok("a@v1/b@v2.sh".to_owned())
    );
}

#[test]
fn an_empty_placeholder_pair_in_a_filename_is_static() {
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &["assets@@2.sh"]);
    assert_eq!(
        resolve(&root, "h.nix", Path::new("assets@@2.sh")),
        Ok("assets@@2.sh".to_owned())
    );
}

#[test]
fn a_load_outside_the_root_is_dangling() {
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &[]);
    for load in ["../../x.sh", "/etc/profile", "../x.sh"] {
        assert_eq!(
            resolve(&root, "h.pkl", Path::new(load)),
            Err(Dangling::Outside),
            "{load}"
        );
    }
}

#[test]
fn a_directory_is_not_an_extract() {
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &["scripts/a.sh"]);
    assert_eq!(
        resolve(&root, "h.pkl", Path::new("scripts")),
        Err(Dangling::NotAFile("scripts".to_owned()))
    );
}

#[cfg(unix)]
#[test]
fn a_load_through_a_symlink_is_dangling() {
    // `src/graph:V72`: a symlinked file, or any symlinked directory on
    // the way, is never followed -- it may point outside the repository.
    use std::os::unix::fs::symlink;

    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &["real/a.sh"]);
    symlink(root.join("real"), root.join("linked")).unwrap_or_else(|e| panic!("{e}"));
    symlink(root.join("real/a.sh"), root.join("b.sh")).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        resolve(&root, "h.pkl", Path::new("linked/a.sh")),
        Err(Dangling::Symlink("linked".to_owned()))
    );
    assert_eq!(
        resolve(&root, "h.pkl", Path::new("b.sh")),
        Err(Dangling::Symlink("b.sh".to_owned()))
    );
    let why = Dangling::Symlink("b.sh".to_owned()).to_string();
    assert!(why.contains("symlink"), "{why}");
}

#[cfg(unix)]
#[test]
fn a_dot_load_only_falls_back_when_missing_from_the_host_directory() {
    use std::os::unix::fs::symlink;

    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &["x.sh"]);
    std::fs::create_dir(root.join("sub")).unwrap_or_else(|e| panic!("{e}"));
    std::fs::create_dir(root.join("sub/x.sh")).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        resolve(&root, "sub/h.pkl", Path::new("./x.sh")),
        Err(Dangling::NotAFile("sub/x.sh".to_owned()))
    );

    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &["x.sh"]);
    std::fs::create_dir(root.join("sub")).unwrap_or_else(|e| panic!("{e}"));
    symlink(root.join("x.sh"), root.join("sub/x.sh")).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        resolve(&root, "sub/h.pkl", Path::new("./x.sh")),
        Err(Dangling::Symlink("sub/x.sh".to_owned()))
    );
}

#[test]
fn a_load_naming_the_root_itself_is_not_a_file() {
    // `.` from a host at the root names no component at all: the root
    // is a directory, never an extract.
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &["a.sh"]);
    let got = resolve(&root, "h.pkl", Path::new("."));
    assert_eq!(got, Err(Dangling::NotAFile(".".to_owned())));
    let why = got.err().map(|d| d.to_string()).unwrap_or_default();
    assert_eq!(why, ". is not a regular file");
}

#[test]
fn a_host_dir_keeps_only_its_named_components() {
    // A `./` in the host's own path is no directory to resolve from.
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &["sub/x.sh"]);
    assert_eq!(
        resolve(&root, "./sub/h.pkl", Path::new("x.sh")),
        Ok("sub/x.sh".to_owned())
    );
}

#[test]
fn each_dangling_reason_reads_as_the_report_says_it() {
    assert_eq!(
        Dangling::Outside.to_string(),
        "it points outside the repository root"
    );
    assert_eq!(
        Dangling::NotAFile("scripts".to_owned()).to_string(),
        "scripts is not a regular file"
    );
}
