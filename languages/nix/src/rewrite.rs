//! The write side of the nix host (`languages/nix:V170`): `rewrite` takes
//! a site's string out and leaves the `languages/nix:V53` load in its
//! place, `inline` puts a body back where a load was.
//!
//! Both answer the question the other one asks. `rewrite` writes only
//! what `loads` reads back -- a relative path literal to a shell file --
//! and checks that it does; `inline` writes a string whose body, read by
//! `unescape`, is exactly the body it was given, and checks that too. A
//! check that fails is a refusal, never a best effort: an extraction that
//! changed what the host runs is worse than none (`src/extract:V4`).
//!
//! What `rewrite` refuses, it refuses by name:
//!
//! - holes: `replaceVars` and friends are advice (`languages/nix:V54`),
//!   and a `${…}` copied into a shell file is shell, not nix;
//! - a systemd exec line: its load is `toShellScript`
//!   (`languages/nix:V69`), which is `languages/nix:T71`;
//! - a guest other than shell: `readWithoutStrict` and `loads` both
//!   speak shell extracts only;
//! - a body whose first line is `set -euo pipefail`: nix-shebang's
//!   `stripStrict` drops that line under a shebang whether the prelude
//!   wrote it or the body did, and `rewrite` does not see the prelude;
//! - a path nix cannot write as a relative path literal, or `loads`
//!   would not read back.
//!
//! Where an argument goes, `rewrite` adds parentheses and `inline` takes
//! them away again. It never takes away an author's: a string inside
//! parentheses is no sink value, so no site, and never rewritten.

use std::path::Path;

use rnix::{SyntaxKind, SyntaxNode};
use xenolith_lang_api::{DelimKind, Error, LangId, LoadRef, Result, Site, Span};

use crate::{escape, loads, parse, sinks, span, unescape};

#[cfg(test)]
mod tests;

/// The load a site with a prelude gets (`languages/nix:V53`). Every shell
/// prelude carries a shebang, and this is the load that strips it.
const LOAD: &str = "nix-shebang.lib.readWithoutStrict";

/// The strict line nix-shebang's `stripStrict` removes below a shebang.
const STRICT: &str = "set -euo pipefail";

/// A refusal naming what was refused.
const fn refuse(operation: &'static str) -> Error {
    Error::unsupported(LangId::Nix, operation)
}

/// `src` with `site`'s string replaced by a load of `path`.
///
/// # Errors
///
/// [`Error::Parse`] when `src` does not parse or holds no such site, and
/// [`Error::Unsupported`] for every refusal in the module doc.
pub(crate) fn rewrite(src: &str, site: &Site, path: &Path) -> Result<String> {
    let root = parse(src)?;
    let (string, sink) = root
        .descendants()
        .filter(|node| node.kind() == SyntaxKind::NODE_STRING)
        .filter(|node| {
            span(node.text_range()) == Span::new(site.delim.open.start, site.delim.close.end)
        })
        .find_map(|node| {
            let sink = sinks::classify(&node)?;
            (crate::site(&node, sink).as_ref() == Some(site)).then_some((node, sink))
        })
        .ok_or_else(|| {
            let at = site.delim.open;
            Error::parse(
                LangId::Nix,
                format!("no site `{}` at bytes {at:?}", site.sink),
            )
        })?;
    if !site.holes.is_empty() {
        return Err(refuse("rewrite of a string with holes"));
    }
    if matches!(sink, sinks::Sink::ExecStart) {
        return Err(refuse("rewrite of a systemd exec line"));
    }
    if site.guest != LangId::Shell {
        return Err(refuse("rewrite of a guest other than shell"));
    }
    let raw = site.delim.body.of(src).unwrap_or_default();
    let body = unescape::unescape(&site.delim.kind, raw)?;
    if body.split('\n').next() == Some(STRICT) {
        return Err(refuse("rewrite of a body led by `set -euo pipefail`"));
    }
    let literal = path_literal(path).ok_or(refuse("rewrite to a path nix cannot load"))?;
    let call = if needs_parens(&string) {
        format!("({LOAD} {literal})")
    } else {
        format!("{LOAD} {literal}")
    };
    let out = splice(src, span(string.text_range()), &call)?;
    let back = loads::loads(&parse(&out)?);
    if !back.iter().any(|load| load.path == Path::new(&literal)) {
        return Err(refuse("rewrite to a path nix cannot load"));
    }
    Ok(out)
}

/// `src` with `load` replaced by a string holding `body`: `''…''` for a
/// body with a line break, indented one step past the load's line, and
/// `"…"` for one without.
///
/// # Errors
///
/// [`Error::Parse`] when `src` does not parse, holds no such load, or the
/// string written would not read back as `body`.
pub(crate) fn inline(src: &str, load: &LoadRef, body: &str) -> Result<String> {
    let root = parse(src)?;
    let missing = || {
        let at = load.span;
        Error::parse(
            LangId::Nix,
            format!("no load of {} at {at:?}", load.path.display()),
        )
    };
    if !loads::loads(&root).contains(load) {
        return Err(missing());
    }
    let apply = root
        .descendants()
        .find(|node| node.kind() == SyntaxKind::NODE_APPLY && span(node.text_range()) == load.span)
        .ok_or_else(missing)?;
    let target = match apply.parent() {
        Some(paren) if paren.kind() == SyntaxKind::NODE_PAREN && needs_parens(&paren) => paren,
        _ => apply,
    };
    let at = span(target.text_range());
    let (kind, literal) = if body.contains('\n') {
        let indent = line_indent(src, at.start);
        let raw = escape::indented(body, &format!("{indent}  "), indent);
        (DelimKind::NixIndented, format!("''{raw}''"))
    } else {
        let raw = escape::double_quoted(body);
        (DelimKind::NixString, format!("\"{raw}\""))
    };
    let out = splice(src, at, &literal)?;
    if read_back(&out, at.start, &kind)? != body {
        return Err(Error::parse(
            LangId::Nix,
            "the inlined string would not read back as the body",
        ));
    }
    Ok(out)
}

/// `path` as a nix relative path literal, `./`-prefixed when it has no
/// leading `.` or `..` segment, or `None` when nix cannot write it as
/// one: absolute, not UTF-8, an empty segment, or a byte outside the
/// path-literal set (`src/extract:V83` is the engine's own guard).
fn path_literal(path: &Path) -> Option<String> {
    let text = path.to_str()?;
    let plain = text
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || "._+-/".contains(ch));
    if path.is_absolute() || !plain || text.split('/').any(str::is_empty) {
        return None;
    }
    if text.starts_with("./") || text.starts_with("../") {
        Some(text.to_owned())
    } else {
        Some(format!("./{text}"))
    }
}

/// Whether a function application standing where `node` stands needs
/// parentheses to stay one expression. Only slots that bind looser than
/// application say no: an attribute value, an operand of a binary
/// operator, the inside of parentheses or of `${…}`, a `let`, `if`,
/// `with`, `assert` or lambda body, the file itself. Everything else --
/// an argument, a list element, a select -- says yes.
fn needs_parens(node: &SyntaxNode) -> bool {
    !node.parent().is_some_and(|parent| {
        matches!(
            parent.kind(),
            SyntaxKind::NODE_ATTRPATH_VALUE
                | SyntaxKind::NODE_BIN_OP
                | SyntaxKind::NODE_PAREN
                | SyntaxKind::NODE_INTERPOL
                | SyntaxKind::NODE_LET_IN
                | SyntaxKind::NODE_IF_ELSE
                | SyntaxKind::NODE_WITH
                | SyntaxKind::NODE_ASSERT
                | SyntaxKind::NODE_LAMBDA
                | SyntaxKind::NODE_ROOT
        )
    })
}

/// The spaces that indent the line `at` sits on. Spaces only: nix does
/// not strip a tab, so a tab here would become part of the body.
fn line_indent(src: &str, at: usize) -> &str {
    let start = src
        .get(..at)
        .and_then(|before| before.rfind('\n'))
        .map_or(0, |newline| newline + 1);
    let line = src.get(start..at).unwrap_or_default();
    let spaces = line.len() - line.trim_start_matches(' ').len();
    line.get(..spaces).unwrap_or_default()
}

/// The body of the string of `kind` starting at byte `at` of `src`, as
/// `unescape` reads it.
fn read_back(src: &str, at: usize, kind: &DelimKind) -> Result<String> {
    let root = parse(src)?;
    let unreadable = || Error::parse(LangId::Nix, "the inlined string is not one string");
    let string = root
        .descendants()
        .find(|node| node.kind() == SyntaxKind::NODE_STRING && span(node.text_range()).start == at)
        .ok_or_else(unreadable)?;
    if string
        .children()
        .any(|c| c.kind() == SyntaxKind::NODE_INTERPOL)
    {
        return Err(unreadable());
    }
    let (quote, open) = if *kind == DelimKind::NixIndented {
        (2, "''")
    } else {
        (1, "\"")
    };
    let text = string.text().to_string();
    if !text.starts_with(open) {
        return Err(unreadable());
    }
    let raw = text
        .get(quote..text.len().saturating_sub(quote))
        .ok_or_else(unreadable)?;
    unescape::unescape(kind, raw)
}

/// `src` with `[at.start, at.end)` replaced by `with`.
fn splice(src: &str, at: Span, with: &str) -> Result<String> {
    match (src.get(..at.start), src.get(at.end..)) {
        (Some(before), Some(after)) => Ok(format!("{before}{with}{after}")),
        _ => Err(Error::parse(LangId::Nix, "span outside the source")),
    }
}
