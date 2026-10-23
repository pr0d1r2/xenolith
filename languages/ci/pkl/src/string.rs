//! Pkl multi-line strings, both directions.
//!
//! [`unescape`] turns the raw text between `"""` delimiters into the
//! string pkl would evaluate it to, which is the body an extract holds
//! (`languages/api/src/lens:V39`). [`escape`] is the inverse under a
//! given delimiter, and [`multiline`] the inverse that picks its own: a
//! body back into a delimited literal, for `inline`.
//!
//! Pkl's rules, which is all this module encodes:
//!
//! - a line break is `\r\n`, a lone `\r` or `\n`, and each evaluates to
//!   `\n` (`languages/ci/pkl:B1`); ranges stay on the host's own bytes;
//! - the opening `"""` is followed by a line break, which is not content;
//! - the closing `"""` sits on its own line, and that line's whitespace
//!   is the indent stripped from every content line;
//! - `\` starts an escape (`\n`, `\t`, `\r`, `\"`, `\\`, `\u{...}`) or an
//!   interpolation (`\(...)`), and under `#"""` it takes as many `#` as the
//!   delimiter has before it means anything -- so `#"""` holds a literal
//!   backslash without escaping it.
//!
//! Interpolations are found by pklr's LEXER, never by scanning for
//! backslashes (`languages/api/src/site:V38`): the raw text is re-lexed
//! inside a one-line module, and the string token says where each
//! `\(...)` is -- its parentheses balanced by the lexer, strings and all.
//! Outside those, a `\` under the delimiter's guard is an escape.

use pklr::lexer::TokenKind;
use xenolith_lang_api::{Delim, DelimKind, Error, LangId, Result};

use crate::syntax;

#[cfg(test)]
mod tests;

/// The string pkl evaluates the multi-line literal `raw` to, where `raw`
/// is the text between the delimiters of `delim`.
///
/// Interpolations (`\(...)`, `\#(...)`) are kept verbatim: they are pkl, not
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
    let not_inside = || Error::parse(LangId::Pkl, "not the inside of a pkl multi-line string");
    let tokens = pklr::lexer::lex(&module).map_err(|_| not_inside())?;
    let token = tokens
        .iter()
        .find(|t| {
            matches!(
                t.kind,
                TokenKind::StringLit(_) | TokenKind::InterpolatedString(_)
            )
        })
        .ok_or_else(not_inside)?;
    // The literal must be exactly the one written: `raw` closing it early
    // leaves more tokens behind it.
    let open = prefix.len() - 3 - pounds;
    if token.offset != open || token.end + 1 != module.len() {
        return Err(not_inside());
    }
    let holes: Vec<(usize, usize)> = syntax::literal(token, &module)
        .holes
        .iter()
        .map(|h| (h.start - prefix.len(), h.end - prefix.len()))
        .collect();

    let deleted = indentation(raw, &holes)?;
    let backslash = format!("\\{guard}");
    let mut out = String::with_capacity(raw.len());
    let mut at = 0;
    while let Some(rest) = raw.get(at..).filter(|rest| !rest.is_empty()) {
        if let Some(&(_, end)) = holes.iter().find(|&&(from, _)| from == at) {
            out.push_str(raw.get(at..end).unwrap_or_default());
            at = end;
            continue;
        }
        if rest.starts_with(&backslash) {
            let len = escape_len(rest, pounds)
                .ok_or_else(|| Error::parse(LangId::Pkl, format!("unknown escape `{rest:.4}`")))?;
            out.push(decode(rest.get(..len).unwrap_or_default(), pounds)?);
            at += len;
            continue;
        }
        let ch = rest.chars().next().unwrap_or_default();
        if !deleted.iter().any(|&(from, to)| from <= at && at < to) {
            match ch {
                // The `\n` after it is this break's one `\n`.
                '\r' if raw.as_bytes().get(at + 1) == Some(&b'\n') => {}
                '\r' => out.push('\n'),
                _ => out.push(ch),
            }
        }
        at += ch.len_utf8();
    }
    Ok(out)
}

/// The length of the escape sequence `rest` starts with, `\` and guard
/// included: one character, or `u{...}`.
fn escape_len(rest: &str, pounds: usize) -> Option<usize> {
    let after = rest.get(1 + pounds..)?;
    match after.chars().next()? {
        'n' | 't' | 'r' | '"' | '\\' => Some(2 + pounds),
        'u' if after.starts_with("u{") => after.find('}').map(|close| 1 + pounds + close + 1),
        _ => None,
    }
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
    format!(
        "{guard}\"\"\"{}{indent}\"\"\"{guard}",
        lines(body, indent, pounds)
    )
}

/// The inverse of [`unescape`]: `body` as it must sit between the
/// delimiters of `delim` to evaluate back to `body`
/// (`languages/api/src/lens:V39`, `languages/ci/pkl:V171`).
///
/// The `#` count is the delimiter's, not chosen here: what the body
/// cannot hold verbatim under it is escaped instead.
///
/// # Errors
///
/// [`Error::Unsupported`] when `delim` is not a pkl multi-line string.
pub fn escape(delim: &Delim, body: &str) -> Result<String> {
    let DelimKind::PklMultiline { pounds } = delim.kind else {
        return Err(Error::unsupported(LangId::Pkl, "escape"));
    };
    Ok(lines(body, "", pounds))
}

/// The inside of a multi-line literal under `pounds` `#`: the opening
/// line break, every non-empty line of `body` behind `indent`, and the
/// line break before the closing line, whose indent the caller writes.
///
/// Escaped, under that guard, is exactly what would not read back:
/// a `\` the guard would make an escape, a `"""` the guard would make
/// the end, and a carriage return, which pkl reads as a line break
/// (`languages/ci/pkl:B1`).
fn lines(body: &str, indent: &str, pounds: usize) -> String {
    let guard = "#".repeat(pounds);
    let backslash = format!("\\{guard}");
    let closing = format!("\"\"\"{guard}");
    let (as_backslash, as_quotes, as_cr) = (
        format!("\\{guard}\\"),
        format!("\"\"\\{guard}\""),
        format!("\\{guard}r"),
    );
    let mut escaped = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(ch) = rest.chars().next() {
        let used = if rest.starts_with(&backslash) {
            escaped.push_str(&as_backslash);
            1
        } else if rest.starts_with(&closing) {
            escaped.push_str(&as_quotes);
            3
        } else if ch == '\r' {
            escaped.push_str(&as_cr);
            1
        } else {
            escaped.push(ch);
            ch.len_utf8()
        };
        rest = rest.get(used..).unwrap_or_default();
    }
    let mut out = String::from("\n");
    for (index, line) in escaped.split('\n').enumerate() {
        if index > 0 {
            out.push('\n');
        }
        if !line.is_empty() {
            out.push_str(indent);
            out.push_str(line);
        }
    }
    out.push('\n');
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
