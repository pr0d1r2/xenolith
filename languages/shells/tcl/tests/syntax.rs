//! The `xenolith-tcl-syntax` binary end to end
//! (`languages/shells/tcl:V198`): the process the lint engine spawns,
//! run as it runs it, on files written for the test.

use std::path::PathBuf;
use std::process::Command;

/// A fresh directory for this test's files.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "xenolith-tcl-syntax-bin-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    dir
}

/// The binary on `files`, from `dir`: exit code and stdout.
fn check(dir: &PathBuf, files: &[&str]) -> (Option<i32>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_xenolith-tcl-syntax"))
        .current_dir(dir)
        .args(files)
        .output()
        .unwrap_or_else(|e| panic!("spawn: {e}"));
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

#[test]
fn a_clean_file_passes_and_a_broken_one_fails_with_its_position() {
    let dir = scratch("verdicts");
    std::fs::write(
        dir.join("ok.exp"),
        "#!/usr/bin/env expect\nspawn ssh host\nexpect \"$ \"\nsend \"ls\\r\"\n",
    )
    .unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(dir.join("bad.tcl"), "proc f {} {\n    puts hi\n\nf\n")
        .unwrap_or_else(|e| panic!("{e}"));

    assert_eq!(check(&dir, &["ok.exp"]), (Some(0), String::new()));
    assert_eq!(
        check(&dir, &["ok.exp", "bad.tcl"]),
        (Some(1), "bad.tcl:1:11: missing close-brace\n".to_owned())
    );
    std::fs::remove_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn no_file_is_a_usage_error() {
    let dir = scratch("usage");
    assert_eq!(
        check(&dir, &[]),
        (Some(2), "usage: xenolith-tcl-syntax FILE...\n".to_owned())
    );
    std::fs::remove_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
}
