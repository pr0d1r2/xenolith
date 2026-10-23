//! A pkl file as far as the host reads it: whether Pkl accepts it, and the
//! object tree its hk steps live in, every node with its byte span.
//!
//! Validity is `pklr`'s (a pure-Rust Pkl parser, built here without its
//! evaluator), which agrees with Pkl 0.31.1 and with hk 2.x: a file it
//! rejects is [`Error::Parse`] and none of its sites are read
//! (`languages:V78`). pklr's tree carries no positions, so the tree here
//! is built from pklr's TOKENS, which do: objects, properties, `["key"]`
//! entries and string values, with every other expression skipped by
//! [`expr`] and kept only for the objects inside it.

use pklr::lexer::{Token, TokenKind};
use xenolith_lang_api::{Error, LangId, Result, Span};

mod expr;
#[cfg(test)]
mod tests;

/// A pkl module.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Module {
    /// The `amends` URI, when it is a plain string.
    pub amends: Option<String>,
    /// The module's members.
    pub body: Body,
}

/// The members of an object, a module or a generator.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Body {
    /// Its members, in source order.
    pub members: Vec<Member>,
}

/// One member of a body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Member {
    /// `name = value` or `name { ... }`.
    Property(Property),
    /// `[key] = value` or `[key] { ... }`.
    Entry(Entry),
    /// Anything else: the objects inside it.
    Other(Vec<Body>),
}

/// A property.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Property {
    /// Its name.
    pub name: String,
    /// Declared `local`.
    pub local: bool,
    /// Its value.
    pub value: Value,
}

/// An entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    /// The key, when it is a plain non-empty string.
    pub key: Option<String>,
    /// Objects written inside the key expression.
    pub inside_key: Vec<Body>,
    /// Its value.
    pub value: Value,
}

/// A member's value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Value {
    /// One string literal and nothing else.
    Str(Str),
    /// An object written in place: an amending `name { ... }` or `[key] { ... }`.
    Object(Body),
    /// `new T { ... }`.
    New(Body),
    /// Any other expression: the objects inside it.
    Other(Vec<Body>),
}

/// A string literal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Str {
    /// The whole literal, delimiters included.
    pub span: Span,
    /// `#"""`, `"`, ...
    pub open: Span,
    /// `"""#`, `"`, ...
    pub close: Span,
    /// How many `#` guard it.
    pub pounds: usize,
    /// `"""` rather than `"`.
    pub multiline: bool,
    /// The content of a one-line literal with no escape and no
    /// interpolation, unless empty.
    pub plain: Option<String>,
    /// Each `\(...)` (or `\#(...)`), backslash to closing parenthesis.
    pub holes: Vec<Span>,
}

impl Str {
    /// The bytes between the delimiters.
    pub(crate) fn body(&self) -> Span {
        Span::new(self.open.end, self.close.start)
    }
}

#[cfg(test)]
impl Body {
    /// `f` on this body and every body nested in it, in source order.
    pub(crate) fn visit(&self, f: &mut impl FnMut(&Body)) {
        f(self);
        for member in &self.members {
            match member {
                Member::Property(Property { value, .. }) => value.visit(f),
                Member::Entry(Entry {
                    inside_key, value, ..
                }) => {
                    for body in inside_key {
                        body.visit(f);
                    }
                    value.visit(f);
                }
                Member::Other(bodies) => bodies.iter().for_each(|b| b.visit(f)),
            }
        }
    }
}

#[cfg(test)]
impl Value {
    fn visit(&self, f: &mut impl FnMut(&Body)) {
        match self {
            Value::Str(_) => {}
            Value::Object(body) | Value::New(body) => body.visit(f),
            Value::Other(bodies) => bodies.iter().for_each(|b| b.visit(f)),
        }
    }
}

/// `src`, checked by pklr and read into a tree.
///
/// # Errors
///
/// [`Error::Parse`] when Pkl would reject the file.
pub(crate) fn parse(src: &str) -> Result<Module> {
    let refuse = |e: pklr::Error| Error::parse(LangId::Pkl, e.to_string());
    let tokens = pklr::lexer::lex_named(src, "pkl").map_err(refuse)?;
    pklr::parser::parse_named(&tokens, src, "pkl").map_err(refuse)?;
    let mut reader = Reader {
        src,
        tokens: &tokens,
        at: 0,
    };
    let amends = reader.header();
    let body = reader.members(false);
    Ok(Module { amends, body })
}

/// The string literal `token` is, read off its raw bytes.
pub(crate) fn literal(token: &Token, src: &str) -> Str {
    let raw = src.get(token.offset..token.end).unwrap_or_default();
    let pounds = raw.chars().take_while(|c| *c == '#').count();
    let multiline = raw.get(pounds..).is_some_and(|r| r.starts_with("\"\"\""));
    let quotes = if multiline { 3 } else { 1 };
    let open = Span::new(token.offset, token.offset + pounds + quotes);
    let close = Span::new(token.end.saturating_sub(pounds + quotes), token.end);
    let content = src.get(open.end..close.start).unwrap_or_default();
    let guard = format!("\\{}", "#".repeat(pounds));
    let holes = match &token.kind {
        TokenKind::InterpolatedString(parts) => holes(parts, src, &guard, open.end),
        _ => Vec::new(),
    };
    let plain =
        (!multiline && holes.is_empty() && !content.is_empty() && !content.contains(&guard))
            .then(|| content.to_owned());
    Str {
        span: Span::new(token.offset, token.end),
        open,
        close,
        pounds,
        multiline,
        plain,
        holes,
    }
}

/// The span of each interpolation: from the `\#(` before its first token
/// to the end of its last, which pklr makes the `)` that closes it.
fn holes(parts: &[pklr::lexer::StringPart], src: &str, guard: &str, from: usize) -> Vec<Span> {
    let opener = format!("{guard}(");
    parts
        .iter()
        .filter_map(|part| match part {
            pklr::lexer::StringPart::Tokens(tokens) => tokens.first().zip(tokens.last()),
            _ => None,
        })
        .map(|(first, last)| {
            let start = src
                .get(from..first.offset)
                .and_then(|before| before.rfind(&opener))
                .map_or(first.offset, |at| from + at);
            Span::new(start, last.end)
        })
        .collect()
}

/// A cursor over pklr's tokens.
struct Reader<'s> {
    src: &'s str,
    tokens: &'s [Token],
    at: usize,
}

impl Reader<'_> {
    fn kind(&self) -> Option<&TokenKind> {
        self.tokens.get(self.at).map(|t| &t.kind)
    }

    fn kind_at(&self, offset: usize) -> Option<&TokenKind> {
        self.tokens.get(self.at + offset).map(|t| &t.kind)
    }

    fn ident(&self) -> Option<String> {
        match self.kind() {
            Some(TokenKind::Ident(name)) => Some(name.clone()),
            _ => None,
        }
    }

    /// The `amends` URI before the first member, if any.
    fn header(&mut self) -> Option<String> {
        let at = self
            .tokens
            .iter()
            .position(|t| matches!(t.kind, TokenKind::KwAmends))?;
        let uri = self.tokens.get(at + 1)?;
        literal(uri, self.src).plain
    }

    /// Members up to the `}` closing this body (consumed) or, not
    /// `nested`, the end of the module.
    fn members(&mut self, nested: bool) -> Body {
        let mut members = Vec::new();
        loop {
            match self.kind() {
                None | Some(TokenKind::Eof) => break,
                Some(TokenKind::RBrace) if nested => {
                    self.at += 1;
                    break;
                }
                Some(
                    TokenKind::DocComment
                    | TokenKind::Comma
                    | TokenKind::Semicolon
                    | TokenKind::RBrace,
                ) => {
                    self.at += 1;
                }
                // The header: `module a.b`, `amends "..."`, `extends "..."`.
                Some(TokenKind::KwAmends | TokenKind::KwExtends | TokenKind::KwModule) => {
                    self.at += 1;
                    while matches!(
                        self.kind(),
                        Some(TokenKind::Ident(_) | TokenKind::Dot | TokenKind::StringLit(_))
                    ) {
                        self.at += 1;
                    }
                }
                Some(_) => members.push(self.member()),
            }
        }
        Body { members }
    }

    /// One member.
    fn member(&mut self) -> Member {
        let mut local = false;
        while let Some(kind) = self.kind() {
            match kind {
                TokenKind::KwLocal => local = true,
                TokenKind::KwConst
                | TokenKind::KwFixed
                | TokenKind::KwHidden
                | TokenKind::KwAbstract
                | TokenKind::KwOpen
                | TokenKind::KwExternal => {}
                _ => break,
            }
            self.at += 1;
        }
        match self.kind() {
            Some(TokenKind::Ident(_)) => {
                let name = self.ident().unwrap_or_default();
                match self.kind_at(1) {
                    Some(TokenKind::Equals | TokenKind::LBrace | TokenKind::Colon) => {
                        self.at += 1;
                        let value = self.assigned();
                        Member::Property(Property { name, local, value })
                    }
                    _ => Member::Other(self.expression_bodies()),
                }
            }
            Some(TokenKind::LBracket) => {
                let (key, inside_key) = self.key();
                let value = self.assigned();
                Member::Entry(Entry {
                    key,
                    inside_key,
                    value,
                })
            }
            _ => Member::Other(self.other()),
        }
    }

    /// `[key]`: the key when it is one plain non-empty string, and the
    /// objects written inside it.
    fn key(&mut self) -> (Option<String>, Vec<Body>) {
        self.at += 1;
        let start = self.at;
        let inside = self.skip_to_close();
        let inner = self
            .tokens
            .get(start..self.at.saturating_sub(1))
            .unwrap_or_default();
        let key = match inner {
            [one] if matches!(one.kind, TokenKind::StringLit(_)) => literal(one, self.src).plain,
            _ => None,
        };
        (key, inside)
    }

    /// After a name or key: `: Type`, then `= value` or `{ body }`.
    fn assigned(&mut self) -> Value {
        if matches!(self.kind(), Some(TokenKind::Colon)) {
            self.skip_type();
        }
        match self.kind() {
            Some(TokenKind::Equals) => {
                self.at += 1;
                self.value()
            }
            Some(TokenKind::LBrace) => {
                self.at += 1;
                let body = self.members(true);
                self.amended(Value::Object(body))
            }
            _ => Value::Other(Vec::new()),
        }
    }
}
