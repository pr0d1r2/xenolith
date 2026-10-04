//! The `xnl` argument parser: the mirror of `src/cli/args.rs`
//! (`src:C139`).
//!
//! The parser is tested apart from the dispatch because it fails apart
//! from it: a flag read wrongly is a run that did something the user did
//! not ask for, even once every verb has an engine behind it. Every
//! refusal here must NAME what it refused -- a usage error that says
//! only "bad arguments" sends the user to read the whole usage block to
//! find the one word they mistyped.

use std::ffi::OsString;
use std::path::PathBuf;

use super::{Invocation, OutputFormat, Scan, Target, Usage, Verb, parse};

fn ok(args: &[&str]) -> Invocation {
    match parse(args) {
        Ok(invocation) => invocation,
        Err(usage) => panic!("expected {args:?} to parse, got: {}", usage.0),
    }
}

fn usage(args: &[&str]) -> String {
    match parse(args) {
        Ok(invocation) => panic!("expected {args:?} to be refused, got: {invocation:?}"),
        Err(Usage(message)) => message,
    }
}

fn paths(items: &[&str]) -> Vec<PathBuf> {
    items.iter().map(PathBuf::from).collect()
}

fn scan(format: OutputFormat, items: &[&str]) -> Scan {
    Scan {
        format,
        paths: paths(items),
    }
}

// ---------------------------------------------------------------------
// the first word
// ---------------------------------------------------------------------

#[test]
fn no_arguments_is_a_usage_error() {
    // Not a default verb: `xnl` alone doing a check would make a typo'd
    // flag before the verb a silent scan.
    assert!(!usage(&[]).is_empty());
}

#[test]
fn version_long_and_short_are_the_version_verb() {
    assert_eq!(ok(&["--version"]).verb, Verb::Version);
    assert_eq!(ok(&["-V"]).verb, Verb::Version);
}

#[test]
fn version_ignores_what_follows_it() {
    assert_eq!(
        ok(&["--version", "check", "--nonsense"]).verb,
        Verb::Version
    );
}

#[test]
fn an_unknown_first_word_is_named() {
    for word in ["frobnicate", "-v", "--help", "-", "", "Check"] {
        let message = usage(&[word]);
        assert!(
            message.contains(&format!("`{word}`")),
            "{word:?}: {message}"
        );
    }
}

#[test]
fn a_flag_before_the_verb_is_refused() {
    // Flags follow the verb, always: one place to look for them.
    let message = usage(&["--verbose", "check"]);
    assert!(message.contains("`--verbose`"), "{message}");
}

// ---------------------------------------------------------------------
// check and graph: --format, paths
// ---------------------------------------------------------------------

#[test]
fn check_with_nothing_is_human_over_no_paths() {
    // No paths is not "nothing to do": it is `git ls-files` (`src/discover:V57`),
    // which the engine decides, so the parser keeps the list empty.
    let got = ok(&["check"]);
    assert_eq!(got.verb, Verb::Check(scan(OutputFormat::Human, &[])));
    assert!(!got.verbose);
    assert!(!got.strict_hosts);
}

#[test]
fn check_keeps_paths_in_the_order_given() {
    // Order of OUTPUT is the engine's to sort (`src:V11`); the parser
    // does not reorder input behind anybody's back.
    let got = ok(&["check", "b.nix", "a.nix"]);
    assert_eq!(
        got.verb,
        Verb::Check(scan(OutputFormat::Human, &["b.nix", "a.nix"]))
    );
}

#[test]
fn format_takes_a_separate_or_an_attached_value() {
    for args in [
        &["check", "--format", "json", "x.nix"][..],
        &["check", "--format=json", "x.nix"][..],
        &["check", "x.nix", "--format", "json"][..],
    ] {
        assert_eq!(
            ok(args).verb,
            Verb::Check(scan(OutputFormat::Json, &["x.nix"])),
            "{args:?}"
        );
    }
}

#[test]
fn every_format_name_parses_for_check_and_graph() {
    for (name, format) in [
        ("human", OutputFormat::Human),
        ("json", OutputFormat::Json),
        ("sarif", OutputFormat::Sarif),
    ] {
        assert_eq!(
            ok(&["check", "--format", name]).verb,
            Verb::Check(scan(format, &[]))
        );
        assert_eq!(
            ok(&["graph", "--format", name]).verb,
            Verb::Graph(scan(format, &[]))
        );
    }
}

#[test]
fn the_last_format_wins() {
    assert_eq!(
        ok(&["check", "--format", "json", "--format", "human"]).verb,
        Verb::Check(scan(OutputFormat::Human, &[]))
    );
}

#[test]
fn an_unknown_format_is_named_with_the_choices() {
    let message = usage(&["check", "--format", "xml"]);
    assert!(message.contains("`xml`"), "{message}");
    assert!(message.contains("human"), "{message}");
    assert!(message.contains("json"), "{message}");
}

#[test]
fn format_without_a_value_is_refused() {
    let message = usage(&["check", "--format"]);
    assert!(message.contains("--format"), "{message}");
    let message = usage(&["check", "--format="]);
    assert!(message.contains("--format"), "{message}");
}

#[test]
fn format_does_not_swallow_a_following_flag_as_its_value() {
    let message = usage(&["check", "--format", "--verbose"]);
    assert!(message.contains("--format"), "{message}");
}

#[test]
fn format_names_are_exact() {
    // As `LangId::from_name`: a near-miss is a mistake worth naming.
    assert!(usage(&["check", "--format", "JSON"]).contains("`JSON`"));
}

// ---------------------------------------------------------------------
// flags every verb takes
// ---------------------------------------------------------------------

#[test]
fn verbose_and_strict_hosts_are_accepted_by_every_verb() {
    for verb in ["check", "graph", "lint", "langs", "migrate"] {
        let got = ok(&[verb, "--verbose", "--strict-hosts"]);
        assert!(got.verbose, "{verb}");
        assert!(got.strict_hosts, "{verb}");
    }
    for verb in ["extract", "inline"] {
        let got = ok(&[verb, "--verbose", "--strict-hosts", "x.nix"]);
        assert!(got.verbose, "{verb}");
        assert!(got.strict_hosts, "{verb}");
    }
}

#[test]
fn a_repeated_boolean_flag_is_harmless() {
    assert!(ok(&["check", "--verbose", "--verbose"]).verbose);
}

#[test]
fn an_unknown_flag_is_named_with_its_verb() {
    let message = usage(&["check", "--frobnicate"]);
    assert!(message.contains("`--frobnicate`"), "{message}");
    assert!(message.contains("check"), "{message}");
}

#[test]
fn a_flag_of_another_verb_is_refused() {
    // `--write` on `check` is a user expecting a write that would never
    // happen; `--fix` on `graph` likewise.
    for args in [
        &["check", "--write"][..],
        &["graph", "--fix"][..],
        &["langs", "--relocate"][..],
        &["extract", "--format", "json", "x.nix"][..],
        &["check", "--trust-config"][..],
    ] {
        let message = usage(args);
        let flag = args.get(1).copied().unwrap_or_default();
        assert!(
            message.contains(&format!("`{flag}`")),
            "{args:?}: {message}"
        );
    }
}

#[test]
fn a_boolean_flag_refuses_an_attached_value() {
    let message = usage(&["check", "--verbose=yes"]);
    assert!(message.contains("--verbose"), "{message}");
}

#[test]
fn a_bundle_of_short_flags_is_an_unknown_flag() {
    assert!(usage(&["check", "-vx"]).contains("`-vx`"));
}

// ---------------------------------------------------------------------
// paths: `--` and odd names
// ---------------------------------------------------------------------

#[test]
fn double_dash_ends_the_flags() {
    let got = ok(&["check", "--", "--verbose", "-x.nix"]);
    assert!(!got.verbose);
    assert_eq!(
        got.verb,
        Verb::Check(scan(OutputFormat::Human, &["--verbose", "-x.nix"]))
    );
}

#[test]
fn a_lone_dash_is_a_path() {
    assert_eq!(
        ok(&["check", "-"]).verb,
        Verb::Check(scan(OutputFormat::Human, &["-"]))
    );
}

#[cfg(unix)]
#[test]
fn a_path_that_is_not_utf8_is_kept_as_bytes() {
    // hk hands over whatever filenames the tree holds; one that is not
    // UTF-8 is still a file to check, not a reason to panic
    // (`std::env::args` would).
    use std::os::unix::ffi::OsStringExt;

    let odd = OsString::from_vec(vec![b'a', 0xff, b'.', b'n', b'i', b'x']);
    let args = [OsString::from("check"), odd.clone()];
    match parse(&args) {
        Ok(got) => assert_eq!(
            got.verb,
            Verb::Check(Scan {
                format: OutputFormat::Human,
                paths: vec![PathBuf::from(odd)],
            })
        ),
        Err(Usage(message)) => panic!("refused: {message}"),
    }
}

#[cfg(unix)]
#[test]
fn a_flag_that_is_not_utf8_is_refused_not_a_panic() {
    use std::os::unix::ffi::OsStringExt;

    let odd = OsString::from_vec(vec![b'-', b'-', 0xff]);
    let args = [OsString::from("check"), odd];
    assert!(parse(&args).is_err());
}

// ---------------------------------------------------------------------
// extract
// ---------------------------------------------------------------------

fn target(path: &str, line: Option<usize>) -> Target {
    Target {
        path: PathBuf::from(path),
        line,
    }
}

#[test]
fn extract_needs_at_least_one_path() {
    // `src/cli` §I: `<path>[:line]…`, one or more. An extract over "the
    // whole tree" is a rewrite of the whole tree, which is too big a
    // thing to happen by forgetting an argument.
    let message = usage(&["extract"]);
    assert!(message.contains("extract"), "{message}");
    assert!(message.contains("path"), "{message}");
}

#[test]
fn extract_defaults_to_the_diff_without_relocating() {
    assert_eq!(
        ok(&["extract", "a.nix"]).verb,
        Verb::Extract {
            write: false,
            relocate: false,
            targets: vec![target("a.nix", None)],
        }
    );
}

#[test]
fn extract_takes_write_and_relocate() {
    assert_eq!(
        ok(&["extract", "--write", "--relocate", "a.nix"]).verb,
        Verb::Extract {
            write: true,
            relocate: true,
            targets: vec![target("a.nix", None)],
        }
    );
}

#[test]
fn inline_takes_extracts_and_write() {
    // `src/cli` §I: `xnl inline [--write] <extract>…`; an operand is a
    // file, never `<path>:<line>` -- an extract has one load to go back to.
    assert_eq!(
        ok(&["inline", "a/x.sh", "b:1"]).verb,
        Verb::Inline {
            write: false,
            extracts: vec![PathBuf::from("a/x.sh"), PathBuf::from("b:1")],
        }
    );
    let invocation = ok(&["inline", "--write", "--verbose", "--strict-hosts", "a/x.sh"]);
    assert_eq!(
        invocation.verb,
        Verb::Inline {
            write: true,
            extracts: vec![PathBuf::from("a/x.sh")],
        }
    );
    assert!(invocation.verbose && invocation.strict_hosts);
    assert_eq!(invocation.verb.name(), "inline");
}

#[test]
fn inline_needs_an_extract_and_takes_no_other_verb_s_flag() {
    let message = usage(&["inline"]);
    assert!(
        message.contains("inline") && message.contains("extract"),
        "{message}"
    );
    for flag in ["--relocate", "--format", "--fix"] {
        let message = usage(&["inline", flag, "a/x.sh"]);
        assert!(message.contains(flag), "{flag}: {message}");
    }
}

#[test]
fn extract_reads_a_trailing_line_number() {
    assert_eq!(
        ok(&["extract", "a.nix:12", "b.pkl"]).verb,
        Verb::Extract {
            write: false,
            relocate: false,
            targets: vec![target("a.nix", Some(12)), target("b.pkl", None)],
        }
    );
}

#[test]
fn only_an_all_digit_suffix_is_a_line() {
    // A colon is a legal filename character; `a:b.nix` is a file, and so
    // is `c:` -- only `:<digits>` at the very end is a line.
    let Verb::Extract { targets, .. } = ok(&["extract", "a:b.nix", "c:", "d:1x"]).verb else {
        panic!("not extract");
    };
    assert_eq!(
        targets,
        vec![
            target("a:b.nix", None),
            target("c:", None),
            target("d:1x", None),
        ]
    );
}

#[test]
fn the_line_is_split_at_the_last_colon() {
    let Verb::Extract { targets, .. } = ok(&["extract", "a:b.nix:3"]).verb else {
        panic!("not extract");
    };
    assert_eq!(targets, vec![target("a:b.nix", Some(3))]);
}

#[test]
fn line_zero_is_refused() {
    // Lines count from 1 (`src/cli` §I human output); 0 names no line,
    // and treating it as "all sites" would widen a narrow request.
    let message = usage(&["extract", "a.nix:0"]);
    assert!(message.contains("a.nix:0"), "{message}");
}

#[test]
fn a_line_too_large_to_count_is_refused() {
    let message = usage(&["extract", "a.nix:99999999999999999999999"]);
    assert!(message.contains("a.nix:"), "{message}");
}

#[test]
fn a_bare_line_with_no_path_is_refused() {
    let message = usage(&["extract", ":12"]);
    assert!(message.contains(":12"), "{message}");
}

// ---------------------------------------------------------------------
// lint
// ---------------------------------------------------------------------

#[test]
fn lint_defaults() {
    assert_eq!(
        ok(&["lint"]).verb,
        Verb::Lint {
            fix: false,
            trust_config: false,
            sites: false,
            scan: scan(OutputFormat::Human, &[]),
        }
    );
}

#[test]
fn lint_takes_fix_trust_config_and_format() {
    assert_eq!(
        ok(&["lint", "--fix", "--trust-config", "--format=sarif", "x.sh"]).verb,
        Verb::Lint {
            fix: true,
            trust_config: true,
            sites: false,
            scan: scan(OutputFormat::Sarif, &["x.sh"]),
        }
    );
}

#[test]
fn lint_takes_sites() {
    // `src/lint:V93`: lint the sites in place, before extraction.
    assert_eq!(
        ok(&["lint", "--sites", "a.nix"]).verb,
        Verb::Lint {
            fix: false,
            trust_config: false,
            sites: true,
            scan: scan(OutputFormat::Human, &["a.nix"]),
        }
    );
    assert!(usage(&["check", "--sites"]).contains("`--sites`"));
}

// ---------------------------------------------------------------------
// langs
// ---------------------------------------------------------------------

#[test]
fn langs_takes_human_and_json() {
    assert_eq!(
        ok(&["langs"]).verb,
        Verb::Langs {
            format: OutputFormat::Human
        }
    );
    assert_eq!(
        ok(&["langs", "--format", "json"]).verb,
        Verb::Langs {
            format: OutputFormat::Json
        }
    );
}

#[test]
fn langs_has_no_sarif() {
    // `src/cli` §I gives `langs` human|json only: a language list has no
    // findings for SARIF to carry, so the name is refused as a usage
    // error rather than deferred to `src/cli:T103`.
    let message = usage(&["langs", "--format", "sarif"]);
    assert!(message.contains("`sarif`"), "{message}");
    assert!(message.contains("langs"), "{message}");
}

#[test]
fn langs_takes_no_paths() {
    let message = usage(&["langs", "x.nix"]);
    assert!(message.contains("`x.nix`"), "{message}");
}

// ---------------------------------------------------------------------
// migrate (`src/cli:T97`)
// ---------------------------------------------------------------------

#[test]
fn migrate_defaults_to_the_diff() {
    assert_eq!(ok(&["migrate"]).verb, Verb::Migrate { write: false });
    assert_eq!(Verb::Migrate { write: false }.name(), "migrate");
}

#[test]
fn migrate_takes_write_verbose_and_strict_hosts() {
    let got = ok(&["migrate", "--write", "--verbose", "--strict-hosts"]);
    assert_eq!(got.verb, Verb::Migrate { write: true });
    assert!(got.verbose);
    assert!(got.strict_hosts);
}

#[test]
fn migrate_takes_no_paths_and_no_format() {
    // The legacy lists sit at the root under fixed names (`src/cli` §I):
    // a path operand would suggest a choice the verb does not offer.
    let message = usage(&["migrate", "x.nix"]);
    assert!(message.contains("`x.nix`"), "{message}");
    assert!(message.contains("migrate"), "{message}");
    let message = usage(&["migrate", "--format", "json"]);
    assert!(message.contains("`--format`"), "{message}");
}

// ---------------------------------------------------------------------
// the value types
// ---------------------------------------------------------------------

#[test]
fn format_names_round_trip() {
    for format in [OutputFormat::Human, OutputFormat::Json, OutputFormat::Sarif] {
        assert_eq!(OutputFormat::from_name(format.as_str()), Some(format));
    }
    assert_eq!(OutputFormat::from_name("xml"), None);
}

#[test]
fn verb_names_are_the_words_typed() {
    assert_eq!(Verb::Version.name(), "--version");
    assert_eq!(Verb::Check(scan(OutputFormat::Human, &[])).name(), "check");
    assert_eq!(Verb::Graph(scan(OutputFormat::Human, &[])).name(), "graph");
    assert_eq!(
        Verb::Lint {
            fix: false,
            trust_config: false,
            sites: false,
            scan: scan(OutputFormat::Human, &[]),
        }
        .name(),
        "lint"
    );
    assert_eq!(
        Verb::Extract {
            write: false,
            relocate: false,
            targets: vec![],
        }
        .name(),
        "extract"
    );
    assert_eq!(
        Verb::Langs {
            format: OutputFormat::Human
        }
        .name(),
        "langs"
    );
}
