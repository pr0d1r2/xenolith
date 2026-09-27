//! Unit tests for the just host's private steps (`src:C139`).
//!
//! `tests/host.rs`, `tests/fixtures.rs` and `tests/lens.rs` hold the
//! trait-level behaviour over fixtures; these pin one branch at a time:
//! which recipe makes which site, what a load line is, and every refusal
//! of the extract direction (`languages/ci/just:V180`).

use xenolith_lang_api::{DelimKind, Error, Invoke, LangId, Site, Span};

use super::{extract, line_break, load, located, plain_word, splice};
use crate::recipe;

fn sites(src: &str) -> Vec<Site> {
    let tree = recipe::parse(src).unwrap_or_else(|e| panic!("{src:?}: {e}"));
    let settings = recipe::settings(&tree, src);
    located(&tree, src, &settings)
        .into_iter()
        .map(|(site, _)| site)
        .collect()
}

fn only(src: &str) -> Site {
    let found = sites(src);
    let [site] = found.as_slice() else {
        panic!("{src:?}: expected one site, got {found:#?}");
    };
    site.clone()
}

fn sh(path: &str) -> Invoke {
    Invoke {
        argv: vec!["sh".to_owned(), path.to_owned()],
    }
}

/// The refused operation's text, or a panic naming what came back.
fn refusal(src: &str) -> &'static str {
    match extract(src, &only(src), &sh("./scripts/just/a.sh")) {
        Err(Error::Unsupported { operation, .. }) => operation,
        other => panic!("{src:?}: expected a refusal, got {other:?}"),
    }
}

// --- site -------------------------------------------------------------------

#[test]
fn a_line_body_is_shell_under_the_file_s_line_shell() {
    let site = only("a:\n    echo hi\n");
    assert_eq!(site.sink, "a");
    assert_eq!(site.guest, LangId::Shell);
    assert_eq!(site.delim.kind, DelimKind::JustRecipe);
    assert_eq!(site.env.dialect.as_deref(), Some("sh"));
    assert_eq!(site.env.options, ["errexit", "nounset"]);
}

#[test]
fn a_line_body_runs_line_by_line_and_a_shebang_body_does_not() {
    // The engine judges a line body line by line up to `[threshold.just]
    // max_lines` (`languages/ci/just:V180`, `src/config:V240`); a shebang
    // body is one script whatever its length.
    let lines = only("a:\n    echo one\n    echo two\n");
    assert!(lines.delim.kind.runs_line_by_line());
    let script = only("a:\n    #!/usr/bin/env bash\n    echo one\n    echo two\n");
    assert!(!script.delim.kind.runs_line_by_line());
}

#[test]
fn an_unreadable_shell_leaves_the_env_empty() {
    let site = only("set windows-shell := [\"pwsh\", \"-c\"]\na:\n    echo hi\n");
    assert_eq!(site.env, xenolith_lang_api::GuestEnv::default());
}

#[test]
fn a_shebang_names_the_guest_and_an_unknown_one_makes_no_site() {
    let py = only("a:\n    #!/usr/bin/env python3\n    print(1)\n");
    assert_eq!(py.guest, LangId::Python);
    assert_eq!(py.delim.kind, DelimKind::JustShebangRecipe);
    assert_eq!(py.env.dialect, None);

    let bash = only("a:\n    #!/bin/bash\n    echo\n");
    assert_eq!(bash.env.dialect.as_deref(), Some("bash"));
    assert!(bash.env.options.is_empty());

    assert!(sites("a:\n    #!/usr/bin/env tclsh\n    puts 1\n").is_empty());
}

#[test]
fn a_script_recipe_and_an_empty_recipe_make_no_site() {
    assert!(sites("[script]\na:\n    echo\n\nb:\n").is_empty());
}

#[test]
fn the_delimiter_is_the_header_then_the_body() {
    let src = "build x: dep\n    echo {{x}}\n\ndep:\n";
    let site = only(src);
    assert_eq!(site.delim.open.of(src), Some("build x: dep"));
    assert_eq!(site.delim.body.of(src), Some("    echo {{x}}"));
    assert_eq!(
        site.delim.close,
        Span::new(site.delim.body.end, site.delim.body.end)
    );
    let holes: Vec<&str> = site.holes.iter().filter_map(|h| h.of(src)).collect();
    assert_eq!(holes, ["{{x}}"]);
}

// --- load -------------------------------------------------------------------

#[test]
fn a_load_is_an_interpreter_and_a_relative_shell_script() {
    let src = concat!(
        "a:\n",
        "    @bash ./scripts/just/a.sh {{args}}\n",
        "    sh scripts/b.bash\n",
        "    python3 scripts/c.py\n",
        "    bash /abs/d.sh\n",
        "    bash scripts/{{x}}.sh\n",
        "    bash -c 'x'\n",
        "    echo scripts/e.sh\n",
    );
    let tree = recipe::parse(src).unwrap_or_else(|e| panic!("{e}"));
    let lines = recipe::recipes(&tree, src)
        .first()
        .map(|r| r.lines.clone())
        .unwrap_or_default();
    let found: Vec<String> = lines
        .iter()
        .filter_map(|line| load(*line, src))
        .map(|l| format!("{} {:?}", l.path.display(), l.span.of(src)))
        .collect();
    assert_eq!(
        found,
        [
            "./scripts/just/a.sh Some(\"@bash ./scripts/just/a.sh {{args}}\")",
            "scripts/b.bash Some(\"sh scripts/b.bash\")",
        ]
    );
}

#[test]
fn plain_words_need_no_quoting() {
    assert!(plain_word("./scripts/just/a-b_c.sh"));
    assert!(!plain_word(""));
    assert!(!plain_word("a b"));
    assert!(!plain_word("{{x}}"));
    assert!(!plain_word("$x"));
}

#[test]
fn the_line_break_is_the_host_s() {
    let lf = "a:\n    x\n";
    assert_eq!(line_break(lf, Span::new(7, 8)), "\n");
    let crlf = "a:\r\n    x\r\n";
    assert_eq!(line_break(crlf, Span::new(8, 9)), "\r\n");
    assert_eq!(line_break("x", Span::new(0, 1)), "\n");
}

#[test]
fn splice_refuses_a_span_outside_the_source() {
    assert_eq!(splice("abc", Span::new(1, 2), "X").as_deref(), Ok("aXc"));
    assert!(splice("abc", Span::new(2, 9), "X").is_err());
}

// --- extract ----------------------------------------------------------------

#[test]
fn lines_become_one_load_and_a_merged_script() {
    let src = "build:\n    -rm -f out\n    cc -o out main.c\n\nnext:\n";
    let got =
        extract(src, &only(src), &sh("./scripts/just/build.sh")).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(got.src, "build:\n    sh ./scripts/just/build.sh\n\nnext:\n");
    assert_eq!(got.body, "rm -f out || true\ncc -o out main.c");
}

#[test]
fn an_all_quiet_body_gives_a_quiet_load() {
    let src = "a:\n  @echo one\n  @echo two\n";
    let got = extract(src, &only(src), &sh("./x.sh")).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(got.src, "a:\n  @sh ./x.sh\n");
    assert_eq!(got.body, "echo one\necho two");
}

#[test]
fn the_last_line_may_change_state_since_no_line_reads_it() {
    let src = "a:\n    make\n    cd sub\n";
    assert!(extract(src, &only(src), &sh("./x.sh")).is_ok());
}

#[test]
fn every_merge_that_changes_what_runs_is_refused() {
    for (src, why) in [
        ("a:\n    cd sub\n    make\n", "changes shell state"),
        ("a:\n    FOO=1\n    echo $FOO\n", "changes shell state"),
        ("a:\n    test -f x && rm x\n    ls\n", "&& or !"),
        ("a:\n    false; true\n    ls\n", "without -e"),
        ("a:\n    #!/bin/sh\n    echo\n    ls\n", "shebang recipe"),
        ("a x:\n    echo {{x}}\n    ls\n", "holes"),
        (
            "set shell := [\"fish\", \"-c\"]\na:\n    a\n    b\n",
            "not statically readable",
        ),
        (
            "import 'o.just'\na:\n    a\n    b\n",
            "not statically readable",
        ),
        (
            "set positional-arguments\na x:\n    a\n    b\n",
            "positional",
        ),
        ("[positional-arguments]\na x:\n    a\n    b\n", "positional"),
        ("a:\n    @a\n    b\n", "@ only in part"),
        ("a:\n    a \\\n      b\n    c\n", "continued"),
    ] {
        let got = refusal(src);
        assert!(got.contains(why), "{src:?}: {got}");
    }
}

#[test]
fn a_semicolon_is_merged_when_the_shell_already_stops_at_it() {
    let src = "set shell := [\"bash\", \"-euc\"]\na:\n    false; true\n    ls\n";
    let got = extract(src, &only(src), &sh("./x.sh")).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(got.body, "false; true\nls");
}

#[test]
fn a_load_needing_quotes_or_a_foreign_site_is_refused() {
    let src = "a:\n    a\n    b\n";
    let spaced = Invoke {
        argv: vec!["sh".to_owned(), "./a b.sh".to_owned()],
    };
    assert!(matches!(
        extract(src, &only(src), &spaced),
        Err(Error::Unsupported { .. })
    ));
    let mut stranger = only(src);
    stranger.sink = "other".to_owned();
    assert!(matches!(
        extract(src, &stranger, &sh("./x.sh")),
        Err(Error::Parse { .. })
    ));
}
