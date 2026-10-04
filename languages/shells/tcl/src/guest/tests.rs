//! Unit tests for `guest.rs` (`src:C139`): the defaults of
//! `languages/shells/tcl:V197`, per dialect.

use std::path::Path;

use xenolith_lang_api::{Error, Guest, GuestEnv, LangId, Shebang};

use super::{TclGuest, is_expect};

fn expect() -> GuestEnv {
    GuestEnv {
        dialect: Some("expect".to_owned()),
        options: Vec::new(),
    }
}

#[test]
fn the_guest_is_tcl() {
    assert_eq!(TclGuest.id(), LangId::Tcl);
}

#[test]
fn only_the_expect_dialect_is_expect() {
    assert!(is_expect(&expect()));
    assert!(!is_expect(&GuestEnv::default()));
    let other = GuestEnv {
        dialect: Some("tcl".to_owned()),
        options: Vec::new(),
    };
    assert!(!is_expect(&other));
}

#[test]
fn the_extension_follows_the_dialect() {
    assert_eq!(TclGuest.extension(&GuestEnv::default()), "tcl");
    assert_eq!(TclGuest.extension(&expect()), "exp");
}

#[test]
fn invoke_runs_an_exp_file_with_expect_and_the_rest_with_tclsh() {
    assert_eq!(
        TclGuest.invoke(Path::new("x/a.tcl")).argv,
        ["tclsh", "x/a.tcl"]
    );
    assert_eq!(
        TclGuest.invoke(Path::new("x/a.exp")).argv,
        ["expect", "x/a.exp"]
    );
    assert_eq!(TclGuest.invoke(Path::new("a")).argv, ["tclsh", "a"]);
}

#[test]
fn the_prelude_is_a_shebang_and_nothing_strict() {
    let plain = TclGuest.prelude(&GuestEnv::default());
    assert_eq!(plain.shebang, Some(Shebang::env("tclsh")));
    assert_eq!(plain.strict, None);
    let expect = TclGuest.prelude(&expect());
    assert_eq!(expect.shebang, Some(Shebang::env("expect")));
    assert_eq!(expect.strict, None);
    assert!(TclGuest.executable());
}

#[test]
fn a_body_the_grammar_reads_is_left_to_the_size_threshold() {
    assert_eq!(TclGuest.trivial("puts hi\n"), Ok(false));
    assert_eq!(TclGuest.trivial(""), Ok(false));
    // No construct vocabulary: the engine falls back to max_lines /
    // max_bytes (`languages/api:V37`).
    assert_eq!(
        TclGuest.constructs("puts hi\n"),
        Err(Error::unsupported(LangId::Tcl, "constructs"))
    );
}

#[test]
fn a_body_the_grammar_rejects_is_a_parse_error() {
    let err = TclGuest.trivial("puts {unclosed\n");
    assert!(
        matches!(
            err,
            Err(Error::Parse {
                lang: LangId::Tcl,
                ..
            })
        ),
        "{err:?}"
    );
}

#[test]
fn the_guest_check_is_the_syntax_check_in_either_dialect() {
    // `languages/shells/tcl:V198`: an extract is tcl text like a host file.
    let check = vec![crate::syntax::lint_cmd()];
    assert_eq!(TclGuest.checks(&GuestEnv::default()), check);
    assert_eq!(TclGuest.checks(&expect()), check);
    assert!(TclGuest.fixers(&expect()).is_empty());
}
