//! A recipe's body, lexed the way just lexes it: indented lines after the
//! header, each read as text with an optional `@`/`-` prefix and `{{...}}`
//! interpolations; a `#!` first line makes the body one script.
//!
//! The body's indent is its first line's leading whitespace; every later
//! non-blank line starts with that same indent (anything else is an
//! error, as in just), and the first unindented non-blank line ends the
//! body. Blank lines are skipped.
//!
//! Interpolations follow just's lexer exactly (`languages/ci/just:B2`):
//! `{{{{` is a literal `{{`, and `{{` opens an expression that runs to
//! its `}}` -- an expression, so a `}}` inside a string or a format
//! string does not close it.

use xenolith_lang_api::{Result, Span};

use super::Parser;
use crate::lexer;
use crate::recipe::Line;

#[cfg(test)]
mod tests;

/// Line prefixes, longest first.
const PREFIXES: &[&str] = &["@-", "-@", "@", "-"];

/// The body starting at byte `start` (the line after the header): its
/// `#!` line, its lines, and where the next item starts.
pub(super) fn read(src: &str, start: usize) -> Result<(Option<Line>, Vec<Line>, usize)> {
    let mut indent: Option<&str> = None;
    let mut shebang = None;
    let mut lines = Vec::new();
    let mut pos = start;
    while pos < src.len() {
        let end = src
            .get(pos..)
            .and_then(|rest| rest.find('\n'))
            .map_or(src.len(), |i| pos + i);
        let raw = src.get(pos..end).unwrap_or_default();
        let content = raw.strip_suffix('\r').unwrap_or(raw);
        let next = (end + 1).min(src.len());
        if content.trim().is_empty() {
            pos = next;
            continue;
        }
        let lead = content.len() - content.trim_start_matches([' ', '\t']).len();
        if lead == 0 {
            break;
        }
        let body_indent = *indent.get_or_insert_with(|| content.get(..lead).unwrap_or_default());
        if !content.starts_with(body_indent) {
            return Err(lexer::error(
                src,
                pos,
                "inconsistent indentation in a recipe body",
            ));
        }
        let text = Span::new(pos + body_indent.len(), pos + content.len());
        let first = shebang.is_none() && lines.is_empty();
        if first && text.of(src).is_some_and(|t| t.starts_with("#!")) {
            shebang = Some(Line {
                span: text,
                prefix: Span::new(text.start, text.start),
                holes: Vec::new(),
            });
        } else {
            lines.push(line(src, text)?);
        }
        pos = next;
    }
    Ok((shebang, lines, pos))
}

/// One body line: its prefix and its interpolations.
fn line(src: &str, text: Span) -> Result<Line> {
    let written = text.of(src).unwrap_or_default();
    let prefix_len = PREFIXES
        .iter()
        .find(|p| written.starts_with(**p))
        .map_or(0, |p| p.len());
    let bytes = src.as_bytes();
    let opens =
        |at: usize, n: usize| (at..at + n).all(|i| i < text.end && bytes.get(i) == Some(&b'{'));
    let mut holes = Vec::new();
    let mut at = text.start + prefix_len;
    while at < text.end {
        if opens(at, 4) {
            at += 4;
        } else if opens(at, 2) {
            let end = interpolation(src, at, text.end)?;
            holes.push(Span::new(at, end));
            at = end;
        } else {
            at += 1;
        }
    }
    Ok(Line {
        span: text,
        prefix: Span::new(text.start, text.start + prefix_len),
        holes,
    })
}

/// The end of the interpolation whose `{{` is at `open`: past the `}}`
/// that closes its expression, which must come before `limit`.
fn interpolation(src: &str, open: usize, limit: usize) -> Result<usize> {
    let mut parser = Parser::new(src, open + 2, false);
    parser.expression()?;
    let after = parser.last_end;
    let rest = src.get(after..limit).unwrap_or_default();
    let close = after + rest.len() - rest.trim_start_matches([' ', '\t']).len();
    if src.get(close..limit).is_some_and(|r| r.starts_with("}}")) {
        Ok(close + 2)
    } else {
        Err(lexer::error(src, open, "unterminated interpolation"))
    }
}
