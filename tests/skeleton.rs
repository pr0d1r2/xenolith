//! The root crate exists, links, and its binary answers for itself.
//!
//! Deliberately thin: the model (`src:T8`), the verbs (`src/cli:T9`) and
//! the engines each have their own tasks and their own tests. What this
//! file pins is the part that is easy to get wrong once and never look at
//! again -- that the library and the binary agree on a version, and that
//! the binary REFUSES rather than pretends when asked to do work it cannot
//! do yet.

use std::process::Command;

const XNL: &str = env!("CARGO_BIN_EXE_xnl");

#[test]
fn library_reports_the_package_version() {
    assert_eq!(xenolith::VERSION, env!("CARGO_PKG_VERSION"));
}

#[test]
fn binary_prints_the_same_version_the_library_reports() {
    let out = Command::new(XNL)
        .arg("--version")
        .output()
        .unwrap_or_else(|e| panic!("running {XNL}: {e}"));
    assert!(out.status.success(), "--version must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains(xenolith::VERSION),
        "--version printed {stdout:?}, which does not contain {}",
        xenolith::VERSION
    );
}

#[test]
fn a_verb_whose_engine_has_not_landed_exits_two_and_says_so() {
    let out = Command::new(XNL)
        .args(["extract", "--relocate", "a.nix"])
        .output()
        .unwrap_or_else(|e| panic!("running {XNL}: {e}"));
    assert_eq!(
        out.status.code(),
        Some(2),
        "an unimplemented verb must exit 2, not pretend to succeed"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("extract"),
        "the refusal must name the verb asked for; got {stderr:?}"
    );
}

#[test]
fn no_arguments_is_a_usage_error() {
    let out = Command::new(XNL)
        .output()
        .unwrap_or_else(|e| panic!("running {XNL}: {e}"));
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("usage"), "got {stderr:?}");
}

#[test]
fn langs_exits_zero_and_lists_languages_on_stdout() {
    let out = Command::new(XNL)
        .arg("langs")
        .output()
        .unwrap_or_else(|e| panic!("running {XNL}: {e}"));
    assert!(out.status.success(), "`xnl langs` must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("shell"), "got {stdout:?}");
    assert!(out.stderr.is_empty());
}

#[cfg(unix)]
#[test]
fn a_path_that_is_not_utf8_is_read_not_a_panic() {
    // `std::env::args` panics on such an argument; the binary must read
    // its arguments as `OsString`s, because hk hands over whatever names
    // the tree holds. The file does not exist, so `check` refuses -- as
    // discovery's refusal naming the path (`src/discover:V57`), not as a panic.
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let out = Command::new(XNL)
        .arg("check")
        .arg(OsString::from_vec(vec![b'a', 0xff, b'.', b'n', b'i', b'x']))
        .output()
        .unwrap_or_else(|e| panic!("running {XNL}: {e}"));
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("panicked"), "got {stderr:?}");
    assert!(stderr.contains("no such file"), "got {stderr:?}");
}
