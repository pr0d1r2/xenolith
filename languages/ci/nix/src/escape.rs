//! Bodies back into nix strings: the inverse of [`crate::unescape`]
//! (`languages/api/src/lens:V39`, `languages/ci/nix:V170`).
//!
//! `unescape(escape(body)) == body`, byte for byte, for every body -- and
//! the text written must also be a string the GRAMMAR reads that way,
//! which is why every `''` this module emits is one nix's lexer takes as
//! an escape and nothing else.
//!
//! A `''…''` body is written one line per line, each behind the indent it
//! is given, and the dedent nix applies takes exactly that indent off
//! again. Three shapes would not survive that, and each gets one escaped
//! space `''\ `, which nix counts as content (`crate::unescape`):
//!
//! - every non-empty line starts with a space: the first line's first
//!   space, or the dedent would eat the body's own indent;
//! - the body runs into the closing `''` on a line of spaces only: that
//!   line's last space, or nix drops the line as layout;
//!
//! and a lone `'` right before anything that starts with `''` -- another
//! escape, or the closing delimiter -- is written `''\'`, or the lexer
//! reads three quotes where there were one and two.

use xenolith_lang_api::{DelimKind, Error, LangId, Result};

#[cfg(test)]
mod tests;

/// `body` as it must sit between the delimiters of `kind`.
///
/// # Errors
///
/// [`Error::Unsupported`] for a delimiter nix does not write.
pub(crate) fn escape(kind: &DelimKind, body: &str) -> Result<String> {
    match kind {
        DelimKind::NixIndented => Ok(indented(body, "", "")),
        DelimKind::NixString => Ok(double_quoted(body)),
        _ => Err(Error::unsupported(LangId::Nix, "escape")),
    }
}

/// The inside of a `''…''` string reading back as `body`: an opening line
/// break, every non-empty line behind `indent`, and -- when the body ends
/// in a line break or is empty -- a closing line of `close`.
pub(crate) fn indented(body: &str, indent: &str, close: &str) -> String {
    let lines: Vec<&str> = body.split('\n').collect();
    let open_end = !(body.is_empty() || body.ends_with('\n'));
    let lead = lead_line(&lines);
    let mut out = String::from("\n");
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        if line.is_empty() {
            continue;
        }
        let last = index + 1 == lines.len();
        let trailing = last && open_end && line.chars().all(|c| c == ' ');
        out.push_str(indent);
        out.push_str(&indented_line(
            line,
            lead == Some(index),
            trailing,
            last && open_end,
        ));
    }
    if !open_end {
        out.push_str(close);
    }
    out
}

/// The line whose first space must be escaped: the first non-empty one,
/// when every non-empty line starts with a space.
fn lead_line(lines: &[&str]) -> Option<usize> {
    let mut non_empty = lines.iter().enumerate().filter(|(_, l)| !l.is_empty());
    let first = non_empty.clone().next()?.0;
    non_empty
        .all(|(_, line)| line.starts_with(' '))
        .then_some(first)
}

/// One line of a `''…''` body, encoded. `lead` escapes its first space,
/// `trailing` its last; `closes` says the closing `''` follows it.
fn indented_line(line: &str, lead: bool, trailing: bool, closes: bool) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut tokens: Vec<String> = Vec::with_capacity(chars.len());
    let mut at = 0;
    while let Some(&ch) = chars.get(at) {
        let next = chars.get(at + 1).copied();
        let escaped_space = ch == ' ' && ((lead && at == 0) || (trailing && next.is_none()));
        let token = match ch {
            _ if escaped_space => "''\\ ".to_owned(),
            '\'' if next == Some('\'') => {
                at += 1;
                "'''".to_owned()
            }
            '$' if next == Some('{') => "''$".to_owned(),
            '\r' => "''\\r".to_owned(),
            other => other.to_string(),
        };
        tokens.push(token);
        at += 1;
    }
    let mut out = String::with_capacity(line.len());
    for (index, token) in tokens.iter().enumerate() {
        let before_quotes = tokens
            .get(index + 1)
            .map_or(closes, |next| next.starts_with("''"));
        if token == "'" && before_quotes {
            out.push_str("''\\'");
        } else {
            out.push_str(token);
        }
    }
    out
}

/// The inside of a `"…"` string reading back as `body`: `\`, `"` and `${`
/// escaped, line breaks written as `\n` so the string stays on one line.
pub(crate) fn double_quoted(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '$' if chars.peek() == Some(&'{') => out.push_str("\\$"),
            other => out.push(other),
        }
    }
    out
}
