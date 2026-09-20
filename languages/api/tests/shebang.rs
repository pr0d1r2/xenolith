//! The shebang surface language crates see: `api::shebang`.
//!
//! One surface (`languages/shebang` §G). A language crate depends on the
//! api crate and gets the shebang functions through it, rather than
//! taking a second dependency that the dependency guard would then have
//! to allow for every crate in the workspace (`languages/api:V32`).

use xenolith_lang_api::LangId;
use xenolith_lang_api::shebang::{self, Prelude, Shebang};

#[test]
fn the_shebang_functions_are_reachable_through_the_api_crate() {
    let text = "#!/usr/bin/env bash\nset -euo pipefail\necho hi\n";
    assert!(shebang::has(text));
    assert_eq!(shebang::get(text), Some("#!/usr/bin/env bash"));

    let prelude = Prelude {
        shebang: Some(Shebang::env("bash")),
        strict: Some("set -euo pipefail"),
    };
    assert_eq!(shebang::strip_strict(text, &prelude), "echo hi\n");
    assert_eq!(shebang::wrap("echo hi\n", &prelude), text);
}

#[test]
fn a_shebang_resolves_to_the_guest_that_runs_it() {
    for (line, lang) in [
        ("#!/usr/bin/env bash", LangId::Shell),
        ("#!/bin/bash", LangId::Shell),
        ("#!/bin/sh", LangId::Shell),
        ("#!/usr/bin/env zsh", LangId::Shell),
        ("#!/usr/bin/env python3", LangId::Python),
        ("#!/usr/bin/python", LangId::Python),
        ("#!/usr/bin/awk -f", LangId::Awk),
        ("#!/usr/bin/env gawk -f", LangId::Awk),
        ("#!/usr/bin/env -S jq -f", LangId::Jq),
        ("#!/usr/bin/perl", LangId::Perl),
        ("#!/usr/bin/env ruby", LangId::Ruby),
        ("#!/usr/bin/env node", LangId::Js),
    ] {
        let parsed =
            shebang::parse(line).unwrap_or_else(|| panic!("{line} should parse as a shebang"));
        assert!(
            shebang::resolves_to(&parsed, lang),
            "{line} should resolve to {lang}"
        );
    }
}

#[test]
fn a_shebang_resolves_to_exactly_one_guest() {
    let parsed = shebang::parse("#!/bin/bash").unwrap_or_else(|| unreachable!());
    for lang in LangId::ALL {
        assert_eq!(
            shebang::resolves_to(&parsed, *lang),
            *lang == LangId::Shell,
            "#!/bin/bash must resolve to shell and nothing else, but {lang} disagreed"
        );
    }
}

#[test]
fn an_unknown_interpreter_resolves_to_nothing_rather_than_a_guess() {
    let parsed = shebang::parse("#!/usr/bin/env tclsh").unwrap_or_else(|| unreachable!());
    for lang in LangId::ALL {
        assert!(
            !shebang::resolves_to(&parsed, *lang),
            "tclsh is not a language xenolith knows, but it matched {lang}"
        );
    }
}

#[test]
fn the_guest_a_shebang_names_can_be_asked_for_directly() {
    let parsed = shebang::parse("#!/usr/bin/env python3").unwrap_or_else(|| unreachable!());
    assert_eq!(shebang::guest_of(&parsed), Some(LangId::Python));

    let parsed = shebang::parse("#!/usr/bin/env tclsh").unwrap_or_else(|| unreachable!());
    assert_eq!(shebang::guest_of(&parsed), None);
}

#[test]
fn a_versioned_interpreter_still_names_its_language() {
    // `python3`, `python3.12`, `ruby3.3`: the version suffix is a fact
    // about the machine the script was written on, not about the
    // language.
    for (line, lang) in [
        ("#!/usr/bin/env python3.12", LangId::Python),
        ("#!/usr/bin/env ruby3.3", LangId::Ruby),
        ("#!/usr/bin/env perl5", LangId::Perl),
    ] {
        let parsed = shebang::parse(line).unwrap_or_else(|| unreachable!());
        assert_eq!(shebang::guest_of(&parsed), Some(lang), "{line}");
    }
}
