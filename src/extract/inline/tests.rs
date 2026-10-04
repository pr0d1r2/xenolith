//! `xnl inline` (`src/extract:V101`, `src/extract:V270`) over the toy
//! host of `src/extract/tests.rs`: `build< sh ./a/build.sh` loads the
//! extract, `build=shell: <body>` is the site it came from, and `&&`
//! makes a body non-trivial for the `sh` guest.

use std::fs;
use std::path::{Path, PathBuf};

use super::{InlineOptions, inline_with};
use crate::config::{self, Config};
use crate::discover::{Sandbox, write as put};
use crate::extract::{Edit, Gone, Options, Target, extract_with, toys, write};

fn repo(sandbox: &Sandbox, files: &[(&str, &str)]) -> PathBuf {
    let root = sandbox.repo("r");
    for (rel, text) in files {
        put(&root, rel, text);
    }
    sandbox.run_git(&root, &["add", "."]);
    root
}

fn inline(sandbox: &Sandbox, root: &Path, config: &Config, extracts: &[&str]) -> Edit {
    let options = InlineOptions {
        extracts: extracts.iter().map(PathBuf::from).collect(),
        strict_hosts: false,
    };
    inline_with(root, config, &options, &toys(), &|| sandbox.git())
        .unwrap_or_else(|e| panic!("{e}"))
}

fn refusals(edit: &Edit) -> String {
    edit.refusals
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_trivial_extract_goes_back_and_is_removed_after_its_host() {
    let sandbox = Sandbox::new();
    let text = "#!/usr/bin/env sh\necho hi\n";
    let root = repo(
        &sandbox,
        &[
            ("a.toy", "build< sh ./a/build.sh\nkeep\n"),
            ("a/build.sh", text),
        ],
    );
    let edit = inline(&sandbox, &root, &Config::default(), &["a/build.sh"]);
    assert!(edit.refusals.is_empty(), "{}", refusals(&edit));
    let [host] = edit.hosts.as_slice() else {
        panic!("{:?}", edit.hosts)
    };
    assert_eq!(host.after, "build=shell: echo hi\nkeep\n");
    assert!(host.extracts.is_empty());
    assert_eq!(
        host.removes,
        [Gone {
            path: "a/build.sh".to_owned(),
            text: text.to_owned(),
            to: None,
        }]
    );
    assert_eq!(edit.exit_code(), 1);
    assert!(
        edit.diff().starts_with("inlining a/build.sh → a.toy\n"),
        "{}",
        edit.diff()
    );
    let written = write::apply(&root, &edit).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        written,
        ["a.toy", "a/build.sh"],
        "the host first, src/extract:V101"
    );
    assert!(!root.join("a/build.sh").exists());
    // src/extract:V5: a rerun of extract leaves the trivial body inline.
    let options = Options {
        targets: vec![Target {
            path: PathBuf::from("a.toy"),
            line: None,
        }],
        strict_hosts: false,
    };
    let rerun = extract_with(&root, &Config::default(), &options, &toys(), &|| {
        sandbox.git()
    })
    .unwrap_or_else(|e| panic!("{e}"));
    assert!(rerun.hosts.is_empty(), "{:?}", rerun.hosts);
    assert_eq!(
        fs::read_to_string(root.join("a.toy")).ok().as_deref(),
        Some("build=shell: echo hi\nkeep\n")
    );
}

#[test]
fn a_body_the_threshold_relaxes_goes_back_too() {
    let sandbox = Sandbox::new();
    let root = repo(
        &sandbox,
        &[
            ("a.toy", "build< sh ./a/build.sh\n"),
            ("a/build.sh", "#!/usr/bin/env sh\nmake && make test\n"),
        ],
    );
    let strict = inline(&sandbox, &root, &Config::default(), &["a/build.sh"]);
    assert!(strict.hosts.is_empty(), "{:?}", strict.hosts);
    let why = refusals(&strict);
    assert!(
        why.contains("a/build.sh: loaded by a.toy:1: its shell body is not trivial"),
        "{why}"
    );
    assert!(why.contains("src/extract:V5"), "{why}");
    let relaxed = config::parse("version = 1\n[threshold.shell]\nallow = [\"and-or\"]\n")
        .unwrap_or_else(|e| panic!("{e}"));
    let edit = inline(&sandbox, &root, &relaxed, &["a/build.sh"]);
    assert!(edit.refusals.is_empty(), "{}", refusals(&edit));
    assert_eq!(edit.hosts.len(), 1);
}

#[test]
fn a_shared_extract_is_refused_naming_each_load() {
    let sandbox = Sandbox::new();
    let root = repo(
        &sandbox,
        &[
            ("a.toy", "build< sh ./x.sh\n"),
            ("b.toy", "keep\nother< sh ./x.sh\n"),
            ("x.sh", "#!/usr/bin/env sh\necho hi\n"),
        ],
    );
    let edit = inline(&sandbox, &root, &Config::default(), &["x.sh"]);
    assert!(edit.hosts.is_empty(), "{:?}", edit.hosts);
    assert_eq!(edit.exit_code(), 2);
    let why = refusals(&edit);
    assert!(
        why.contains("x.sh: loaded 2 times (a.toy:1, b.toy:2)"),
        "{why}"
    );
}

#[test]
fn an_extract_nothing_loads_is_refused() {
    let sandbox = Sandbox::new();
    let root = repo(&sandbox, &[("x.sh", "#!/usr/bin/env sh\necho hi\n")]);
    let edit = inline(&sandbox, &root, &Config::default(), &["x.sh"]);
    assert!(
        refusals(&edit).contains("x.sh: no host loads it"),
        "{}",
        refusals(&edit)
    );
}

#[test]
fn an_extract_that_is_not_the_exact_inverse_is_refused() {
    // A shebang the prelude would not write back: putting the body back
    // would lose it, so this inline is not the inverse of an extraction.
    let sandbox = Sandbox::new();
    let root = repo(
        &sandbox,
        &[
            ("a.toy", "build< sh ./a/build.sh\n"),
            ("a/build.sh", "#!/bin/dash\necho hi\n"),
        ],
    );
    let edit = inline(&sandbox, &root, &Config::default(), &["a/build.sh"]);
    assert!(edit.hosts.is_empty(), "{:?}", edit.hosts);
    assert!(
        refusals(&edit).contains("a/build.sh: loaded by a.toy:1:"),
        "{}",
        refusals(&edit)
    );
}

#[test]
fn two_extracts_of_one_host_go_back_together_back_to_front() {
    let sandbox = Sandbox::new();
    let root = repo(
        &sandbox,
        &[
            ("a.toy", "build< sh ./a/build.sh\ntest< sh ./a/test.sh\n"),
            ("a/build.sh", "#!/usr/bin/env sh\nmake\n"),
            ("a/test.sh", "#!/usr/bin/env sh\nmake test\n"),
        ],
    );
    let edit = inline(
        &sandbox,
        &root,
        &Config::default(),
        &["a/test.sh", "a/build.sh"],
    );
    let [host] = edit.hosts.as_slice() else {
        panic!("{}", refusals(&edit))
    };
    assert_eq!(host.after, "build=shell: make\ntest=shell: make test\n");
    let gone: Vec<&str> = host.removes.iter().map(|g| g.path.as_str()).collect();
    assert_eq!(gone, ["a/build.sh", "a/test.sh"]);
}

#[test]
fn one_refused_extract_leaves_its_host_s_others_where_they_are() {
    // `src/extract:V64`: a host is inlined whole or not at all; the
    // trivial one is refused too, saying why.
    let sandbox = Sandbox::new();
    let root = repo(
        &sandbox,
        &[
            ("a.toy", "build< sh ./a/build.sh\ntest< sh ./a/test.sh\n"),
            ("a/build.sh", "#!/usr/bin/env sh\nmake\n"),
            ("a/test.sh", "#!/usr/bin/env sh\nmake && make test\n"),
        ],
    );
    let edit = inline(
        &sandbox,
        &root,
        &Config::default(),
        &["a/build.sh", "a/test.sh"],
    );
    assert!(edit.hosts.is_empty(), "{:?}", edit.hosts);
    let said = refusals(&edit);
    assert!(said.contains("a/test.sh: loaded by a.toy:2:"), "{said}");
    assert!(said.contains("is not trivial"), "{said}");
    assert!(
        said.contains("a.toy: left untouched with its 1 other extract(s)"),
        "{said}"
    );
    assert_eq!(edit.exit_code(), 2);
}

#[test]
fn an_extract_whose_host_is_excluded_is_refused_naming_the_exclude() {
    let sandbox = Sandbox::new();
    let root = repo(
        &sandbox,
        &[
            ("a.toy", "build< sh ./a/build.sh\n"),
            ("a/build.sh", "#!/usr/bin/env sh\nmake\n"),
        ],
    );
    let excluded = config::parse(
        "version = 1\n[extract]\nexclude = [{ glob = \"a.toy\", reason = \"generated\" }]\n",
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let edit = inline(&sandbox, &root, &excluded, &["a/build.sh"]);
    assert!(edit.hosts.is_empty(), "{:?}", edit.hosts);
    let said = refusals(&edit);
    assert!(
        said.contains("a/build.sh: its host a.toy is skipped: excluded by `a.toy` (generated)"),
        "{said}"
    );
}
