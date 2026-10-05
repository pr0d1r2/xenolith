//! The write side of the nix host (`languages/ci/nix:V170`): `rewrite` takes
//! a site's string out and leaves the `languages/ci/nix:V53` load in its
//! place, `inline` puts a body back where a load was.
//!
//! Both answer the question the other one asks. `rewrite` writes only
//! what `loads` reads back -- a relative path literal to a shell file --
//! and checks that it does; `inline` writes a string whose body, read by
//! `unescape`, is exactly the body it was given, and checks that too. A
//! check that fails is a refusal, never a best effort: an extraction that
//! changed what the host runs is worse than none (`src/extract:V4`).
//!
//! Which load is the site's own scope's call: `readWithoutStrict` through
//! whatever name `scope` proves nix-shebang goes by there, and
//! `builtins.readFile` wherever it proves none -- a load that does not
//! evaluate is no load.
//!
//! What `rewrite` refuses, it refuses by name:
//!
//! - holes, in `rewrite`: a `${...}` copied into a shell file is shell, not
//!   nix. `rewrite_bound` takes them, with the params bind named, into a
//!   `replaceStrings` load (`languages/ci/nix:V174`, `crate::bound`);
//! - a systemd exec line: its load is `toShellScript`
//!   (`languages/ci/nix:V69`), which is `languages/ci/nix:T71`;
//! - a guest other than shell: `readWithoutStrict` and `loads` both
//!   speak shell extracts only;
//! - a `"..."` string holding a line break: `inline` writes that body back
//!   as `''...''`, and the host would not come back as it was;
//! - under `readWithoutStrict`, a body whose first line is
//!   `set -euo pipefail`: nix-shebang's `stripStrict` drops that line
//!   under a shebang whether the prelude wrote it or the body did, and
//!   `rewrite` does not see the prelude. `readFile` strips nothing;
//! - a path nix cannot write as a relative path literal, or `loads`
//!   would not read back.
//!
//! Where an argument goes, `rewrite` adds parentheses and `inline` takes
//! them away again. It never takes away an author's: a string inside
//! parentheses is no sink value, so no site, and never rewritten.

use std::path::Path;

use rnix::{SyntaxKind, SyntaxNode};
use xenolith_lang_api::holes::Param;
use xenolith_lang_api::lens::Rewrite;
use xenolith_lang_api::{DelimKind, Error, LangId, LoadRef, Result, Site, Span};

use crate::{bound, escape, loads, parse, scope, sinks, span, unescape};

#[cfg(test)]
mod tests;

/// The load a site gets where nix-shebang is in scope, after the name it
/// goes by there (`languages/ci/nix:V53`): every shell prelude carries a
/// shebang, and this is the load that strips it.
const STRIPPING: &str = "lib.readWithoutStrict";

/// The load a site gets everywhere else: it always evaluates, and the
/// prelude it keeps is a comment and a harmless `set -e`
/// (`languages/ci/nix:V53`).
const READ_FILE: &str = "builtins.readFile";

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
    let (string, sink) = find(&root, site)?;
    if !site.holes.is_empty() {
        return Err(refuse("rewrite of a string with holes"));
    }
    shell_site(site, sink)?;
    let raw = site.delim.body.of(src).unwrap_or_default();
    let body = unescape::unescape(&site.delim.kind, raw)?;
    one_line(site, &body)?;
    let load = v53_load(&string, &body, path)?;
    place(src, &string, &load, path)
}

/// [`rewrite`] for a site whose holes became `params`
/// (`languages/ci/nix:V174`): the V53 load goes inside
/// `builtins.replaceStrings`, and the extract holds `__NAME__` wherever a
/// hole was ([`bound`]). With no params it IS [`rewrite`], and `body`
/// passes through.
///
/// # Errors
///
/// As [`rewrite`], and [`Error::Unsupported`] for params that are not
/// the site's holes or a body `replaceStrings` would not give back.
pub(crate) fn rewrite_bound(
    src: &str,
    site: &Site,
    path: &Path,
    body: &str,
    params: &[Param],
) -> Result<Rewrite> {
    if params.is_empty() {
        return Ok(Rewrite {
            src: rewrite(src, site, path)?,
            body: body.to_owned(),
        });
    }
    let root = parse(src)?;
    let (string, sink) = find(&root, site)?;
    shell_site(site, sink)?;
    let (body, pairs) = bound::bind(src, site, params)?;
    one_line(site, &body)?;
    let load = v53_load(&string, &body, path)?;
    let call = bound::call(&pairs, &format!("({load})"));
    Ok(Rewrite {
        src: place(src, &string, &call, path)?,
        body,
    })
}

/// The string node of `site` in `root`, and its sink.
fn find(root: &SyntaxNode, site: &Site) -> Result<(SyntaxNode, sinks::Sink)> {
    root.descendants()
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
        })
}

/// Refuses a `"..."` body holding a line break: `inline` writes it back as
/// `''...''`, so the host would not come back as it was
/// (`languages/api/src/lens:V34` (a)).
fn one_line(site: &Site, body: &str) -> Result<()> {
    if site.delim.kind == DelimKind::NixString && body.contains('\n') {
        return Err(refuse("rewrite of a `\"...\"` string holding a line break"));
    }
    Ok(())
}

/// Refuses the sites no V53 load stands in for: an exec line, a guest
/// other than shell.
fn shell_site(site: &Site, sink: sinks::Sink) -> Result<()> {
    if matches!(sink, sinks::Sink::ExecStart) {
        return Err(refuse("rewrite of a systemd exec line"));
    }
    if site.guest != LangId::Shell {
        return Err(refuse("rewrite of a guest other than shell"));
    }
    Ok(())
}

/// The V53 load of `path` for the site at `string` whose extract holds
/// `body`: `readWithoutStrict` through the name scope proves, else
/// `readFile`.
fn v53_load(string: &SyntaxNode, body: &str, path: &Path) -> Result<String> {
    let function = match scope::nix_shebang(string) {
        Some(nix_shebang) => {
            if body.split('\n').next() == Some(STRICT) {
                return Err(refuse("rewrite of a body led by `set -euo pipefail`"));
            }
            format!("{nix_shebang}.{STRIPPING}")
        }
        None => READ_FILE.to_owned(),
    };
    let literal = path_literal(path).ok_or(refuse("rewrite to a path nix cannot load"))?;
    Ok(format!("{function} {literal}"))
}

/// `src` with `string` replaced by `call`, parenthesised where an
/// argument goes, and checked: `loads` must read back a load of `path`
/// spanning exactly `call`.
fn place(src: &str, string: &SyntaxNode, call: &str, path: &Path) -> Result<String> {
    let text = if needs_parens(string) {
        format!("({call})")
    } else {
        call.to_owned()
    };
    let out = splice(src, span(string.text_range()), &text)?;
    let literal = path_literal(path).ok_or(refuse("rewrite to a path nix cannot load"))?;
    let back = loads::loads(&parse(&out)?);
    if !back
        .iter()
        .any(|load| load.path == Path::new(&literal) && load.span.of(&out) == Some(call))
    {
        return Err(refuse("rewrite to a path nix cannot load"));
    }
    Ok(out)
}

/// `src` with `load` replaced by a string holding `body`: `''...''` for a
/// body with a line break, indented one step past the load's line, and
/// `"..."` for one without.
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
    // A `replaceStrings` load puts its holes back where the body holds
    // their patterns (`languages/ci/nix:V174`).
    let pairs = bound::parts(&apply).map(|(pairs, _)| pairs);
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
    let Some(pairs) = pairs else {
        let out = splice(src, at, &literal)?;
        if read_back(&out, at.start, &kind)? != body {
            return Err(Error::parse(
                LangId::Nix,
                "the inlined string would not read back as the body",
            ));
        }
        return Ok(out);
    };
    // The escaped body still spells each pattern as it was: `escape`
    // never touches `@`, letters, digits or `_`.
    let (patterns, holes): (Vec<String>, Vec<String>) = pairs.iter().cloned().unzip();
    let literal = bound::replace_strings(&literal, &patterns, &holes);
    let out = splice(src, at, &literal)?;
    let string = parse(&out)?
        .descendants()
        .find(|node| {
            node.kind() == SyntaxKind::NODE_STRING && span(node.text_range()).start == at.start
        })
        .ok_or_else(|| Error::parse(LangId::Nix, "the inlined string is not one string"))?;
    bound::read_back(&out, &string, &pairs, body)?;
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
/// operator, the inside of parentheses or of `${...}`, a `let`, `if`,
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
