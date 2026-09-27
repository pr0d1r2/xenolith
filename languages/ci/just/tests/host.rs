//! just as a host, at the trait (`languages/ci/just:T16`).
//!
//! `tests/fixtures.rs` pins which recipe is which site over files; these
//! pin the rest of the `Host` contract: the id, what is claimed
//! (`languages/ci/just:V58`), ordering (`languages/api:V36`), the shebang
//! guest flag, and rewrite at the trait.

use std::path::Path;

use xenolith_lang_api::{DelimKind, Error, Host, LangId};
use xenolith_lang_just::JustHost;

#[test]
fn the_host_is_just() {
    assert_eq!(JustHost.id(), LangId::Just);
}

#[test]
fn claims_justfiles_in_any_case_and_just_modules() {
    for path in [
        "justfile",
        "Justfile",
        "JUSTFILE",
        ".justfile",
        ".JustFile",
        "sub/dir/justfile",
        "ci/release.just",
    ] {
        assert!(JustHost.claims(Path::new(path), ""), "{path}");
    }
    for path in [
        "justfile.bak",
        "my-justfile",
        "Justfile.md",
        "notes.JUST",
        "scripts/just/build.sh",
    ] {
        assert!(!JustHost.claims(Path::new(path), ""), "{path}");
    }
}

#[test]
fn sites_come_back_sorted_by_span() {
    let src = "b:\n    x\n    y\n\na:\n    #!/bin/sh\n    z\n\nc:\n    w\n";
    let found = JustHost.sites(src).unwrap_or_else(|e| panic!("{e}"));
    let sinks: Vec<&str> = found.iter().map(|s| s.sink.as_str()).collect();
    assert_eq!(sinks, ["b", "a", "c"]);
    let starts: Vec<usize> = found.iter().map(|s| s.delim.open.start).collect();
    let mut sorted = starts.clone();
    sorted.sort_unstable();
    assert_eq!(starts, sorted);
}

#[test]
fn a_file_just_cannot_parse_is_a_parse_error() {
    // `languages:V78`: the engine applies `[parse] host_errors`.
    let got = JustHost.sites("set shell := bash\na:\n    x\n");
    assert!(
        matches!(
            got,
            Err(Error::Parse {
                lang: LangId::Just,
                ..
            })
        ),
        "{got:?}"
    );
    assert!(JustHost.loads("set shell := bash\n").is_err());
}

#[test]
fn only_a_shebang_recipe_has_its_guest_named_by_a_shebang() {
    let src = "a:\n    #!/usr/bin/env python3\n    print(1)\n\nb:\n    echo\n";
    let found = JustHost.sites(src).unwrap_or_else(|e| panic!("{e}"));
    let flags: Vec<(DelimKind, bool)> = found
        .iter()
        .map(|s| (s.delim.kind.clone(), JustHost.guest_by_shebang(src, s)))
        .collect();
    assert_eq!(
        flags,
        [
            (DelimKind::JustShebangRecipe, true),
            (DelimKind::JustRecipe, false)
        ]
    );
}

#[test]
fn rewrite_is_rewrite_bound_without_params() {
    let src = "a:\n    ls\n    pwd\n";
    let found = JustHost.sites(src).unwrap_or_else(|e| panic!("{e}"));
    let site = found.first().unwrap_or_else(|| panic!("one site"));
    let invoke = xenolith_lang_api::Invoke {
        argv: vec!["sh".to_owned(), "./x.sh".to_owned()],
    };
    let got = JustHost.rewrite(src, site, &invoke, Path::new("./x.sh"));
    assert_eq!(got.as_deref(), Ok("a:\n    sh ./x.sh\n"));
}

#[test]
fn params_are_refused_rather_than_dropped() {
    let src = "a x:\n    echo {{x}}\n    ls\n";
    let found = JustHost.sites(src).unwrap_or_else(|e| panic!("{e}"));
    let site = found.first().unwrap_or_else(|| panic!("one site"));
    let invoke = xenolith_lang_api::Invoke {
        argv: vec!["sh".to_owned(), "./x.sh".to_owned()],
    };
    let param = xenolith_lang_api::holes::Param {
        name: "X".to_owned(),
        hole: "{{x}}".to_owned(),
        marker: xenolith_lang_api::holes::marker(0),
    };
    let got = JustHost.rewrite_bound(src, site, &invoke, Path::new("./x.sh"), "", &[param]);
    assert!(matches!(got, Err(Error::Unsupported { .. })), "{got:?}");
}
