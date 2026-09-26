//! Pkl multi-line strings, both directions.
//!
//! [`unescape`] turns the raw text between `"""` delimiters into the
//! string pkl would evaluate it to, which is the body an extract holds
//! (`languages/api/src/lens:V39`). [`multiline`] is the inverse: a body
//! back into a delimited literal, for `inline`.
//!
//! Pkl's rules, which is all this module encodes:
//!
//! - a line break is `\r\n`, a lone `\r` or `\n`, and each evaluates to
//!   `\n` (`languages/pkl:B1`); ranges stay on the host's own bytes;
//! - the opening `"""` is followed by a line break, which is not content;
//! - the closing `"""` sits on its own line, and that line's whitespace
//!   is the indent stripped from every content line;
//! - `\` starts an escape (`\n`, `\t`, `\r`, `\"`, `\\`, `\u{…}`) or an
//!   interpolation (`\(…)`), and under `#"""` it takes as many `#` as the
//!   delimiter has before it means anything -- so `#"""` holds a literal
//!   backslash without escaping it.
//!
//! Escapes and interpolations are found by the GRAMMAR, never by
//! scanning for backslashes (`languages/api/src/site:V38`): the raw text
//! is re-parsed inside a one-line module, and the tree says which bytes
//! are an escape, which are a hole and which are plain text.

use tree_sitter::{Node, Parser};
use xenolith_lang_api::{Delim, DelimKind, Error, LangId, Result};

use crate::grammar;

#[cfg(test)]
mod tests;

/// The string pkl evaluates the multi-line literal `raw` to, where `raw`
/// is the text between the delimiters of `delim`.
///
/// Interpolations (`\(…)`, `\#(…)`) are kept verbatim: they are pkl, not
/// guest text, and resolving them is the holes machinery's job
/// (`languages/api/src/holes:V40`), not this function's.
///
/// # Errors
///
/// [`Error::Unsupported`] when `delim` is not a pkl multi-line string, and
/// [`Error::Parse`] when `raw` is not the inside of a well-formed one.
pub fn unescape(delim: &Delim, raw: &str) -> Result<String> {
    let DelimKind::PklMultiline { pounds } = delim.kind else {
        return Err(Error::unsupported(LangId::Pkl, "unescape"));
    };
    let guard = "#".repeat(pounds);
    let prefix = format!("x = {guard}\"\"\"");
    let module = format!("{prefix}{raw}\"\"\"{guard}\n");

    let mut parser = Parser::new();
    parser
        .set_language(&grammar::language())
        .map_err(|e| Error::parse(LangId::Pkl, e.to_string()))?;
    let tree = parser
        .parse(&module, None)
        .ok_or_else(|| Error::parse(LangId::Pkl, "the parser returned no tree"))?;
    let root = tree.root_node();
    if root.has_error() {
        return Err(Error::parse(
            LangId::Pkl,
            "not the inside of a pkl multi-line string",
        ));
    }
    let literal = find_kind(root, "mlStringLiteralExpr")
        .ok_or_else(|| Error::parse(LangId::Pkl, "no multi-line string"))?;

    let deleted = indentation(raw, &holes_in(literal, prefix.len()))?;
    let mut out = String::with_capacity(raw.len());
    let mut cursor = literal.walk();
    for child in literal.named_children(&mut cursor) {
        let start = child.start_byte().saturating_sub(prefix.len());
        let end = child.end_byte().saturating_sub(prefix.len());
        let text = raw
            .get(start..end)
            .ok_or_else(|| Error::parse(LangId::Pkl, "a string part is outside the body"))?;
        match child.kind() {
            "escapeSequence" => out.push(decode(text, pounds)?),
            "stringInterpolation" => out.push_str(text),
            _ => {
                for (offset, ch) in text.char_indices() {
                    let at = start + offset;
                    if deleted.iter().any(|&(from, to)| from <= at && at < to) {
                        continue;
                    }
                    match ch {
                        // The `\n` after it is this break's one `\n`.
                        '\r' if raw.as_bytes().get(at + 1) == Some(&b'\n') => {}
                        '\r' => out.push('\n'),
                        _ => out.push(ch),
                    }
                }
            }
        }
    }
    Ok(out)
}

/// `body` as a pkl multi-line literal whose content lines carry `indent`.
///
/// The delimiter takes the FEWEST `#` that let the body through
/// unescaped: none when it has no backslash and no `"""`, otherwise as
/// many as it takes for no `\` or `"""` in the body to be followed by
/// that many `#`. Escaping instead would also work, and would turn a
/// `sed 's/\t/ /'` a reader recognises into `sed 's/\\t/ /'` they have to
/// decode.
///
/// The result evaluates to exactly `body` ([`unescape`] is its inverse):
/// a trailing newline in the body becomes an empty last line, and an
/// empty line stays empty rather than gaining trailing whitespace.
#[must_use]
pub fn multiline(body: &str, indent: &str) -> String {
    let pounds = pounds_for(body);
    let guard = "#".repeat(pounds);
    let mut out = format!("{guard}\"\"\"\n");
    for (index, line) in body.split('\n').enumerate() {
        if index > 0 {
            out.push('\n');
        }
        if !line.is_empty() {
            out.push_str(indent);
            out.push_str(line);
        }
    }
    out.push('\n');
    out.push_str(indent);
    out.push_str("\"\"\"");
    out.push_str(&guard);
    out
}

/// The fewest `#` a delimiter needs for `body` to go in verbatim.
fn pounds_for(body: &str) -> usize {
    // A body of N bytes cannot hold `\` followed by N + 1 `#`, so the
    // search always ends inside this range.
    (0..=body.len() + 1)
        .find(|&pounds| {
            if pounds == 0 {
                return !body.contains('\\') && !body.contains("\"\"\"");
            }
            let guard = "#".repeat(pounds);
            !body.contains(&format!("\\{guard}")) && !body.contains(&format!("\"\"\"{guard}"))
        })
        .unwrap_or_default()
}

/// The first node of `kind` under `node`, depth first.
fn find_kind<'t>(node: Node<'t>, kind: &str) -> Option<Node<'t>> {
    if node.kind() == kind {
        return Some(node);
    }
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find_map(|child| find_kind(child, kind))
}

/// Interpolation ranges inside `literal`, relative to the raw body.
fn holes_in(literal: Node<'_>, shift: usize) -> Vec<(usize, usize)> {
    let mut cursor = literal.walk();
    literal
        .named_children(&mut cursor)
        .filter(|child| child.kind() == "stringInterpolation")
        .map(|child| {
            (
                child.start_byte().saturating_sub(shift),
                child.end_byte().saturating_sub(shift),
            )
        })
        .collect()
}

/// The line breaks of `raw` as byte ranges, as pkl reads them inside a
/// string: `\r\n`, a lone `\r` and `\n` are one break each.
fn line_breaks(raw: &str) -> Vec<(usize, usize)> {
    let bytes = raw.as_bytes();
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(&byte) = bytes.get(at) {
        let len = match (byte, bytes.get(at + 1)) {
            (b'\r', Some(b'\n')) => 2,
            (b'\r' | b'\n', _) => 1,
            _ => 0,
        };
        if len > 0 {
            out.push((at, at + len));
        }
        at += len.max(1);
    }
    out
}

/// The byte ranges of `raw` that are delimiter layout rather than
/// content: the line break after the opening `"""`, the closing line,
/// and the closing line's indent at the start of every content line.
///
/// Line breaks inside an interpolation are pkl expression text, not
/// string lines, so they are skipped.
fn indentation(raw: &str, holes: &[(usize, usize)]) -> Result<Vec<(usize, usize)>> {
    let bad = |why: &str| Error::parse(LangId::Pkl, format!("multi-line string: {why}"));
    let breaks = line_breaks(raw);
    let (Some(&(0, first)), Some(&(last, closing))) = (breaks.first(), breaks.last()) else {
        return Err(bad("content starts on the delimiter line"));
    };
    let indent = raw.get(closing..).unwrap_or_default();
    if !indent.chars().all(|ch| ch == ' ' || ch == '\t') {
        return Err(bad("the closing delimiter is not on its own line"));
    }

    let mut deleted = vec![(0, first), (last, raw.len())];
    let in_hole = |at: usize| holes.iter().any(|&(from, to)| from <= at && at < to);
    for (index, &(newline, start)) in breaks.iter().enumerate() {
        if newline >= last || in_hole(newline) {
            continue;
        }
        let end = breaks.get(index + 1).map_or(raw.len(), |&(next, _)| next);
        let line = raw.get(start..end).unwrap_or_default();
        if line.starts_with(indent) {
            deleted.push((start, start + indent.len()));
        } else if line.chars().all(|ch| ch == ' ' || ch == '\t') {
            deleted.push((start, end));
        } else {
            return Err(bad("a line is indented less than the closing delimiter"));
        }
    }
    Ok(deleted)
}

/// The character an escape sequence stands for, `\` and guard included.
fn decode(text: &str, pounds: usize) -> Result<char> {
    let bad = || Error::parse(LangId::Pkl, format!("unknown escape `{text}`"));
    let rest = text
        .strip_prefix('\\')
        .and_then(|rest| rest.get(pounds..))
        .ok_or_else(bad)?;
    match rest {
        "n" => Ok('\n'),
        "t" => Ok('\t'),
        "r" => Ok('\r'),
        "\"" => Ok('"'),
        "\\" => Ok('\\'),
        _ => rest
            .strip_prefix("u{")
            .and_then(|hex| hex.strip_suffix('}'))
            .and_then(|hex| u32::from_str_radix(hex, 16).ok())
            .and_then(char::from_u32)
            .ok_or_else(bad),
    }
}
