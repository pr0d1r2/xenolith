use std::fs;
use std::path::Path;

use super::apply;
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
