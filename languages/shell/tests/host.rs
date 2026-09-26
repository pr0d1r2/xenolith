//! Shell as a HOST: which files it claims (`languages/shell:T135`).
//!
//! A claim is the decision to parse a file with tree-sitter-bash, and a
//! wrong one is not an error anyone sees: a `*.bats` file parses as bash
//! and its `@test` blocks read as scripts with control flow
//! (`languages/shell:V137`). So the negative cases matter as much as the
//! positive ones (`tests:V15`), and bats is the one named by the spec.

use std::path::Path;

use xenolith_lang_api::{Host, LangId};
use xenolith_lang_shell::ShellHost;

fn claims(path: &str, head: &str) -> bool {
    ShellHost.claims(Path::new(path), head)
}

#[test]
fn the_host_is_shell() {
    assert_eq!(ShellHost.id(), LangId::Shell);
}

#[test]
fn shell_files_are_claimed_by_extension() {
    assert!(claims("scripts/guard/x.sh", ""));
    assert!(claims("x.bash", ""));
    // The extension decides, whatever the first line says.
    assert!(claims("scripts/x.sh", "set -euo pipefail"));
}

#[test]
fn envrc_is_claimed_by_name() {
    assert!(claims(".envrc", "use flake"));
    assert!(claims("sub/dir/.envrc", ""));
}

#[test]
fn a_file_is_claimed_by_a_shell_shebang_alone() {
    assert!(claims("bin/tool", "#!/usr/bin/env bash"));
    assert!(claims("bin/tool", "#!/bin/sh"));
    assert!(claims("bin/tool", "#!/usr/bin/env zsh"));
    assert!(claims("bin/tool", "#!/bin/dash -e"));
}

#[test]
fn a_bats_file_is_never_claimed() {
    // `languages/shell:V137`: the bash grammar ACCEPTS bats and gets the
    // verdict wrong, so the refusal cannot wait for a parse error.
    assert!(!claims("tests/unit/x.bats", ""));
    assert!(!claims("tests/unit/x.bats", "#!/usr/bin/env bats"));
    assert!(!claims("tests/unit/x.bats", "#!/usr/bin/env bash"));
}

#[test]
fn other_files_are_not_claimed() {
    assert!(!claims("x.nix", ""));
    assert!(!claims("x.py", "#!/usr/bin/env python3"));
    assert!(!claims("bin/tool", "#!/usr/bin/env ruby"));
    assert!(!claims("bin/tool", ""));
    assert!(!claims("bin/tool", "echo hi"));
    // A shebang names the interpreter; one that merely mentions a shell
    // in its arguments does not make the file shell.
    assert!(!claims("bin/tool", "#!/usr/bin/env nix-shell"));
    // `.sh` as a directory name, or `sh` as a bare name, is not an
    // extension.
    assert!(!claims("x.sh/readme", ""));
    assert!(!claims("sh", ""));
    assert!(!claims("x.envrc", ""));
}

#[test]
fn the_host_checks_its_own_files_with_shellcheck_and_shfmt() {
    // `languages/shell` §G: the default linters. The dialect is the
    // file's own shebang or extension, which both tools read themselves.
    let checks: Vec<String> = ShellHost
        .checks()
        .iter()
        .map(|cmd| cmd.argv.join(" "))
        .collect();
    assert_eq!(checks, ["shellcheck --format=json", "shfmt --diff"]);
    let fixers: Vec<String> = ShellHost
        .fixers()
        .iter()
        .map(|cmd| cmd.argv.join(" "))
        .collect();
    assert_eq!(fixers, ["shfmt --write"]);
}
