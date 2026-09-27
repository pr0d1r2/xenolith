use std::fs;

use super::{Flags, render, run, write};
use crate::cli::args::Target;
use crate::discover::{Sandbox, write as put};
use crate::extract::{Edit, Gone, HostEdit, NewFile, Refusal};

fn change() -> HostEdit {
    HostEdit {
        path: "h.toy".to_owned(),
        before: "old\n".to_owned(),
        after: "new\n".to_owned(),
        extracts: vec![NewFile {
            path: "h/x.sh".to_owned(),
            text: "x\n".to_owned(),
            executable: false,
            present: false,
        }],
        removes: Vec::new(),
    }
}

fn refusal() -> Refusal {
    Refusal {
        file: "h.toy".to_owned(),
        line: 3,
        message: "no.".to_owned(),
    }
}

fn rendered(edit: &Edit, verbose: bool) -> (u8, String, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = render(edit, verbose, &mut out, &mut err);
    (
        code,
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    )
}

fn target(path: &str) -> Target {
    Target {
        path: path.into(),
        line: None,
    }
}

#[test]
fn an_empty_edit_is_silent_and_exits_zero() {
    assert_eq!(
        rendered(&Edit::default(), false),
        (0, String::new(), String::new())
    );
}

#[test]
fn the_diff_alone_goes_to_stdout_and_exits_one() {
    let edit = Edit {
        hosts: vec![change()],
        explain: vec!["h.toy:1:1 s: path = h/x.sh (host)".to_owned()],
        ..Edit::default()
    };
    let (code, out, err) = rendered(&edit, false);
    assert_eq!(code, 1);
    assert_eq!(out, edit.diff());
    assert!(out.starts_with("removing xenolith → h/x.sh\n"), "{out}");
    assert!(err.is_empty(), "{err:?}");
    let (_, _, err) = rendered(&edit, true);
    assert_eq!(err, "h.toy:1:1 s: path = h/x.sh (host)\n");
}

#[test]
fn a_refusal_goes_to_stderr_and_exits_two_over_a_diff() {
    let edit = Edit {
        hosts: vec![change()],
        refusals: vec![refusal()],
        ..Edit::default()
    };
    let (code, out, err) = rendered(&edit, false);
    assert_eq!(code, 2);
    assert!(!out.is_empty());
    assert_eq!(err, "xnl: h.toy:3: no.\n");
}

#[test]
fn write_applies_the_edit_and_says_what_it_wrote_when_verbose() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "h.toy", "old\n");
    let edit = Edit {
        hosts: vec![change()],
        ..Edit::default()
    };
    let mut err = Vec::new();
    assert_eq!(write(&root, &edit, true, &mut err), 0);
    assert_eq!(String::from_utf8_lossy(&err), "wrote h/x.sh\nwrote h.toy\n");
    assert_eq!(
        fs::read_to_string(root.join("h.toy")).ok().as_deref(),
        Some("new\n")
    );
}

#[test]
fn write_says_which_files_it_removed_when_verbose() {
    // `--relocate` and `xnl inline` remove the old extract last
    // (`src/extract:V99`, `src/extract:V101`).
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "h.toy", "old\n");
    put(&root, "old/x.sh", "x\n");
    let mut host = change();
    host.removes = vec![Gone {
        path: "old/x.sh".to_owned(),
        text: "x\n".to_owned(),
        to: Some("h/x.sh".to_owned()),
    }];
    let edit = Edit {
        hosts: vec![host],
        ..Edit::default()
    };
    let (_, out, _) = rendered(&edit, false);
    assert!(
        out.starts_with("moving extract old/x.sh → h/x.sh\n"),
        "{out}"
    );
    let mut err = Vec::new();
    assert_eq!(write(&root, &edit, true, &mut err), 0);
    assert_eq!(
        String::from_utf8_lossy(&err),
        "wrote h/x.sh\nwrote h.toy\nremoved old/x.sh\n"
    );
    assert!(!root.join("old/x.sh").exists());
}

#[test]
fn a_removed_file_changed_since_the_plan_stops_the_write() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "h.toy", "old\n");
    put(&root, "old/x.sh", "edited\n");
    let mut host = change();
    host.removes = vec![Gone {
        path: "old/x.sh".to_owned(),
        text: "x\n".to_owned(),
        to: None,
    }];
    let edit = Edit {
        hosts: vec![host],
        ..Edit::default()
    };
    let mut err = Vec::new();
    assert_eq!(write(&root, &edit, false, &mut err), 2);
    let err = String::from_utf8_lossy(&err);
    assert!(err.contains("old/x.sh changed since the plan"), "{err}");
    assert_eq!(
        fs::read_to_string(root.join("h.toy")).ok().as_deref(),
        Some("old\n"),
        "nothing was written"
    );
}

#[test]
fn write_with_a_refusal_still_writes_the_rest_and_exits_two() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "h.toy", "old\n");
    let edit = Edit {
        hosts: vec![change()],
        refusals: vec![refusal()],
        ..Edit::default()
    };
    let mut err = Vec::new();
    assert_eq!(write(&root, &edit, false, &mut err), 2);
    assert!(root.join("h/x.sh").is_file());
}

#[test]
fn a_file_no_host_claims_has_nothing_to_extract() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "notes.md", "# notes\n");
    for flags in [
        Flags::default(),
        Flags {
            write: true,
            ..Flags::default()
        },
    ] {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = run(&root, &[target("notes.md")], flags, &mut out, &mut err);
        assert_eq!(code, 0, "{}", String::from_utf8_lossy(&err));
        assert!(out.is_empty() && err.is_empty());
    }
}

#[test]
fn a_missing_path_or_a_broken_config_exits_two() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run(
        &root,
        &[target("nope.nix")],
        Flags::default(),
        &mut out,
        &mut err,
    );
    assert_eq!(code, 2);
    assert!(String::from_utf8_lossy(&err).contains("nope.nix"));
    put(&root, "xenolith.toml", "version = [\n");
    put(&root, "a.nix", "{ }\n");
    let mut err = Vec::new();
    let code = run(
        &root,
        &[target("a.nix")],
        Flags::default(),
        &mut out,
        &mut err,
    );
    assert_eq!(code, 2);
    assert!(out.is_empty());
}
