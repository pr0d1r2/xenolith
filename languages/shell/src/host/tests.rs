//! Unit tests for `host.rs` (`src:C139`): the claim, part by part.

use std::path::Path;

use xenolith_lang_api::{
    Delim, DelimKind, Error, Format, GuestEnv, Host, Invoke, LangId, LoadRef, Site, Span,
};

use super::{BATS, EXTENSIONS, FILENAMES, ShellHost, raw};

fn claims(path: &str, head: &str) -> bool {
    ShellHost.claims(Path::new(path), head)
}

#[test]
fn every_listed_extension_and_filename_is_claimed() {
    for ext in EXTENSIONS {
        assert!(claims(&format!("dir/x.{ext}"), ""), "{ext}");
    }
    for name in FILENAMES {
        assert!(claims(&format!("dir/{name}"), ""), "{name}");
    }
}

#[test]
fn bats_is_refused_before_any_shebang_is_read() {
    assert!(!claims(&format!("x.{BATS}"), "#!/bin/bash"));
    assert!(!claims(&format!("x.{BATS}"), "#!/usr/bin/env sh"));
    // bats is not in the extension list either, so the refusal is not
    // an accident of ordering.
    assert!(!EXTENSIONS.contains(&BATS));
}

#[test]
fn every_shell_dialect_shebang_is_claimed() {
    for dialect in ["sh", "bash", "zsh", "dash", "ksh", "ash"] {
        assert!(
            claims("tool", &format!("#!/usr/bin/env {dialect}")),
            "{dialect}"
        );
        assert!(claims("tool", &format!("#!/bin/{dialect}")), "{dialect}");
    }
}

#[test]
fn a_foreign_or_missing_shebang_claims_nothing() {
    assert!(!claims("tool", "#!/usr/bin/env python3"));
    assert!(!claims("tool", "#!/usr/bin/env awk -f"));
    assert!(!claims("tool", "#!"));
    assert!(!claims("tool", "# !/bin/sh"));
    assert!(!claims("tool", ""));
}

#[test]
fn a_dotted_name_that_is_not_the_extension_is_not_claimed() {
    assert!(!claims("x.sh.txt", ""));
    assert!(!claims("x.envrc", ""));
    assert!(!claims(".envrc.local", ""));
}

#[test]
fn unoffered_operations_are_refused_by_name() {
    let refused = |operation: &'static str| Some(Error::unsupported(LangId::Shell, operation));
    let site = Site {
        sink: String::new(),
        guest: LangId::Python,
        env: GuestEnv::default(),
        delim: Delim {
            kind: DelimKind::ArgvString,
            open: Span::new(0, 1),
            body: Span::new(1, 1),
            close: Span::new(1, 2),
        },
        holes: Vec::new(),
    };
    let load = LoadRef {
        span: Span::new(0, 0),
        path: "x.py".into(),
        guest: LangId::Python,
    };
    let invoke = Invoke { argv: Vec::new() };
    assert_eq!(ShellHost.loads("").err(), refused("loads"));
    assert_eq!(ShellHost.inline("", &load, "").err(), refused("inline"));
    let rewritten = ShellHost.rewrite("", &site, &invoke, Path::new("x"));
    assert_eq!(rewritten.err(), refused("rewrite"));
}

#[test]
fn raw_appends_the_file_and_reads_no_output() {
    let cmd = raw(&["shfmt", "--diff"]);
    assert_eq!(cmd.argv, ["shfmt", "--diff"]);
    assert_eq!(cmd.format, Format::Raw);
}
