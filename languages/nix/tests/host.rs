//! Nix as a HOST: what it claims, what it refuses, and the shape of a
//! site it reports (`languages/nix:T12`).
//!
//! The per-sink cases live in `tests/fixtures/` and run through
//! `tests/fixtures.rs`; this file holds what a fixture cannot say -- the
//! exact spans, the error on a broken file, and the capabilities this
//! crate does not offer YET.

use std::path::Path;

use xenolith_lang_api::{Delim, DelimKind, Error, GuestEnv, Host, LangId, Site, Span};
use xenolith_lang_nix::NixHost;

const NIX: NixHost = NixHost;

/// The sites in `src`, or a panic naming the parse failure. The workspace
/// denies `expect` (`src:C5`), tests included.
fn sites(src: &str) -> Vec<Site> {
    NIX.sites(src)
        .unwrap_or_else(|e| panic!("{src:?} did not parse: {e}"))
}

/// The one site in `src`, or a panic saying how many there were.
fn only_site(src: &str) -> Site {
    match <[Site; 1]>::try_from(sites(src)) {
        Ok([site]) => site,
        Err(found) => panic!("{src:?}: expected one site, got {found:#?}"),
    }
}

fn bash(options: &[&str]) -> GuestEnv {
    GuestEnv {
        dialect: Some("bash".to_owned()),
        options: options.iter().map(|o| (*o).to_owned()).collect(),
    }
}

#[test]
fn nix_is_the_host_it_says_it_is() {
    assert_eq!(NIX.id(), LangId::Nix);
}

#[test]
fn claims_nix_files_by_extension_only() {
    // `languages:V56`: `claims` is the only thing that routes a file to a
    // host. A `nix-shell` shebang makes a file a SHELL script with a nix
    // interpreter line, not a nix expression, so the head is ignored.
    assert!(NIX.claims(Path::new("flake.nix"), ""));
    assert!(NIX.claims(Path::new("nix/module/default.nix"), ""));
    assert!(!NIX.claims(Path::new("flake.lock"), ""));
    assert!(!NIX.claims(Path::new("run.sh"), "#!/usr/bin/env nix-shell"));
    assert!(!NIX.claims(Path::new("nix"), ""));
}

#[test]
fn an_indented_string_site_carries_grammar_spans() {
    // `languages/api/src/site:V38`: the delimiter comes off the grammar
    // node, so `open` is exactly the two quotes, `close` the two closing
    // ones, and `body` everything between -- still in host escaping.
    let src = "{ systemd.services.foo.script = ''\n  echo hi\n''; }";
    let site = only_site(src);
    assert_eq!(site.sink, "systemd.services.foo.script");
    assert_eq!(site.guest, LangId::Shell);
    assert_eq!(site.delim.kind, DelimKind::NixIndented);
    assert_eq!(site.delim.open, Span::new(32, 34));
    assert_eq!(site.delim.body, Span::new(34, 45));
    assert_eq!(site.delim.close, Span::new(45, 47));
    assert_eq!(site.delim.body.of(src), Some("\n  echo hi\n"));
    assert!(site.holes.is_empty());
}

#[test]
fn a_double_quoted_string_is_a_site_too() {
    let src = r#"{ shellHook = "echo hi"; }"#;
    let site = only_site(src);
    assert_eq!(site.delim.kind, DelimKind::NixString);
    assert_eq!(site.delim.open, Span::new(14, 15));
    assert_eq!(site.delim.body.of(src), Some("echo hi"));
    assert_eq!(site.delim.close, Span::new(22, 23));
}

#[test]
fn holes_are_the_interpolations_and_an_escaped_dollar_is_not_one() {
    // `''${` is nix's escape for a literal `${` inside an indented
    // string: shell sees `${HOME}`, nix interpolates nothing. The grammar
    // already knows that; counting `${` in the bytes would not.
    let src = "{ script = ''\n  ${pkgs.hello}/bin/hello ''${HOME}\n''; }";
    let holes: Vec<Option<&str>> = only_site(src).holes.iter().map(|h| h.of(src)).collect();
    assert_eq!(holes, vec![Some("${pkgs.hello}")]);
}

#[test]
fn the_env_is_the_one_nix_runs_that_body_under() {
    // `languages/shell:V82`: the HOST declares dialect and options, and
    // the prelude reproduces them. Each value below is what nixpkgs or
    // NixOS itself wraps the body in.
    let src = r#"{
  systemd.services.a.script = "x";
  shellHook = "x";
  p = pkgs.writeShellScript "p" "x";
  q = pkgs.writeShellApplication { name = "q"; text = "x"; };
  r = pkgs.runCommand "r" { } "x";
  buildPhase = "x";
}"#;
    let envs: Vec<(String, GuestEnv)> = sites(src).into_iter().map(|s| (s.sink, s.env)).collect();
    assert_eq!(
        envs,
        vec![
            // NixOS `makeJobScript`: `#! ${runtimeShell} -e`.
            ("systemd.services.a.script".to_owned(), bash(&["errexit"])),
            // Sourced into the interactive shell; nothing is imposed.
            ("shellHook".to_owned(), bash(&[])),
            // `writeShellScript`: `#!${runtimeShell}` and nothing else.
            ("p.writeShellScript".to_owned(), bash(&[])),
            // `writeShellApplication`'s default `bashOptions`.
            (
                "q.writeShellApplication.text".to_owned(),
                bash(&["errexit", "nounset", "pipefail"])
            ),
            // stdenv `setup.sh`: `set -eu` and `set -o pipefail`.
            (
                "r.runCommand".to_owned(),
                bash(&["errexit", "nounset", "pipefail"])
            ),
            (
                "buildPhase".to_owned(),
                bash(&["errexit", "nounset", "pipefail"])
            ),
        ]
    );
}

#[test]
fn sites_come_back_sorted_by_span() {
    // `languages/api:V36`: sorted output, so a parallel scan merges into
    // the same bytes every time.
    let src = r#"{ b.script = "two"; a.script = "one"; c = { installPhase = "three"; }; }"#;
    let starts: Vec<usize> = sites(src).iter().map(|s| s.delim.open.start).collect();
    let mut sorted = starts.clone();
    sorted.sort_unstable();
    assert_eq!(starts.len(), 3);
    assert_eq!(starts, sorted);
}

#[test]
fn a_file_that_does_not_parse_is_an_error_not_an_empty_list() {
    // An empty list reads as "clean". A broken file is not clean, it is
    // unread, and the caller has to be able to tell (`languages:V78`).
    let found = NIX.sites("{ script = ''echo hi");
    assert!(
        matches!(
            found,
            Err(Error::Parse {
                lang: LangId::Nix,
                ..
            })
        ),
        "{found:?}"
    );
}

#[test]
fn what_is_not_built_yet_says_so() {
    // `languages/api:V37`: a missing capability is loud. `rewrite` and
    // `inline` are not T12's; until a task lands them they refuse rather
    // than answer "nothing here".
    assert!(matches!(
        NIX.inline("{ }", &dummy_load(), "echo hi"),
        Err(Error::Unsupported {
            lang: LangId::Nix,
            operation: "inline"
        })
    ));
}

#[test]
fn loads_reads_the_v53_calls_back_with_their_spans() {
    // `languages/nix:V53`; the table of shapes is
    // `tests/fixtures/neg-loads/loads.txt`. The span is the whole call,
    // which is what `inline` will replace with the body.
    let src = "{ systemd.services.a.script = builtins.readFile ./a/x.sh; }";
    let found = NIX
        .loads(src)
        .unwrap_or_else(|e| panic!("loads failed: {e}"));
    let [load] = found.as_slice() else {
        panic!("expected one load, got {found:#?}");
    };
    assert_eq!(load.path, Path::new("./a/x.sh"));
    assert_eq!(load.guest, LangId::Shell);
    assert_eq!(load.span.of(src), Some("builtins.readFile ./a/x.sh"));
    assert_eq!(NIX.loads("{ }"), Ok(Vec::new()));
}

#[test]
fn loads_refuses_a_file_that_does_not_parse() {
    // `languages:V78`, as `sites` does: spans after an error are guesses.
    assert!(matches!(
        NIX.loads("{ a = builtins.readFile ./x.sh"),
        Err(Error::Parse {
            lang: LangId::Nix,
            ..
        })
    ));
}

fn dummy_load() -> xenolith_lang_api::LoadRef {
    xenolith_lang_api::LoadRef {
        span: Span::new(0, 0),
        path: "x.sh".into(),
        guest: LangId::Shell,
    }
}

#[test]
fn host_checks_are_statix_deadnix_and_nixfmt() {
    // `languages/api` §I names the nix host checks.
    let argv: Vec<Vec<String>> = NIX.checks().into_iter().map(|c| c.argv).collect();
    let tools: Vec<&str> = argv
        .iter()
        .map(|a| a.first().map_or("", String::as_str))
        .collect();
    assert_eq!(tools, vec!["statix", "deadnix", "nixfmt"]);
    assert!(!NIX.fixers().is_empty());
}

// --- placement & hole advice (`languages/nix:T55`) ---------------------

#[test]
fn placement_names_the_extract_after_its_attribute_path() {
    // `languages/nix:V53`; the per-shape table is `tests/fixtures/*/
    // placement.txt`. Both fields are `src/extract:V46` templates: the
    // site does not know which file it came from.
    let site = only_site("{ systemd.services.foo.script = ''\n  a\n  b\n''; }");
    let at = NIX
        .placement(&site)
        .unwrap_or_else(|e| panic!("no placement: {e}"));
    assert_eq!(at.name, "foo-script");
    assert_eq!(at.dir, "{host_dir}/{host_stem}");
}

#[test]
fn hole_advice_proposes_replace_vars_then_argv_then_env() {
    // `languages/nix:V54`: advice only, one direction per way out, the
    // nix-native one first.
    let src = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pos-holes/input.nix"),
    )
    .unwrap_or_else(|e| panic!("pos-holes: {e}"));
    let site = only_site(&src);
    assert_eq!(site.holes.len(), 2);
    let advice = NIX
        .hole_advice(&site)
        .unwrap_or_else(|e| panic!("no advice: {e}"));
    let [replace_vars, argv, env] = advice.as_slice() else {
        panic!("expected three directions, got {advice:#?}");
    };
    assert!(replace_vars.contains("replaceVars ./<file> { var = …; }"));
    assert!(replace_vars.contains("`@var@`"));
    assert!(argv.contains("argument"), "{argv}");
    assert!(env.contains("environment variable"), "{env}");
}

// --- unescape (`languages/api/src/lens:V39`, `languages/nix:T158`) ------

/// A delimiter of `kind`; `unescape` reads only the kind.
fn delim(kind: DelimKind) -> Delim {
    Delim {
        kind,
        open: Span::new(0, 0),
        body: Span::new(0, 0),
        close: Span::new(0, 0),
    }
}

fn indented(raw: &str) -> String {
    NIX.unescape(&delim(DelimKind::NixIndented), raw)
        .unwrap_or_else(|e| panic!("{raw:?}: {e}"))
}

#[test]
fn unescape_dedents_an_indented_body_as_nix_does() {
    // The opening line break is not content, the common indent of the
    // content lines goes, and the closing line's spaces go with it -- so
    // an indented heredoc terminator is a terminator again.
    assert_eq!(
        indented("\n    cat <<EOF\n    hi\n    EOF\n  "),
        "cat <<EOF\nhi\nEOF\n"
    );
    assert_eq!(
        indented("\n    if a; then\n      b\n    fi\n"),
        "if a; then\n  b\nfi\n"
    );
    // A line of spaces only does not set the indent.
    assert_eq!(indented("\n    a\n\n  \n    b\n"), "a\n\n\nb\n");
    // Content on the opening line counts, spaces before it included.
    assert_eq!(indented("  a\n  b"), "a\nb");
    // Tabs are content to nix, never indentation.
    assert_eq!(indented("\n\ta\n\tb\n"), "\ta\n\tb\n");
}

#[test]
fn unescape_resolves_indented_string_escapes() {
    assert_eq!(indented("a ''${b}"), "a ${b}");
    assert_eq!(indented("a ''$b"), "a $b");
    assert_eq!(indented("'''q'''"), "''q''");
    assert_eq!(indented("a''\\nb''\\tc''\\rd''\\xe"), "a\nb\tc\rdxe");
    // A lone quote and `$${` are plain text.
    assert_eq!(indented("it's $${x}"), "it's $${x}");
}

#[test]
fn unescape_counts_an_escape_as_content_for_the_indent() {
    // `''$` at the start of a line ends the line's indent and is never
    // stripped itself.
    assert_eq!(indented("\n    ''$a\n      b\n"), "$a\n  b\n");
    assert_eq!(indented("\n  ''$a\n    b\n"), "$a\n  b\n");
}

#[test]
fn unescape_resolves_double_quoted_escapes_and_keeps_indent() {
    let quoted = |raw: &str| {
        NIX.unescape(&delim(DelimKind::NixString), raw)
            .unwrap_or_else(|e| panic!("{raw:?}: {e}"))
    };
    assert_eq!(
        quoted(r#"a\nb\tc\rd\"e\\f\${g}\h"#),
        "a\nb\tc\rd\"e\\f${g}h"
    );
    assert_eq!(quoted("  a\n  b"), "  a\n  b");
}

#[test]
fn unescape_refuses_a_delimiter_nix_does_not_write() {
    assert_eq!(
        NIX.unescape(&delim(DelimKind::PklMultiline { pounds: 0 }), "x"),
        Err(Error::unsupported(LangId::Nix, "unescape"))
    );
}
