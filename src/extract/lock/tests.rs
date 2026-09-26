use std::fs;

use super::{FILE, Lock};
use crate::discover::{Sandbox, write as put};

#[test]
fn a_lock_names_its_holder_and_is_gone_once_released() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    let lock = Lock::take(&root).unwrap_or_else(|e| panic!("{e}"));
    let text = fs::read_to_string(root.join(FILE)).unwrap_or_default();
    assert!(
        text.starts_with(&format!("pid {} start ", std::process::id())),
        "{text:?}"
    );
    drop(lock);
    assert!(!root.join(FILE).exists());
}

#[test]
fn a_second_writer_is_refused_naming_the_holder() {
    // src/extract:V127: one writer.
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    let held = Lock::take(&root).unwrap_or_else(|e| panic!("{e}"));
    let err = Lock::take(&root).err().unwrap_or_default();
    assert!(
        err.contains(&format!("pid {}", std::process::id())),
        "{err}"
    );
    assert!(err.contains("src/extract:V127"), "{err}");
    drop(held);
    assert!(Lock::take(&root).is_ok());
}

#[test]
fn a_stale_lock_whose_holder_is_gone_is_reclaimed() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, FILE, "pid 999999 start 1\n");
    let lock = Lock::take(&root).unwrap_or_else(|e| panic!("{e}"));
    let text = fs::read_to_string(root.join(FILE)).unwrap_or_default();
    assert!(!text.contains("999999"), "{text:?}");
    drop(lock);
}

#[cfg(unix)]
#[test]
fn a_lock_file_that_is_a_symlink_is_refused() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    let outside = sandbox.plain("outside");
    std::os::unix::fs::symlink(outside.join("lock"), root.join(FILE))
        .unwrap_or_else(|e| panic!("symlink: {e}"));
    let err = Lock::take(&root).err().unwrap_or_default();
    assert!(err.contains("src/extract:V71"), "{err}");
    assert!(!outside.join("lock").exists());
}
