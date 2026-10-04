//! Tcl as a HOST: which files it claims and in which dialect
//! (`languages/shells/tcl:V195`), and what a fixture line cannot say --
//! a file that does not parse, the unescaped body, the sort order.
//!
//! Its SITES (`languages/shells/tcl:V196`) are pinned case by case in
//! `tests/fixtures/`.

use std::path::Path;

use xenolith_lang_api::{Error, Host, LangId, Site};
use xenolith_lang_tcl::{TclHost, dialect};

fn claims(path: &str, head: &str) -> bool {
    TclHost.claims(Path::new(path), head)
}

fn sites(src: &str) -> Vec<Site> {
    TclHost
        .sites(src)
        .unwrap_or_else(|e| panic!("sites failed: {e}"))
}

#[test]
fn the_host_is_tcl() {
    assert_eq!(TclHost.id(), LangId::Tcl);
}

#[test]
fn tcl_files_are_claimed_by_extension() {
    assert!(claims("lib/x.tcl", ""));
    assert!(claims("ui/x.tk", ""));
    assert!(claims("bin/login.exp", ""));
    // The extension decides, whatever the first line says.
    assert!(claims("x.tcl", "puts hi"));
}

#[test]
fn a_file_is_claimed_by_a_tcl_shebang_alone() {
    assert!(claims("bin/tool", "#!/usr/bin/env tclsh"));
    assert!(claims("bin/tool", "#!/usr/bin/tclsh8.6"));
    assert!(claims("bin/tool", "#!/usr/bin/env wish"));
    assert!(claims("bin/tool", "#!/usr/bin/expect -f"));
}

#[test]
fn other_files_are_not_claimed() {
    assert!(!claims("x.sh", "#!/usr/bin/env bash"));
    assert!(!claims("bin/tool", "#!/usr/bin/env python3"));
    assert!(!claims("notes.txt", "exec sh -c {a | b}"));
    assert!(!claims("x.tcl.txt", ""));
}

#[test]
fn an_exp_file_is_the_expect_dialect() {
    // T199: `.exp` → dialect expect, as is an expect shebang; plain tcl
    // names no dialect.
    assert_eq!(dialect(Path::new("bin/login.exp"), ""), Some("expect"));
    assert_eq!(
        dialect(Path::new("bin/login"), "#!/usr/bin/env expect"),
        Some("expect")
    );
    assert_eq!(
        dialect(Path::new("lib/x.tcl"), "#!/usr/bin/env tclsh"),
        None
    );
}

#[test]
fn a_file_with_an_error_outside_braces_fails_whole() {
    // `languages:V78`: spans after an `ERROR` node are the parser's guess.
    // `set x a(b)` is valid Tcl the vendored grammar still rejects, even
    // patched (`languages/shells/tcl:B1`); the file is then a host parse
    // error, never a partial list of sites.
    let src = "set x a(b)\nexec sh -c {a | b}\n";
    let err = TclHost.sites(src);
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
fn an_unclosed_brace_fails_whole() {
    let err = TclHost.sites("exec sh -c {a | b\n");
    assert!(matches!(err, Err(Error::Parse { .. })), "{err:?}");
}

#[test]
fn errors_inside_a_data_argument_do_not_fail_the_file() {
    // expect's pattern-action list is a braced ARGUMENT: data to this
    // walk, however the grammar reads it.
    let src = "expect {\n  \"yes/no\" { send \"yes\\r\"; exp_continue }\n}\n\
               exec sh -c {a | b}\n";
    let found = sites(src);
    assert_eq!(found.len(), 1, "{found:#?}");
}

#[test]
fn sites_come_back_sorted_by_span() {
    let src = "exec sh -c {a | b}\nset x [exec bash -c {c | d}]\nexec sh -c {e | f}\n";
    let found = sites(src);
    let opens: Vec<usize> = found.iter().map(|s| s.delim.open.start).collect();
    let mut sorted = opens.clone();
    sorted.sort_unstable();
    assert_eq!(opens, sorted);
    assert_eq!(found.len(), 3);
}

#[test]
fn the_body_unescapes_to_itself() {
    let src = "exec sh -c {printf '%s\\n' \"$x\" | tr a b}\n";
    let site = sites(src)
        .into_iter()
        .next()
        .unwrap_or_else(|| unreachable!());
    let raw = site.delim.body.of(src).unwrap_or_default();
    assert_eq!(
        TclHost.unescape(&site.delim, raw),
        Ok("printf '%s\\n' \"$x\" | tr a b".to_owned())
    );
}

#[test]
fn an_empty_file_has_no_sites() {
    assert!(sites("").is_empty());
}
