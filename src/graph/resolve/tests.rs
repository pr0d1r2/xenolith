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
fn a_missing_file_is_dangling_and_named() {
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &[]);
    let got = resolve(&root, "sub/h.pkl", Path::new("gone.sh"));
    assert_eq!(got, Err(Dangling::Missing("sub/gone.sh".to_owned())));
    let why = got.err().map(|d| d.to_string()).unwrap_or_default();
    assert!(why.contains("sub/gone.sh"), "{why}");
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
