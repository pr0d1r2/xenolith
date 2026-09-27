//! The host's own helpers, one by one (`languages/nix:T12`, `src:C139`).
//!
//! `tests/host.rs` drives `NixHost` through the `Host` trait; this file
//! reaches the private functions behind it -- `parse`'s all-or-nothing
//! refusal (`languages:V78`), `site`'s spans off grammar tokens
//! (`languages/api/src/site:V38`), `span`'s conversion and `cmd`'s argv --
//! so a regression names the helper that broke rather than a scenario.

use std::path::Path;

use rnix::{SyntaxKind, SyntaxNode, TextRange, TextSize};
use xenolith_lang_api::{
    DelimKind, Error, FileArg, Format, Host, Invoke, LangId, LintCmd, LoadRef, Site, Span,
};

use super::{NixHost, cmd, parse, sinks, site, span};

/// The tree for `src`, or a panic naming the refusal. The workspace denies
/// `expect` (`src:C5`), tests included.
fn tree(src: &str) -> SyntaxNode {
    parse(src).unwrap_or_else(|e| panic!("{src:?} did not parse: {e}"))
}

/// The string node whose source text is exactly `text`, quotes included.
fn string(src: &str, text: &str) -> SyntaxNode {
    tree(src)
        .descendants()
        .filter(|n| n.kind() == SyntaxKind::NODE_STRING)
        .find(|n| n.text() == text)
        .unwrap_or_else(|| panic!("{src:?}: no string {text:?}"))
}

/// The site `site` builds for the string `text` in `src`, under `sink`.
fn site_of(src: &str, text: &str, sink: sinks::Sink) -> Site {
    site(&string(src, text), sink).unwrap_or_else(|| panic!("{src:?}: no site for {text:?}"))
}

/// The text `span` covers in `src`, or a panic.
fn cut(span: Span, src: &str) -> &str {
    span.of(src)
        .unwrap_or_else(|| panic!("{span:?} does not fit {src:?}"))
}

fn sites(src: &str) -> Vec<Site> {
    NixHost
        .sites(src)
        .unwrap_or_else(|e| panic!("{src:?} did not parse: {e}"))
}

// --- guest_by_shebang ---------------------------------------------------

#[test]
fn only_a_site_its_shebang_made_is_guest_by_shebang() {
    // `languages/nix:T157` against a named sink: `script` is shell by its
    // sink even when its body starts with `#!`.
    let src = "{\n  environment.etc.\"x\".text = ''\n    #!/usr/bin/env python3\n    \
               print(1)\n  '';\n  systemd.services.a.script = ''\n    #!/bin/sh\n    \
               a && b\n  '';\n}\n";
    let found = sites(src);
    let by_shebang: Vec<(LangId, bool)> = found
        .iter()
        .map(|site| (site.guest, NixHost.guest_by_shebang(src, site)))
        .collect();
    assert_eq!(
        by_shebang,
        vec![(LangId::Python, true), (LangId::Shell, false)]
    );
    // Source that no longer parses has no shebang site to point at.
    let first = found.first().unwrap_or_else(|| panic!("two sites"));
    assert!(!NixHost.guest_by_shebang("{ broken", first));
}

#[test]
fn builder_text_its_shebang_made_is_guest_by_shebang() {
    // `languages/nix:T160`: the text of `writeScript` / `writeText` is a
    // site only by its shebang, so the shebang-guest path answers for it.
    let src = "{\n  a = pkgs.writeScript \"a\" ''\n    #!/bin/sh\n    a && b\n  '';\n  \
               b = pkgs.writeText \"b\" \"#!/usr/bin/env python3\\nprint(1)\";\n}\n";
    let by_shebang: Vec<(LangId, bool)> = sites(src)
        .iter()
        .map(|site| (site.guest, NixHost.guest_by_shebang(src, site)))
        .collect();
    assert_eq!(
        by_shebang,
        vec![(LangId::Shell, true), (LangId::Python, true)]
    );
}

// --- span --------------------------------------------------------------

#[test]
fn span_carries_a_rowan_range_byte_for_byte() {
    let range = TextRange::new(TextSize::from(3), TextSize::from(9));
    assert_eq!(span(range), Span::new(3, 9));
}

#[test]
fn span_of_an_empty_range_is_empty() {
    let range = TextRange::empty(TextSize::from(4));
    assert_eq!(span(range), Span::new(4, 4));
    assert!(span(range).is_empty());
}

// --- cmd ---------------------------------------------------------------

#[test]
fn cmd_appends_the_file_and_keeps_argv_and_format() {
    assert_eq!(
        cmd(&["statix", "check"], Format::Json("statix")),
        LintCmd {
            argv: vec!["statix".to_owned(), "check".to_owned()],
            file_arg: FileArg::Append,
            format: Format::Json("statix"),
        }
    );
}

#[test]
fn cmd_of_no_arguments_is_an_empty_argv() {
    let empty = cmd(&[], Format::Raw);
    assert!(empty.argv.is_empty());
    assert_eq!(empty.file_arg, FileArg::Append);
}

// --- parse -------------------------------------------------------------

#[test]
fn parse_hands_back_the_whole_tree() {
    let src = "{ a = 1; }";
    let root = tree(src);
    assert_eq!(root.kind(), SyntaxKind::NODE_ROOT);
    // Lossless: the tree's text is the source, byte for byte.
    assert_eq!(root.text().to_string(), src);
}

#[test]
fn parse_refuses_a_file_with_any_error() {
    // `languages:V78`: rnix recovers, but spans after an error are not
    // trustworthy, so one error fails the whole file.
    // An empty file is not a nix expression either.
    for src in [
        "{ a = ; }",
        "{ a = 1; ",
        "''unterminated",
        "{ a = 1; } }",
        "",
    ] {
        match parse(src) {
            Err(Error::Parse { lang, message }) => {
                assert_eq!(lang, LangId::Nix, "{src:?}");
                assert!(!message.is_empty(), "{src:?}: empty message");
            }
            other => panic!("{src:?}: expected a parse error, got {other:?}"),
        }
    }
}

// --- site --------------------------------------------------------------

#[test]
fn site_of_an_indented_string_spans_its_two_quote_pairs() {
    let src = "{ script = ''\n  echo hi\n''; }";
    let found = site_of(src, "''\n  echo hi\n''", sinks::Sink::ServiceScript);
    assert_eq!(found.delim.kind, DelimKind::NixIndented);
    assert_eq!(cut(found.delim.open, src), "''");
    assert_eq!(cut(found.delim.close, src), "''");
    assert_eq!(cut(found.delim.body, src), "\n  echo hi\n");
    assert_eq!(found.delim.body.start, found.delim.open.end);
    assert_eq!(found.delim.body.end, found.delim.close.start);
}

#[test]
fn site_of_a_double_quoted_string_spans_single_quotes() {
    let src = r#"{ ExecStart = "run x"; }"#;
    let found = site_of(src, r#""run x""#, sinks::Sink::ExecStart);
    assert_eq!(found.delim.kind, DelimKind::NixString);
    assert_eq!(cut(found.delim.open, src), "\"");
    assert_eq!(cut(found.delim.close, src), "\"");
    assert_eq!(cut(found.delim.body, src), "run x");
}

#[test]
fn site_of_an_empty_string_has_an_empty_body_between_its_quotes() {
    let src = r#"{ script = ""; }"#;
    let found = site_of(src, r#""""#, sinks::Sink::ServiceScript);
    assert!(found.delim.body.is_empty());
    assert_eq!(found.delim.body.start, found.delim.open.end);
    assert!(found.holes.is_empty());
}

#[test]
fn site_takes_env_and_guest_from_the_sink_and_path_from_the_tree() {
    let src = "{ a.b.shellHook = ''x''; }";
    let found = site_of(src, "''x''", sinks::Sink::ShellHook);
    assert_eq!(found.guest, LangId::Shell);
    assert_eq!(found.env, sinks::Sink::ShellHook.env());
    assert_eq!(found.sink, "a.b.shellHook");
}

#[test]
fn site_takes_the_guest_a_shebang_names() {
    // `languages/nix:T157`: a shebang-led value is the guest its
    // interpreter names, not shell by default.
    let src = r##"{ a.text = "#!/usr/bin/env python3\nprint(1)"; }"##;
    let python = sinks::Sink::Shebang {
        guest: LangId::Python,
        dialect: None,
    };
    let found = site_of(src, r##""#!/usr/bin/env python3\nprint(1)""##, python);
    assert_eq!(found.guest, LangId::Python);
    assert_eq!(found.env, python.env());
    assert_eq!(found.sink, "a.text");
    assert_eq!(sites(src), vec![found]);
}

#[test]
fn site_holes_are_each_interpolation_whole() {
    let src = "{ script = ''a ${x} b ${y.z} c''; }";
    let found = site_of(src, "''a ${x} b ${y.z} c''", sinks::Sink::ServiceScript);
    let holes: Vec<&str> = found.holes.iter().map(|h| cut(*h, src)).collect();
    assert_eq!(holes, ["${x}", "${y.z}"]);
}

#[test]
fn site_does_not_count_escaped_dollars_as_holes() {
    // `''${` in an indented string and `\${` in a double-quoted one are
    // literal `${` -- string tokens, never interpolations.
    let src = "{ script = ''a ''${x} b''; }";
    let found = site_of(src, "''a ''${x} b''", sinks::Sink::ServiceScript);
    assert!(found.holes.is_empty(), "{:?}", found.holes);

    let src = r#"{ script = "a \${x}"; }"#;
    let found = site_of(src, r#""a \${x}""#, sinks::Sink::ServiceScript);
    assert!(found.holes.is_empty(), "{:?}", found.holes);
}

#[test]
fn site_does_not_descend_into_a_holes_own_strings() {
    // The inner string is a child of the hole, not of the outer string:
    // it adds no hole and no delimiter of its own to the outer site.
    let src = r#"{ script = "a ${f "b"} c"; }"#;
    let found = site_of(src, r#""a ${f "b"} c""#, sinks::Sink::ServiceScript);
    let holes: Vec<&str> = found.holes.iter().map(|h| cut(*h, src)).collect();
    assert_eq!(holes, [r#"${f "b"}"#]);
    assert_eq!(cut(found.delim.close, src), "\"");
    assert_eq!(src.get(found.delim.close.end..), Some("; }"));
}

// --- NixHost -----------------------------------------------------------

#[test]
fn sites_skip_strings_that_are_not_sinks() {
    assert!(sites(r#"{ description = "echo hi"; }"#).is_empty());
    assert!(sites("1").is_empty());
}

#[test]
fn sites_come_back_ordered_by_opening_delimiter() {
    let src = r#"{ b.script = "x"; a.shellHook = "${writeShellScript "n" "y"}"; }"#;
    let opens: Vec<usize> = sites(src).iter().map(|s| s.delim.open.start).collect();
    assert_eq!(opens.len(), 3);
    assert!(opens.windows(2).all(|w| w.first() < w.get(1)), "{opens:?}");
}

#[test]
fn sites_refuse_a_broken_file_instead_of_reporting_none() {
    assert!(matches!(
        NixHost.sites("{ script = ''x''; "),
        Err(Error::Parse {
            lang: LangId::Nix,
            ..
        })
    ));
}

#[test]
fn claims_reads_the_extension_and_never_the_head() {
    assert!(NixHost.claims(Path::new("a/b.nix"), ""));
    // `.nix` is a dotfile with no extension, per `Path::extension`.
    assert!(!NixHost.claims(Path::new(".nix"), ""));
    assert!(!NixHost.claims(Path::new("b.NIX"), ""));
    assert!(!NixHost.claims(Path::new("b.nix.bak"), ""));
    assert!(!NixHost.claims(Path::new("shell"), "#!/usr/bin/env nix-shell"));
}

#[test]
fn rewrite_and_inline_are_the_write_side_and_undo_each_other() {
    // `languages/nix:V170`: the V53 load in -- `readFile`, as nothing binds
    // `nix-shebang` here -- then the body back out of it.
    let src = "{ script = ''\n  a\n  b\n''; }";
    let site = sites(src)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("no site"));
    let invoke = Invoke { argv: Vec::new() };
    let call = "builtins.readFile ./x.sh";
    let written = NixHost.rewrite(src, &site, &invoke, Path::new("./x.sh"));
    assert_eq!(written, Ok(format!("{{ script = {call}; }}")));
    let load = LoadRef {
        span: Span::new(11, 11 + call.len()),
        path: "./x.sh".into(),
        guest: LangId::Shell,
    };
    let written = written.unwrap_or_default();
    assert_eq!(
        NixHost.inline(&written, &load, "a\nb\n"),
        Ok(src.to_owned())
    );
    assert!(matches!(
        NixHost.inline("", &load, ""),
        Err(Error::Parse {
            lang: LangId::Nix,
            ..
        })
    ));
}

#[test]
fn checks_and_fixers_pair_up_tool_by_tool() {
    let tool = |c: &LintCmd| c.argv.first().cloned();
    let checks: Vec<_> = NixHost.checks().iter().map(tool).collect();
    let fixers: Vec<_> = NixHost.fixers().iter().map(tool).collect();
    assert_eq!(checks, fixers);
    // A fixer's output is the file it rewrote, never a report.
    assert!(NixHost.fixers().iter().all(|c| c.format == Format::Raw));
    assert!(
        NixHost
            .checks()
            .iter()
            .chain(NixHost.fixers().iter())
            .all(|c| c.file_arg == FileArg::Append)
    );
}
