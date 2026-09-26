//! Placement's helpers one by one (`languages/pkl:T54`, `src:C139`):
//! `kebab`'s word splitting, `name`'s step cut and fallback, and the
//! fixed dir.

use xenolith_lang_api::{Delim, DelimKind, GuestEnv, LangId, Site, Span};

use super::{DIR, kebab, name, placement};

#[test]
fn kebab_splits_snake_camel_and_punctuation() {
    assert_eq!(kebab("shellcheck"), "shellcheck");
    assert_eq!(kebab("cargo_clippy"), "cargo-clippy");
    assert_eq!(kebab("Rust.Fmt"), "rust-fmt");
    assert_eq!(kebab("preCommit"), "pre-commit");
    assert_eq!(kebab("x11Forwarding"), "x11-forwarding");
    assert_eq!(kebab("already-kebab"), "already-kebab");
}

#[test]
fn kebab_leaves_no_empty_word_and_no_edge_dash() {
    assert_eq!(kebab("__a  b__"), "a-b");
    assert_eq!(kebab("+++"), "");
    assert_eq!(kebab(""), "");
    assert_eq!(kebab("é"), "");
}

#[test]
fn the_name_is_the_step_key_before_the_last_dot() {
    assert_eq!(name("shellcheck.check"), "shellcheck");
    assert_eq!(name("typos.check_list_files"), "typos");
    assert_eq!(name("Rust.Fmt.fix"), "rust-fmt");
    // No property at all: the whole sink is the step.
    assert_eq!(name("lonely"), "lonely");
}

#[test]
fn a_key_naming_nothing_falls_back_to_the_host_stem() {
    assert_eq!(name("+++.check"), "{host_stem}-check");
    assert_eq!(name("+++"), "{host_stem}");
}

#[test]
fn placement_pairs_the_name_with_scripts_hk() {
    let site = Site {
        sink: "shellcheck.fix".to_owned(),
        guest: LangId::Shell,
        env: GuestEnv::default(),
        delim: Delim {
            kind: DelimKind::PklMultiline { pounds: 0 },
            open: Span::new(0, 3),
            body: Span::new(3, 3),
            close: Span::new(3, 6),
        },
        holes: Vec::new(),
    };
    let at = placement(&site);
    assert_eq!(at.name, "shellcheck");
    assert_eq!(at.dir, DIR);
    assert_eq!(DIR, "scripts/hk");
}
