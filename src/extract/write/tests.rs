use std::fs;
use std::path::Path;

use super::{apply, apply_with};
use crate::discover::{Sandbox, write as put};
use crate::extract::{Edit, HostEdit, NewFile};

fn new_file(path: &str, text: &str, executable: bool, present: bool) -> NewFile {
    NewFile {
        path: path.to_owned(),
        text: text.to_owned(),
        executable,
        present,
    }
}

fn edit(extracts: Vec<NewFile>) -> Edit {
    Edit {
        hosts: vec![HostEdit {
            path: "h.toy".to_owned(),
            before: "old\n".to_owned(),
            after: "new\n".to_owned(),
            extracts,
        }],
        ..Edit::default()
    }
}

fn read(root: &Path, rel: &str) -> String {
    fs::read_to_string(root.join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

#[test]
fn extracts_are_written_before_their_host_and_present_ones_skipped() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "h.toy", "old\n");
    put(&root, "d/kept.sh", "same\n");
    let plan = edit(vec![
        new_file("d/kept.sh", "same\n", true, true),
        new_file("d/deep/x.sh", "x\n", true, false),
    ]);
    let written = apply(&root, &plan).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(written, ["d/deep/x.sh", "h.toy"]);
    assert_eq!(read(&root, "d/deep/x.sh"), "x\n");
    assert_eq!(read(&root, "h.toy"), "new\n");
}

#[cfg(unix)]
#[test]
fn an_extract_gets_the_mode_its_guest_asks_for() {
    use std::os::unix::fs::PermissionsExt;
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "h.toy", "old\n");
    let plan = edit(vec![
        new_file("run.sh", "x\n", true, false),
        new_file("prog.jq", ".\n", false, false),
    ]);
    apply(&root, &plan).unwrap_or_else(|e| panic!("{e}"));
    let mode = |rel: &str| {
        fs::metadata(root.join(rel))
            .map_or_else(|e| panic!("{rel}: {e}"), |m| m.permissions().mode() & 0o777)
    };
    assert_eq!(mode("run.sh"), 0o755);
    assert_eq!(mode("prog.jq"), 0o644);
}

#[cfg(unix)]
#[test]
fn a_symlink_appearing_after_the_plan_stops_the_write() {
    // src/extract:V71, checked again at write time: the plan was made
    // against a tree that may have changed since.
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    let outside = sandbox.plain("outside");
    put(&root, "h.toy", "old\n");
    std::os::unix::fs::symlink(&outside, root.join("d")).unwrap_or_else(|e| panic!("symlink: {e}"));
    let plan = edit(vec![new_file("d/x.sh", "x\n", true, false)]);
    let err = apply(&root, &plan).err().unwrap_or_default();
    assert!(err.contains("src/extract:V71"), "{err}");
    assert!(!outside.join("x.sh").exists());
    assert_eq!(read(&root, "h.toy"), "old\n");
}

// ---------------------------------------------------------------------
// atomic, extracts first (T85)
// ---------------------------------------------------------------------

/// Every entry under `dir`, recursively, root relative and sorted.
fn tree(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            tree(root, &path, out);
        } else if let Ok(rel) = path.strip_prefix(root) {
            out.push(rel.display().to_string());
        }
    }
    out.sort();
}

#[test]
fn a_run_killed_before_its_host_leaves_a_whole_extract_and_the_old_host() {
    // src/extract:V84: the crash leaves an orphan extract, which
    // `xnl graph` reports, and never a load of a missing file.
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "h.toy", "old\n");
    let plan = edit(vec![new_file("h/x.sh", "whole\n", true, false)]);
    let mut seen = Vec::new();
    let mut kill = |path: &str| {
        seen.push(path.to_owned());
        if path == "h.toy" {
            Err(std::io::Error::other("killed"))
        } else {
            Ok(())
        }
    };
    let err = apply_with(&root, &plan, &mut kill)
        .err()
        .unwrap_or_default();
    assert!(err.contains("killed"), "{err}");
    assert_eq!(seen, ["h/x.sh", "h.toy"]);
    assert_eq!(read(&root, "h/x.sh"), "whole\n");
    assert_eq!(read(&root, "h.toy"), "old\n");
    let mut files = Vec::new();
    tree(&root, &root, &mut files);
    assert_eq!(files, ["h.toy", "h/x.sh"], "no temp file is left behind");
}

#[test]
fn a_write_leaves_no_temp_file_behind() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "h.toy", "old\n");
    let plan = edit(vec![new_file("h/x.sh", "x\n", true, false)]);
    apply(&root, &plan).unwrap_or_else(|e| panic!("{e}"));
    let mut files = Vec::new();
    tree(&root, &root, &mut files);
    assert_eq!(files, ["h.toy", "h/x.sh"]);
}

#[cfg(unix)]
#[test]
fn the_host_keeps_its_own_mode() {
    use std::os::unix::fs::PermissionsExt;
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "h.toy", "old\n");
    fs::set_permissions(root.join("h.toy"), fs::Permissions::from_mode(0o750))
        .unwrap_or_else(|e| panic!("chmod: {e}"));
    apply(&root, &edit(Vec::new())).unwrap_or_else(|e| panic!("{e}"));
    let mode = fs::metadata(root.join("h.toy")).map_or(0, |m| m.permissions().mode() & 0o777);
    assert_eq!(mode, 0o750);
}

// ---------------------------------------------------------------------
// one writer (T126)
// ---------------------------------------------------------------------

#[test]
fn a_concurrent_write_is_refused_and_writes_nothing() {
    // src/extract:V127: V64's all-or-nothing assumes one rewriter.
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "h.toy", "old\n");
    let held = crate::extract::lock::Lock::take(&root).unwrap_or_else(|e| panic!("{e}"));
    let plan = edit(vec![new_file("h/x.sh", "x\n", true, false)]);
    let err = apply(&root, &plan).err().unwrap_or_default();
    assert!(err.contains("src/extract:V127"), "{err}");
    assert!(!root.join("h/x.sh").exists());
    assert_eq!(read(&root, "h.toy"), "old\n");
    drop(held);
    apply(&root, &plan).unwrap_or_else(|e| panic!("{e}"));
    assert!(!root.join(crate::extract::lock::FILE).exists());
}

#[test]
fn a_host_changed_since_the_plan_is_refused_and_nothing_is_written() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "h.toy", "someone else's edit\n");
    let plan = edit(vec![new_file("h/x.sh", "x\n", true, false)]);
    let err = apply(&root, &plan).err().unwrap_or_default();
    assert!(err.contains("h.toy"), "{err}");
    assert!(err.contains("changed"), "{err}");
    assert!(!root.join("h/x.sh").exists());
    assert_eq!(read(&root, "h.toy"), "someone else's edit\n");
}
