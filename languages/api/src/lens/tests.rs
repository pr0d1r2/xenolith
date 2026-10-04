//! Unit tests for `lens` (`src:C139`): the escape law
//! (`languages/api/src/lens:V39`), run against toy hosts that keep it,
//! break it, or never offer `escape` at all.

use std::path::Path;

use super::escape_law;
use crate::{Delim, DelimKind, Error, Invoke, LangId, LintCmd, LoadRef, Result, Site, Span};

/// Every delimiter kind, one of each. The `match` fails to compile when a
/// kind is added, which is the moment V39 wants a fixture for it too.
fn every_kind() -> Vec<DelimKind> {
    let kinds = vec![
        DelimKind::NixIndented,
        DelimKind::NixString,
        DelimKind::Heredoc {
            tag: "EOF".into(),
            quoted: false,
            strip_indent: true,
        },
        DelimKind::PklMultiline { pounds: 1 },
        DelimKind::YamlBlock {
            literal: true,
            chomp: Some('-'),
        },
        DelimKind::HtmlElement {
            tag: "script".into(),
        },
        DelimKind::RustRawString { pounds: 1 },
        DelimKind::RubyHeredoc {
            tag: "SQL".into(),
            squiggly: true,
        },
        DelimKind::ArgvString,
        DelimKind::JustRecipe,
        DelimKind::JustShebangRecipe,
    ];
    for kind in &kinds {
        match kind {
            DelimKind::NixIndented
            | DelimKind::NixString
            | DelimKind::Heredoc { .. }
            | DelimKind::PklMultiline { .. }
            | DelimKind::YamlBlock { .. }
            | DelimKind::HtmlElement { .. }
            | DelimKind::RustRawString { .. }
            | DelimKind::RubyHeredoc { .. }
            | DelimKind::ArgvString
            | DelimKind::JustRecipe
            | DelimKind::JustShebangRecipe => {}
        }
    }
    kinds
}

fn delim(kind: DelimKind) -> Delim {
    Delim {
        kind,
        open: Span::new(0, 2),
        body: Span::new(2, 2),
        close: Span::new(2, 4),
    }
}

/// Indent AND escapes in one raw body: a flush line, a nested line, a
/// blank line, an escaped `$` and an escaped backslash.
const RAW: &str = "  if true; then\n    echo \\$HOME \\\\n\n\n  fi\n";

/// A host with a nix-like body: common indent stripped, `\x` decodes to
/// `x`. `LOSSY` forgets to escape backslashes on the way back.
struct Toy<const LOSSY: bool>;

/// A host that never wrote `escape`.
struct NoEscape;

fn dedent(raw: &str) -> String {
    let indent = raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start_matches(' ').len())
        .min()
        .unwrap_or(0);
    raw.split_inclusive('\n')
        .map(|l| l.get(indent..).unwrap_or(l.trim_start_matches(' ')))
        .collect()
}

fn toy_unescape(raw: &str) -> Result<String> {
    let mut out = String::new();
    let text = dedent(raw);
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            let next = chars
                .next()
                .ok_or_else(|| Error::parse(LangId::Nix, "a trailing backslash"))?;
            out.push(next);
        } else {
            out.push(c);
        }
    }
    Ok(out)
}

fn toy_escape(body: &str, lossy: bool) -> String {
    let mut out = String::new();
    for line in body.split_inclusive('\n') {
        if !line.trim().is_empty() {
            out.push_str("  ");
        }
        for c in line.chars() {
            if c == '$' || (c == '\\' && !lossy) {
                out.push('\\');
            }
            out.push(c);
        }
    }
    out
}

macro_rules! host_body {
    () => {
        fn id(&self) -> LangId {
            LangId::Nix
        }
        fn claims(&self, _: &Path, _: &str) -> bool {
            false
        }
        fn sites(&self, _: &str) -> Result<Vec<Site>> {
            Ok(Vec::new())
        }
        fn loads(&self, _: &str) -> Result<Vec<LoadRef>> {
            Ok(Vec::new())
        }
        fn rewrite(&self, src: &str, _: &Site, _: &Invoke, _: &Path) -> Result<String> {
            Ok(src.to_owned())
        }
        fn inline(&self, src: &str, _: &LoadRef, _: &str) -> Result<String> {
            Ok(src.to_owned())
        }
        fn unescape(&self, _: &Delim, raw: &str) -> Result<String> {
            toy_unescape(raw)
        }
        fn checks(&self) -> Vec<LintCmd> {
            Vec::new()
        }
        fn fixers(&self) -> Vec<LintCmd> {
            Vec::new()
        }
    };
}

impl<const LOSSY: bool> crate::Host for Toy<LOSSY> {
    host_body!();
    fn escape(&self, _: &Delim, body: &str) -> Result<String> {
        Ok(toy_escape(body, LOSSY))
    }
}

impl crate::Host for NoEscape {
    host_body!();
}

/// The break a law reported; a law that held here is the test failing.
fn break_of(law: std::result::Result<(), String>, why: &str) -> String {
    match law {
        Err(message) => message,
        Ok(()) => panic!("the law held, but should break: {why}"),
    }
}

#[test]
fn the_toy_reads_indent_and_escapes() {
    // The fixture really has both, or the law below proves nothing.
    assert_eq!(
        toy_unescape(RAW),
        Ok("if true; then\n  echo $HOME \\n\n\nfi\n".to_owned())
    );
}

#[test]
fn a_host_whose_escape_inverts_unescape_keeps_the_law() {
    let host = Toy::<false>;
    assert_eq!(
        escape_law(&host, &delim(DelimKind::NixIndented), RAW),
        Ok(())
    );
}

#[test]
fn the_law_holds_through_every_delimiter_kind() {
    // The law is kind-agnostic: it hands the delimiter through to the
    // host, whose own fixtures per kind live in its crate (`tests:V14`).
    let host = Toy::<false>;
    for kind in every_kind() {
        assert_eq!(
            escape_law(&host, &delim(kind.clone()), RAW),
            Ok(()),
            "{kind:?}"
        );
    }
}

#[test]
fn a_lossy_escape_breaks_the_law_and_the_break_names_it() {
    let host = Toy::<true>;
    let broken = break_of(
        escape_law(&host, &delim(DelimKind::NixIndented), RAW),
        "a backslash left unescaped is read back as an escape",
    );
    assert!(broken.contains("languages/api/src/lens:V39"), "{broken}");
    assert!(
        broken.contains("echo $HOME \\\\n"),
        "names the body: {broken}"
    );
}

#[test]
fn a_host_without_escape_breaks_the_law_rather_than_passing_it() {
    // `languages/api:V37`: a missing `escape` is a missing capability,
    // and a harness that skipped it would call the host lawful.
    let broken = break_of(
        escape_law(&NoEscape, &delim(DelimKind::NixIndented), RAW),
        "no escape, no round trip",
    );
    assert!(broken.contains("does not support `escape`"), "{broken}");
}

#[test]
fn a_raw_body_the_host_cannot_read_is_a_break_not_a_pass() {
    let broken = break_of(
        escape_law(&Toy::<false>, &delim(DelimKind::NixIndented), "x\\"),
        "a trailing backslash does not unescape",
    );
    assert!(broken.contains("trailing backslash"), "{broken}");
}

#[test]
fn a_body_whose_escape_does_not_read_back_is_a_break() {
    // escape(b) must itself unescape: a lossy escape of a trailing
    // backslash leaves one the host cannot read.
    let broken = break_of(
        escape_law(&Toy::<true>, &delim(DelimKind::NixIndented), "x\\\\"),
        "escape(b) ends in a lone backslash",
    );
    assert!(broken.contains("trailing backslash"), "{broken}");
}
