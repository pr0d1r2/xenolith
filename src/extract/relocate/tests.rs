//! `--relocate` (`src/extract:V99`, `src/extract:V270`) over the toy
//! host of `src/extract/tests.rs`: a load `build< sh ./<path>` whose
//! site the config places at `a/build.sh` for the host `a.toy`.

use std::fs;
use std::path::{Path, PathBuf};

use super::relocate_with;
use crate::config::{self, Config};
use crate::discover::{Sandbox, write as put};
use crate::extract::{Edit, Gone, Options, Target, toys, write};
use crate::graph::{self, judge};

const SCRIPT: &str = "#!/usr/bin/env sh\nmake && make test\n";

/// A repository holding `files`, all tracked.
fn repo(sandbox: &Sandbox, files: &[(&str, &str)]) -> PathBuf {
    let root = sandbox.repo("r");
    for (rel, text) in files {
        put(&root, rel, text);
    }
    sandbox.run_git(&root, &["add", "."]);
    root
}

fn relocate(sandbox: &Sandbox, root: &Path, config: &Config, operands: &[&str]) -> Edit {
    let targets = operands
        .iter()
        .map(|op| match op.rsplit_once(':') {
            Some((path, line)) => Target {
                path: PathBuf::from(path),
                line: line.parse().ok(),
            },
            None => Target {
                path: PathBuf::from(op),
                line: None,
            },
        })
        .collect();
    let options = Options {
        targets,
        strict_hosts: false,
    };
    relocate_with(root, config, &options, &toys(), &|| sandbox.git())
        .unwrap_or_else(|e| panic!("{e}"))
}

fn refusals(edit: &Edit) -> String {
    edit.refusals
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

fn warnings(sandbox: &Sandbox, root: &Path, config: &Config) -> Vec<String> {
    let found = graph::graph_with(root, config, &graph::Options::default(), &toys(), &|| {
        sandbox.git()
    })
    .unwrap_or_else(|e| panic!("{e}"));
    assert!(found.report.violations().is_empty(), "{:?}", found.report);
    found
        .report
        .warnings()
        .iter()
        .filter(|w| w.code == judge::MISPLACED)
        .map(|w| w.message.clone())
        .collect()
}

#[test]
fn a_misplaced_extract_moves_its_load_follows_and_the_graph_is_clean() {
    let sandbox = Sandbox::new();
    let root = repo(
        &sandbox,
        &[
            ("a.toy", "build< sh ./old/build.sh\n"),
            ("old/build.sh", SCRIPT),
        ],
    );
    let config = Config::default();
    assert_eq!(warnings(&sandbox, &root, &config).len(), 1, "src/graph:V98");
    let edit = relocate(&sandbox, &root, &config, &["a.toy"]);
    assert!(edit.refusals.is_empty(), "{}", refusals(&edit));
    let [host] = edit.hosts.as_slice() else {
        panic!("{:?}", edit.hosts)
    };
    assert_eq!(host.after, "build< sh ./a/build.sh\n");
    let paths: Vec<&str> = host.extracts.iter().map(|e| e.path.as_str()).collect();
    assert_eq!(paths, ["a/build.sh"]);
    assert_eq!(
        host.removes,
        [Gone {
            path: "old/build.sh".to_owned(),
            text: SCRIPT.to_owned(),
            to: Some("a/build.sh".to_owned()),
        }]
    );
    assert_eq!(edit.exit_code(), 1);
    let diff = edit.diff();
    assert!(
        diff.starts_with("moving extract old/build.sh → a/build.sh\n"),
        "{diff}"
    );
    assert!(
        diff.contains("--- a/old/build.sh\n+++ /dev/null\n"),
        "{diff}"
    );
    let written = write::apply(&root, &edit).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        written,
        ["a/build.sh", "a.toy", "old/build.sh"],
        "src/extract:V99 order"
    );
    assert!(!root.join("old/build.sh").exists());
    assert_eq!(
        fs::read_to_string(root.join("a/build.sh")).ok().as_deref(),
        Some(SCRIPT)
    );
    sandbox.run_git(&root, &["add", "-A"]);
    assert!(warnings(&sandbox, &root, &config).is_empty(), "graph clean");
    let again = relocate(&sandbox, &root, &config, &["a.toy"]);
    assert_eq!(
        again,
        Edit {
            explain: again.explain.clone(),
            ..Edit::default()
        },
        "no-op"
    );
}

#[test]
fn a_layout_change_is_what_misplaces_it() {
    let sandbox = Sandbox::new();
    let root = repo(
        &sandbox,
        &[
            ("a.toy", "build< sh ./a/build.sh\n"),
            ("a/build.sh", SCRIPT),
        ],
    );
    let central = config::parse("version = 1\n[extract]\nlayout = \"central\"\n")
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(
        relocate(&sandbox, &root, &Config::default(), &["a.toy"])
            .hosts
            .is_empty()
    );
    let edit = relocate(&sandbox, &root, &central, &["a.toy"]);
    let [host] = edit.hosts.as_slice() else {
        panic!("{}", refusals(&edit))
    };
    assert_eq!(host.after, "build< sh ./scripts/shell/build.sh\n");
}

#[test]
fn a_shared_extract_is_refused_and_its_host_left_alone() {
    let sandbox = Sandbox::new();
    let root = repo(
        &sandbox,
        &[
            ("a.toy", "build< sh ./old.sh\n"),
            ("b.toy", "other< sh ./old.sh\n"),
            ("old.sh", SCRIPT),
        ],
    );
    let edit = relocate(&sandbox, &root, &Config::default(), &["a.toy"]);
    assert!(edit.hosts.is_empty(), "{:?}", edit.hosts);
    assert_eq!(edit.exit_code(), 2);
    let why = refusals(&edit);
    assert!(why.contains("a.toy:1: old.sh is loaded 2 time(s)"), "{why}");
    assert!(why.contains("src/extract:V270"), "{why}");
}

#[test]
fn a_move_that_would_rewrite_the_extract_is_refused() {
    // The prelude writes `#!/usr/bin/env sh`; this file says dash.
    let sandbox = Sandbox::new();
    let root = repo(
        &sandbox,
        &[
            ("a.toy", "build< sh ./old/build.sh\n"),
            ("old/build.sh", "#!/bin/dash\nmake && make test\n"),
        ],
    );
    let edit = relocate(&sandbox, &root, &Config::default(), &["a.toy"]);
    assert!(edit.hosts.is_empty(), "{:?}", edit.hosts);
    assert!(
        refusals(&edit).contains("src/extract:V270"),
        "{}",
        refusals(&edit)
    );
}

#[test]
fn a_line_names_one_load_and_a_line_with_none_is_refused() {
    let sandbox = Sandbox::new();
    let root = repo(
        &sandbox,
        &[
            ("a.toy", "build< sh ./x/build.sh\ntest< sh ./x/test.sh\n"),
            ("x/build.sh", SCRIPT),
            ("x/test.sh", SCRIPT),
        ],
    );
    let edit = relocate(&sandbox, &root, &Config::default(), &["a.toy:2"]);
    let [host] = edit.hosts.as_slice() else {
        panic!("{}", refusals(&edit))
    };
    assert_eq!(host.after, "build< sh ./x/build.sh\ntest< sh ./a/test.sh\n");
    let both = relocate(&sandbox, &root, &Config::default(), &["a.toy"]);
    let [host] = both.hosts.as_slice() else {
        panic!("{}", refusals(&both))
    };
    assert_eq!(
        host.after, "build< sh ./a/build.sh\ntest< sh ./a/test.sh\n",
        "back to front, both moves in one host"
    );
    let none = relocate(&sandbox, &root, &Config::default(), &["a.toy:3"]);
    assert!(
        refusals(&none).contains("a.toy:3: no misplaced extract"),
        "{}",
        refusals(&none)
    );
}

#[test]
fn two_moves_to_one_path_are_refused() {
    let sandbox = Sandbox::new();
    let root = repo(
        &sandbox,
        &[
            ("a.toy", "build< sh ./x.sh\nbuild< sh ./y.sh\n"),
            ("x.sh", SCRIPT),
            ("y.sh", SCRIPT),
        ],
    );
    let edit = relocate(&sandbox, &root, &Config::default(), &["a.toy"]);
    assert!(edit.hosts.is_empty(), "{:?}", edit.hosts);
    assert!(
        refusals(&edit).contains("src/extract:V47"),
        "{}",
        refusals(&edit)
    );
}
