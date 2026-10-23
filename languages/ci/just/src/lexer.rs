//! just's tokens, as its GRAMMAR.md names them, each with its byte span.
//!
//! The lexer is driven by the parser, one token at a time from wherever
//! the parser puts it, because just's tokenizer is "mildly context
//! sensitive": a recipe body is lexed as lines of text with `{{...}}`
//! interpolations, which [`crate::parse`] reads off the source itself and
//! hands back here only for the expression inside each interpolation.
//!
//! Strings are lexed whole, delimiters and prefix included: `'raw'`,
//! `"cooked"` (escapes `\n \r \t \" \\ \u{...}` and a backslash-newline;
//! any other escape is an error, as in just), their `'''`/`"""` indented
//! forms, and the `x` (shell-expanded) and `f` (format) prefixes. A
//! format string's `{{...}}` stay inside the token.

use xenolith_lang_api::{Error, LangId, Result, Span};

#[cfg(test)]
mod tests;

/// What a token is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// `[a-zA-Z_][a-zA-Z0-9_-]*`.
    Name,
    /// A string of any kind, delimiters and prefix included.
    Str,
    /// `` `...` `` or ```` ```...``` ````.
    Backtick,
    /// An operator or punctuation mark.
    Punct,
    /// `\n` or `\r\n`.
    Newline,
    /// `#` to the end of the line.
    Comment,
    /// The end of the text.
    Eof,
}

/// One token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Token {
    /// What it is.
    pub kind: Kind,
    /// Where it is.
    pub span: Span,
}

impl Token {
    /// The token's text in `src`.
    pub(crate) fn text<'s>(&self, src: &'s str) -> &'s str {
        self.span.of(src).unwrap_or_default()
    }
}

/// Multi-character operators, longest first where one prefixes another.
const OPERATORS: &[&str] = &[
    ":=", "::", "==", "!=", "=~", "!~", "&&", "||", "++", "(", ")", "[", "]", "{", "}", ",", ":",
    "+", "/", "!", "=", "*", "$", "@", "?",
];

/// A cursor over the source.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Lexer<'s> {
    src: &'s str,
    /// The next byte to read.
    pub pos: usize,
}

/// An error at byte `at` of `src`, named by its line and column.
pub(crate) fn error(src: &str, at: usize, message: &str) -> Error {
    let before = src.get(..at).unwrap_or(src);
    let line = before.matches('\n').count() + 1;
    let col = before.len() - before.rfind('\n').map_or(0, |nl| nl + 1) + 1;
    Error::parse(LangId::Just, format!("{line}:{col}: {message}"))
}

fn name_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn name_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
}

impl<'s> Lexer<'s> {
    /// A lexer over `src` starting at byte `pos`.
    pub(crate) fn new(src: &'s str, pos: usize) -> Lexer<'s> {
        Lexer { src, pos }
    }

    fn rest(&self) -> &'s str {
        self.src.get(self.pos..).unwrap_or_default()
    }

    fn token(&mut self, kind: Kind, len: usize) -> Token {
        let start = self.pos;
        self.pos += len;
        Token {
            kind,
            span: Span::new(start, self.pos),
        }
    }

    /// The next token, past spaces and tabs.
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] for a character just does not lex, an
    /// unterminated string or backtick, or an unknown escape.
    pub(crate) fn next(&mut self) -> Result<Token> {
        let skipped = self.rest().len() - self.rest().trim_start_matches([' ', '\t']).len();
        self.pos += skipped;
        let rest = self.rest();
        let Some(&first) = rest.as_bytes().first() else {
            return Ok(self.token(Kind::Eof, 0));
        };
        match first {
            b'\n' => Ok(self.token(Kind::Newline, 1)),
            b'\r' if rest.starts_with("\r\n") => Ok(self.token(Kind::Newline, 2)),
            b'#' => Ok(self.token(Kind::Comment, rest.find(['\r', '\n']).unwrap_or(rest.len()))),
            b'\'' | b'"' => self.string(0),
            b'`' => self.backtick(),
            b if name_start(b) => {
                let len = rest.bytes().take_while(|b| name_byte(*b)).count();
                let prefixed = len == 1
                    && matches!(first, b'x' | b'f')
                    && matches!(rest.as_bytes().get(1), Some(b'\'' | b'"'));
                if prefixed {
                    self.string(1)
                } else {
                    Ok(self.token(Kind::Name, len))
                }
            }
            _ => {
                if let Some(op) = OPERATORS.iter().find(|op| rest.starts_with(**op)) {
                    return Ok(self.token(Kind::Punct, op.len()));
                }
                let ch = rest.chars().next().unwrap_or_default();
                Err(error(
                    self.src,
                    self.pos,
                    &format!("unexpected character `{ch}`"),
                ))
            }
        }
    }

    /// A string whose quote is `prefix` bytes in.
    fn string(&mut self, prefix: usize) -> Result<Token> {
        let start = self.pos;
        let body = self.rest().get(prefix..).unwrap_or_default();
        let quote = if body.starts_with('"') { '"' } else { '\'' };
        let triple = if quote == '"' { "\"\"\"" } else { "'''" };
        let (open, close) = if body.starts_with(triple) {
            (3, triple)
        } else if quote == '"' {
            (1, "\"")
        } else {
            (1, "'")
        };
        let inside = body.get(open..).unwrap_or_default();
        let end = if quote == '"' {
            self.cooked(inside, close, start + prefix + open)?
        } else {
            inside.find(close)
        };
        let Some(end) = end else {
            return Err(error(self.src, start, "unterminated string"));
        };
        Ok(self.token(Kind::Str, prefix + open + end + close.len()))
    }

    /// Where `close` ends a cooked string's `inside`, past its escapes;
    /// `None` if it never does. `at` is where `inside` starts in the
    /// source, for an escape's error.
    fn cooked(&self, inside: &str, close: &str, at: usize) -> Result<Option<usize>> {
        let bytes = inside.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if inside.get(i..).is_some_and(|r| r.starts_with(close)) {
                return Ok(Some(i));
            }
            if bytes.get(i) == Some(&b'\\') {
                match bytes.get(i + 1) {
                    Some(b'n' | b'r' | b't' | b'"' | b'\\' | b'\n') => i += 2,
                    Some(b'u') if bytes.get(i + 2) == Some(&b'{') => {
                        let Some(brace) = inside.get(i..).and_then(|r| r.find('}')) else {
                            return Ok(None);
                        };
                        i += brace + 1;
                    }
                    _ => return Err(error(self.src, at + i, "invalid escape in a string")),
                }
            } else {
                i += 1;
            }
        }
        Ok(None)
    }

    fn backtick(&mut self) -> Result<Token> {
        let rest = self.rest();
        let close = if rest.starts_with("```") { "```" } else { "`" };
        let inside = rest.get(close.len()..).unwrap_or_default();
        match inside.find(close) {
            Some(end) => Ok(self.token(Kind::Backtick, 2 * close.len() + end)),
            None => Err(error(self.src, self.pos, "unterminated backtick")),
        }
    }
}
