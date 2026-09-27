//! Nix string bodies as the guest reads them
//! (`languages/api/src/lens:V39`, `languages/ci/nix:T158`).
//!
//! What bash runs is not the bytes between the quotes. For a `''…''`
//! string nix first drops an opening line of spaces, then strips the
//! common indentation of the content lines, then drops a closing line of
//! spaces; its `''` escapes decode along the way. A heredoc terminator
//! indented with the rest of the body is a terminator only after that --
//! which is why the guest must see this text, not the host's.
//!
//! The rules are nix's own `stripIndentation`, step for step:
//!
//! - only SPACES indent; a tab is content;
//! - a line of spaces only does not set the indent;
//! - an escape (`''$`, `'''`, `''\x`) ends a line's indent when the
//!   indent is measured, and is then stripped in the same pass as text;
//! - no content line at all means every leading space goes;
//! - the last-line rule looks inside the last part only.
//!
//! Holes are replaced by the engine's placeholder word before this runs,
//! and a hole is content to nix just as the word is to this module.

use xenolith_lang_api::{DelimKind, Error, LangId, Result};

#[cfg(test)]
mod tests;

/// One piece of an indented string body, as nix's lexer cuts it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Part {
    /// Plain text, which may carry indentation.
    Text(String),
    /// A decoded escape: never indentation when the indent is measured.
    Escaped(String),
}

/// `raw` as its guest reads it, for a nix delimiter.
///
/// # Errors
///
/// [`Error::Unsupported`] for a delimiter nix does not write.
pub(crate) fn unescape(kind: &DelimKind, raw: &str) -> Result<String> {
    match kind {
        DelimKind::NixIndented => Ok(indented(raw)),
        DelimKind::NixString => Ok(double_quoted(raw)),
        _ => Err(Error::unsupported(LangId::Nix, "unescape")),
    }
}

/// The body of a `''…''` string, dedented and decoded.
pub(crate) fn indented(raw: &str) -> String {
    let parts = lex_indented(drop_opening_line(raw));
    strip_indent(&parts, min_indent(&parts))
}

/// `raw` without an opening line of spaces: nix's lexer takes
/// `''` followed by spaces and a line break as the delimiter.
pub(crate) fn drop_opening_line(raw: &str) -> &str {
    let spaces = raw.len() - raw.trim_start_matches(' ').len();
    match raw.get(spaces..).and_then(|rest| rest.strip_prefix('\n')) {
        Some(rest) => rest,
        None => raw,
    }
}

/// The text and escapes of an indented string body.
///
/// `'''` is `''`, `''$` is `$`, and `''\` takes the next character:
/// `n`, `r` and `t` as the control characters, anything else as itself.
/// Every other byte is text, a lone `'` and `$${` included.
pub(crate) fn lex_indented(raw: &str) -> Vec<Part> {
    let mut parts = Vec::new();
    let mut text = String::new();
    let mut rest = raw;
    while !rest.is_empty() {
        let Some((decoded, used)) = escape_at(rest) else {
            let ch = rest.chars().next().unwrap_or_default();
            text.push(ch);
            rest = rest.get(ch.len_utf8()..).unwrap_or_default();
            continue;
        };
        if !text.is_empty() {
            parts.push(Part::Text(std::mem::take(&mut text)));
        }
        parts.push(Part::Escaped(decoded));
        rest = rest.get(used..).unwrap_or_default();
    }
    if !text.is_empty() {
        parts.push(Part::Text(text));
    }
    parts
}

/// The escape `rest` starts with, decoded, and its length in bytes.
fn escape_at(rest: &str) -> Option<(String, usize)> {
    let after = rest.strip_prefix("''")?;
    if after.starts_with('\'') {
        return Some(("''".to_owned(), 3));
    }
    if after.starts_with('$') {
        return Some(("$".to_owned(), 3));
    }
    let ch = after.strip_prefix('\\')?.chars().next()?;
    let decoded = match ch {
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        other => other,
    };
    Some((decoded.to_string(), 3 + ch.len_utf8()))
}

/// The least number of leading spaces on a line with content, or `None`
/// when no line has any.
pub(crate) fn min_indent(parts: &[Part]) -> Option<usize> {
    let mut least: Option<usize> = None;
    let mut at_start = true;
    let mut indent = 0;
    let mut content = |indent: usize| least = Some(least.map_or(indent, |l| l.min(indent)));
    for part in parts {
        match part {
            Part::Escaped(_) => {
                if at_start {
                    at_start = false;
                    content(indent);
                }
            }
            Part::Text(text) => {
                for ch in text.chars() {
                    if at_start {
                        match ch {
                            ' ' => indent += 1,
                            '\n' => indent = 0,
                            _ => {
                                at_start = false;
                                content(indent);
                            }
                        }
                    } else if ch == '\n' {
                        at_start = true;
                        indent = 0;
                    }
                }
            }
        }
    }
    least
}

/// `parts` joined, with up to `indent` leading spaces taken off every
/// line (every leading space when `None`), and a last line of spaces
/// dropped when the last part holds a line break.
pub(crate) fn strip_indent(parts: &[Part], indent: Option<usize>) -> String {
    let indent = indent.unwrap_or(usize::MAX);
    let mut out = String::new();
    let mut at_start = true;
    let mut dropped = 0;
    for (index, part) in parts.iter().enumerate() {
        let (Part::Text(text) | Part::Escaped(text)) = part;
        let mut piece = String::new();
        for ch in text.chars() {
            if at_start {
                match ch {
                    ' ' => {
                        if dropped >= indent {
                            piece.push(ch);
                        }
                        dropped += 1;
                    }
                    '\n' => {
                        dropped = 0;
                        piece.push(ch);
                    }
                    _ => {
                        at_start = false;
                        dropped = 0;
                        piece.push(ch);
                    }
                }
            } else {
                piece.push(ch);
                if ch == '\n' {
                    at_start = true;
                }
            }
        }
        if index + 1 == parts.len()
            && let Some(newline) = piece.rfind('\n')
            && piece
                .get(newline + 1..)
                .is_some_and(|tail| tail.chars().all(|c| c == ' '))
        {
            piece.truncate(newline + 1);
        }
        out.push_str(&piece);
    }
    out
}

/// The body of a `"…"` string, its backslash escapes decoded: `\n`,
/// `\r`, `\t` as the control characters, anything else as itself. No
/// indentation is stripped from a double-quoted string.
pub(crate) fn double_quoted(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}
