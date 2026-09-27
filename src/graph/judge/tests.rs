//! `misplaced-extract` and `inlineable-extract` (`src/graph:V98`,
//! `src/graph:V100`) through [`graph_with`], over the toy host of
//! `src/extract/tests.rs`: `<sink>< sh ./<path>` loads an extract whose
//! site the config places at `<host stem>/<sink>.sh`; `&&` makes a
//! body non-trivial for the `sh` guest.

use std::path::{Path, PathBuf};

use super::{INLINEABLE, MISPLACED};
use crate::config::{self, Config};
use crate::discover::{Sandbox, write as put};
use crate::extract::toys;
use crate::graph::{Options, graph_with};

fn repo(sandbox: &Sandbox, files: &[(&str, &str)]) -> PathBuf {
    let root = sandbox.repo("r");
    for (rel, text) in files {
        put(&root, rel, text);
    }
    sandbox.run_git(&root, &["add", "."]);
    root
}

/// `(code, file, message)` of each warning of the two codes, in order.
fn judged(
    sandbox: &Sandbox,
    root: &Path,
    config: &Config,
    paths: &[&str],
) -> Vec<(String, String, String)> {
    let options = Options {
        paths: paths.iter().map(PathBuf::from).collect(),
        ..Options::default()
    };
    let graph = graph_with(root, config, &options, &toys(), &|| sandbox.git())
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(graph.report.violations().is_empty(), "{:?}", graph.report);
    assert_eq!(graph.exit_code(), 0, "warnings never change the exit code");
    graph
        .report
        .warnings()
        .iter()
        .filter(|w| w.code == MISPLACED || w.code == INLINEABLE)
        .map(|w| {
            let file = w
                .file
                .as_ref()
                .map(|f| f.display().to_string())
                .unwrap_or_default();
            (w.code.clone(), file, w.message.clone())
        })
        .collect()
}

const SCRIPT: &str = "#!/usr/bin/env sh\nmake && make test\n";

#[test]
fn an_extract_where_its_placement_puts_it_is_not_warned_about() {
    let sandbox = Sandbox::new();
    let root = repo(
        &sandbox,
        &[
            ("a.toy", "build< sh ./a/build.sh\n"),
            ("a/build.sh", SCRIPT),
        ],
    );
    assert!(judged(&sandbox, &root, &Config::default(), &[]).is_empty());
}

#[test]
fn a_layout_change_warns_misplaced_naming_the_path_and_the_relocate() {
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
    let found = judged(&sandbox, &root, &central, &[]);
    let [(code, file, message)] = found.as_slice() else {
        panic!("{found:?}")
    };
    assert_eq!(code, "misplaced-extract");
    assert_eq!(file, "a/build.sh");
    assert!(
        message.contains("puts it at scripts/shell/build.sh"),
        "{message}"
    );
    assert!(
        message.contains("`xnl extract --relocate a.toy:1`"),
        "{message}"
    );
    // A named-path run judges the loads it reads, too.
    assert_eq!(judged(&sandbox, &root, &central, &["a.toy"]).len(), 1);
}

#[test]
fn a_trivial_body_warns_inlineable_and_a_shared_one_does_not() {
    let sandbox = Sandbox::new();
    let root = repo(
        &sandbox,
        &[
            ("a.toy", "build< sh ./a/build.sh\n"),
            ("a/build.sh", "#!/usr/bin/env sh\nmake\n"),
        ],
    );
    let found = judged(&sandbox, &root, &Config::default(), &[]);
    let [(code, file, message)] = found.as_slice() else {
        panic!("{found:?}")
    };
    assert_eq!(code, "inlineable-extract");
    assert_eq!(file, "a/build.sh");
    assert!(message.contains("`xnl inline a/build.sh`"), "{message}");
    put(&root, "b.toy", "build< sh ./a/build.sh\n");
    sandbox.run_git(&root, &["add", "."]);
    let shared = judged(&sandbox, &root, &Config::default(), &[]);
    assert!(
        shared
            .iter()
            .all(|(code, _, _)| code != "inlineable-extract"),
        "inline refuses a shared extract: {shared:?}"
    );
}

#[test]
fn a_threshold_that_relaxes_the_body_makes_it_inlineable() {
    let sandbox = Sandbox::new();
    let root = repo(
        &sandbox,
        &[
            ("a.toy", "build< sh ./a/build.sh\n"),
            ("a/build.sh", SCRIPT),
        ],
    );
    assert!(judged(&sandbox, &root, &Config::default(), &[]).is_empty());
    let relaxed = config::parse("version = 1\n[threshold.shell]\nallow = [\"and-or\"]\n")
        .unwrap_or_else(|e| panic!("{e}"));
    let found = judged(&sandbox, &root, &relaxed, &[]);
    assert_eq!(found.len(), 1, "{found:?}");
}

#[test]
fn a_load_that_cannot_be_read_back_is_not_judged() {
    // The toy `sticky` host's inline deletes the load's whole line, text
    // before the load included: no site can be found back, so nothing
    // is said about it.
    let sandbox = Sandbox::new();
    let root = repo(
        &sandbox,
        &[
            ("a.sticky", "x\nbuild< sh ./elsewhere.sh\n"),
            ("elsewhere.sh", "#!/usr/bin/env sh\nmake\n"),
        ],
    );
    let found = judged(&sandbox, &root, &Config::default(), &[]);
    assert!(found.is_empty(), "{found:?}");
}
