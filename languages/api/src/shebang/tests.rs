//! Unit tests for `shebang` (`src:C139`): the interpreter table and the
//! two private helpers that normalise a name before it is looked up.
//!
//! `tests/shebang.rs` checks the surface a language crate sees; these
//! check `basename` and `strip_version` directly, and the table's own
//! property -- no interpreter answers to two languages.

use super::{INTERPRETERS, basename, guest_of, resolves_to, strip_version};
use crate::LangId;
use crate::shebang::{Shebang, parse};

fn guest(line: &str) -> Option<LangId> {
    let shebang = parse(line).unwrap_or_else(|| panic!("expected a shebang in {line:?}"));
    guest_of(&shebang)
}

// ---------------------------------------------------------------------
// basename
// ---------------------------------------------------------------------

#[test]
fn basename_is_the_last_path_segment() {
    assert_eq!(basename("/usr/bin/python3"), "python3");
    assert_eq!(basename("/bin/sh"), "sh");
    assert_eq!(basename("./local/jq"), "jq");
}

#[test]
fn basename_of_a_bare_name_is_the_name() {
    assert_eq!(basename("bash"), "bash");
    assert_eq!(basename(""), "");
}

#[test]
fn basename_of_a_trailing_slash_is_empty() {
    // A directory is no interpreter, and "" matches nothing in the table.
    assert_eq!(basename("/usr/bin/"), "");
}

// ---------------------------------------------------------------------
// strip_version
// ---------------------------------------------------------------------

#[test]
fn strip_version_drops_a_trailing_version() {
    assert_eq!(strip_version("python3"), "python");
    assert_eq!(strip_version("python3.12"), "python");
    assert_eq!(strip_version("ruby3.3"), "ruby");
    assert_eq!(strip_version("ksh93"), "ksh");
}

#[test]
fn strip_version_leaves_an_unversioned_name_alone() {
    assert_eq!(strip_version("bash"), "bash");
    assert_eq!(strip_version("busybox-awk"), "busybox-awk");
}

#[test]
fn strip_version_keeps_digits_that_are_not_a_suffix() {
    assert_eq!(strip_version("2to3x"), "2to3x");
}

#[test]
fn strip_version_of_an_all_digit_name_is_the_name() {
    // Stripping everything would leave "", which is no name at all.
    assert_eq!(strip_version("3"), "3");
    assert_eq!(strip_version("3.12"), "3.12");
    assert_eq!(strip_version(""), "");
}

// ---------------------------------------------------------------------
// the table
// ---------------------------------------------------------------------

#[test]
fn no_interpreter_answers_to_two_languages() {
    let mut seen: Vec<&str> = Vec::new();
    for (_, names) in INTERPRETERS {
        for name in *names {
            assert!(!seen.contains(name), "{name} listed twice");
            seen.push(name);
        }
    }
}

#[test]
fn no_language_is_listed_twice() {
    let mut langs: Vec<LangId> = INTERPRETERS.iter().map(|(lang, _)| *lang).collect();
    let count = langs.len();
    langs.sort();
    langs.dedup();
    assert_eq!(langs.len(), count);
}

#[test]
fn every_table_name_is_already_normalised() {
    // A name `strip_version` would change can never be looked up, so it
    // would be a dead row: `python3` in the table would never match.
    for (_, names) in INTERPRETERS {
        for name in *names {
            assert_eq!(strip_version(basename(name)), *name, "{name}");
        }
    }
}

#[test]
fn every_table_name_resolves_to_its_language() {
    for (lang, names) in INTERPRETERS {
        for name in *names {
            assert_eq!(guest_of(&Shebang::env(name)), Some(*lang), "{name}");
            let absolute = format!("/usr/bin/{name}");
            assert_eq!(
                guest_of(&Shebang::absolute(&absolute)),
                Some(*lang),
                "{name}"
            );
        }
    }
}

// ---------------------------------------------------------------------
// guest_of / resolves_to
// ---------------------------------------------------------------------

#[test]
fn guest_of_normalises_path_and_version_together() {
    assert_eq!(
        guest("#!/usr/local/bin/python3.12 -u"),
        Some(LangId::Python)
    );
    assert_eq!(guest("#!/usr/bin/env -S jq -f"), Some(LangId::Jq));
}

#[test]
fn guest_of_an_unknown_interpreter_is_none() {
    for line in ["#!/usr/bin/tclsh", "#!/usr/bin/env fish", "#!/bin/csh"] {
        assert_eq!(guest(line), None, "{line}");
    }
}

#[test]
fn guest_of_a_bare_env_is_none() {
    // Resolves to `env` itself, which runs nothing on its own.
    assert_eq!(guest_of(&Shebang::absolute("/usr/bin/env")), None);
    assert_eq!(guest("#!/usr/bin/env -S"), None);
}

#[test]
fn resolves_to_is_true_only_for_the_named_language() {
    let bash = Shebang::env("bash");
    assert!(resolves_to(&bash, LangId::Shell));
    assert!(!resolves_to(&bash, LangId::Python));
    let unknown = Shebang::env("tclsh");
    for &lang in LangId::ALL {
        assert!(!resolves_to(&unknown, lang), "{lang}");
    }
}
