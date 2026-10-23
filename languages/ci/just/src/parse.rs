//! A justfile parsed the way just parses it: a recursive descent over
//! just's own grammar (its GRAMMAR.md), plus the just 1.51 syntax that
//! grammar does not list yet -- `eager` and `unexport`, attributes on any
//! item, format strings -- so a file just accepts reads, and a file just
//! rejects is [`Error::Parse`] naming the line and column
//! (`languages:V78`; the engine then applies `[parse] host_errors`).
//!
//! Only what a site needs is kept ([`Justfile`]); everything else is
//! parsed to be checked and dropped. A recipe's body is read by
//! [`body`], its expressions by [`expr`].

use xenolith_lang_api::{Error, Result, Span};

use crate::lexer::{self, Kind, Lexer, Token};
use crate::recipe::{Justfile, Recipe, Settings};

mod body;
mod expr;
#[cfg(test)]
mod tests;

/// Settings whose value is a list of strings.
const LIST_SETTINGS: &[&str] = &["script-interpreter", "shell", "windows-shell"];

/// Settings whose value is one string.
const STRING_SETTINGS: &[&str] = &[
    "dotenv-command",
    "dotenv-filename",
    "dotenv-path",
    "indentation",
    "minimum-version",
    "tempdir",
    "working-directory",
];

/// Settings that are booleans, `set NAME` meaning true.
const BOOL_SETTINGS: &[&str] = &[
    "allow-duplicate-recipes",
    "allow-duplicate-variables",
    "default-list",
    "default-script",
    "dotenv-load",
    "dotenv-override",
    "dotenv-required",
    "export",
    "fallback",
    "guards",
    "ignore-comments",
    "lazy",
    "lists",
    "no-cd",
    "no-exit-message",
    "positional-arguments",
    "quiet",
    "unstable",
    "windows-powershell",
];

/// `src` as a justfile.
///
/// # Errors
///
/// [`Error::Parse`] for anything just would reject, naming where.
pub(crate) fn parse(src: &str) -> Result<Justfile> {
    let mut parser = Parser::new(src, 0, true);
    let mut out = Justfile::default();
    loop {
        let token = parser.peek()?;
        match token.kind {
            Kind::Eof => return Ok(out),
            Kind::Newline => {
                parser.bump()?;
            }
            Kind::Comment => {
                parser.bump()?;
                parser.eol()?;
            }
            _ => {
                parser.unindented(token)?;
                parser.item(&mut out)?;
            }
        }
    }
}

/// The parser's cursor: a lexer, and how deep in brackets it is.
#[derive(Debug, Clone)]
struct Parser<'s> {
    src: &'s str,
    lexer: Lexer<'s>,
    /// `(`, `[` and `{` open; inside them a line break is whitespace.
    depth: usize,
    /// Whether a line break may be skipped at all: never inside one
    /// interpolation of a recipe line.
    multiline: bool,
    /// Where the last token taken ended.
    last_end: usize,
}

impl<'s> Parser<'s> {
    fn new(src: &'s str, pos: usize, multiline: bool) -> Parser<'s> {
        Parser {
            src,
            lexer: Lexer::new(src, pos),
            depth: 0,
            multiline,
            last_end: pos,
        }
    }

    fn error(&self, at: usize, message: &str) -> Error {
        lexer::error(self.src, at, message)
    }

    /// The next token from `lexer`, past line breaks and comments while
    /// inside brackets.
    fn advance(&self, lexer: &mut Lexer<'s>) -> Result<Token> {
        loop {
            let token = lexer.next()?;
            let skip = self.multiline
                && self.depth > 0
                && matches!(token.kind, Kind::Newline | Kind::Comment);
            if !skip {
                return Ok(token);
            }
        }
    }

    fn peek(&self) -> Result<Token> {
        self.advance(&mut { self.lexer })
    }

    /// The next two tokens.
    fn peek2(&self) -> Result<(Token, Token)> {
        let mut lexer = self.lexer;
        let first = self.advance(&mut lexer)?;
        Ok((first, self.advance(&mut lexer)?))
    }

    fn bump(&mut self) -> Result<Token> {
        let mut lexer = self.lexer;
        let token = self.advance(&mut lexer)?;
        self.lexer = lexer;
        self.last_end = token.span.end;
        if token.kind == Kind::Punct {
            match token.text(self.src) {
                "(" | "[" | "{" => self.depth += 1,
                ")" | "]" | "}" => self.depth = self.depth.saturating_sub(1),
                _ => {}
            }
        }
        Ok(token)
    }

    /// Whether the next token is the punctuation or name `text`.
    fn at(&self, text: &str) -> Result<bool> {
        let token = self.peek()?;
        Ok(matches!(token.kind, Kind::Punct | Kind::Name) && token.text(self.src) == text)
    }

    fn expect(&mut self, text: &str) -> Result<Token> {
        let token = self.peek()?;
        if matches!(token.kind, Kind::Punct | Kind::Name) && token.text(self.src) == text {
            return self.bump();
        }
        Err(self.error(token.span.start, &format!("expected `{text}`")))
    }

    fn expect_kind(&mut self, kind: Kind, what: &str) -> Result<Token> {
        let token = self.peek()?;
        if token.kind == kind {
            return self.bump();
        }
        Err(self.error(token.span.start, &format!("expected {what}")))
    }

    /// The end of an item's line: a comment, then a line break or EOF.
    fn eol(&mut self) -> Result<()> {
        if self.peek()?.kind == Kind::Comment {
            self.bump()?;
        }
        let token = self.peek()?;
        match token.kind {
            Kind::Newline => self.bump().map(|_| ()),
            Kind::Eof => Ok(()),
            _ => Err(self.error(token.span.start, "expected the end of the line")),
        }
    }

    /// An item must start its line: only a recipe body is indented.
    fn unindented(&self, token: Token) -> Result<()> {
        let before = self.src.get(..token.span.start).unwrap_or_default();
        let column = before.len() - before.rfind('\n').map_or(0, |nl| nl + 1);
        if column == 0 {
            Ok(())
        } else {
            Err(self.error(token.span.start, "unexpected indentation"))
        }
    }

    /// One top-level item, with the attributes before it.
    fn item(&mut self, out: &mut Justfile) -> Result<()> {
        let mut attributes = Vec::new();
        while self.at("[")? {
            attributes.extend(self.attributes()?);
            while self.peek()?.kind == Kind::Newline {
                self.bump()?;
            }
        }
        let (first, second) = self.peek2()?;
        if first.kind == Kind::Punct && first.text(self.src) == "@" {
            return self.recipe(out, &attributes);
        }
        if first.kind != Kind::Name {
            return Err(self.error(first.span.start, "expected an item"));
        }
        let next = (second.kind, second.text(self.src));
        match (first.text(self.src), next) {
            ("set", (Kind::Name, _)) => self.setting(&mut out.settings),
            ("alias", (Kind::Name, _)) => self.alias(),
            ("export" | "eager", (Kind::Name, _)) => {
                self.bump()?;
                self.assignment()
            }
            ("unexport", (Kind::Name, _)) => {
                self.bump()?;
                self.bump()?;
                self.eol()
            }
            (
                "import",
                (Kind::Punct, "?") | (Kind::Str | Kind::Newline | Kind::Comment | Kind::Eof, _),
            ) => {
                out.settings.imports = true;
                self.import()
            }
            ("mod", (Kind::Punct, "?") | (Kind::Name, _)) => self.module(),
            (_, (Kind::Punct, ":=")) => self.assignment(),
            (_, (Kind::Punct, "(")) => self.function(),
            _ => self.recipe(out, &attributes),
        }
    }

    /// `NAME := expression`.
    fn assignment(&mut self) -> Result<()> {
        self.expect_kind(Kind::Name, "a name")?;
        self.expect(":=")?;
        self.expression()?;
        self.eol()
    }

    /// `NAME(params) := expression`.
    fn function(&mut self) -> Result<()> {
        self.bump()?;
        self.expect("(")?;
        while !self.at(")")? {
            self.expect_kind(Kind::Name, "a parameter")?;
            if !self.at(")")? {
                self.expect(",")?;
            }
        }
        self.expect(")")?;
        self.expect(":=")?;
        self.expression()?;
        self.eol()
    }

    /// `alias NAME := target`.
    fn alias(&mut self) -> Result<()> {
        self.bump()?;
        self.expect_kind(Kind::Name, "an alias name")?;
        self.expect(":=")?;
        self.target()?;
        self.eol()
    }

    /// `NAME (:: NAME)*`.
    fn target(&mut self) -> Result<()> {
        self.expect_kind(Kind::Name, "a recipe name")?;
        while self.at("::")? {
            self.bump()?;
            self.expect_kind(Kind::Name, "a recipe name")?;
        }
        Ok(())
    }

    /// `import ?? string?`.
    fn import(&mut self) -> Result<()> {
        self.bump()?;
        if self.at("?")? {
            self.bump()?;
        }
        if self.peek()?.kind == Kind::Str {
            self.bump()?;
        }
        self.eol()
    }

    /// `mod ?? NAME string?`.
    fn module(&mut self) -> Result<()> {
        self.bump()?;
        if self.at("?")? {
            self.bump()?;
        }
        self.expect_kind(Kind::Name, "a module name")?;
        if self.peek()?.kind == Kind::Str {
            self.bump()?;
        }
        self.eol()
    }

    /// `set NAME ...`, its value checked against the setting's type.
    fn setting(&mut self, out: &mut Settings) -> Result<()> {
        self.bump()?;
        let name = self.expect_kind(Kind::Name, "a setting")?.text(self.src);
        if LIST_SETTINGS.contains(&name) {
            self.expect(":=")?;
            let strings = self.string_list()?;
            match name {
                "shell" => out.shell = Some(strings),
                "windows-shell" => out.windows_shell = true,
                _ => {}
            }
        } else if STRING_SETTINGS.contains(&name) {
            self.expect(":=")?;
            self.expect_kind(Kind::Str, "a string")?;
        } else if BOOL_SETTINGS.contains(&name) {
            let on = self.boolean()?;
            match name {
                "windows-powershell" => out.windows_shell |= on,
                "positional-arguments" => out.positional = on,
                _ => {}
            }
        } else if self.at(":=")? {
            // A setting newer than this list: its value is still read.
            self.bump()?;
            self.expression()?;
        }
        self.eol()
    }

    /// `(:= (true|false))?`, absent meaning true.
    fn boolean(&mut self) -> Result<bool> {
        if !self.at(":=")? {
            return Ok(true);
        }
        self.bump()?;
        let token = self.expect_kind(Kind::Name, "true or false")?;
        match token.text(self.src) {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(self.error(token.span.start, "expected true or false")),
        }
    }

    /// `[ string (, string)* ,? ]`, each string as written.
    fn string_list(&mut self) -> Result<Vec<String>> {
        self.expect("[")?;
        let mut out = vec![
            self.expect_kind(Kind::Str, "a string")?
                .text(self.src)
                .to_owned(),
        ];
        while self.at(",")? {
            self.bump()?;
            if self.at("]")? {
                break;
            }
            out.push(
                self.expect_kind(Kind::Str, "a string")?
                    .text(self.src)
                    .to_owned(),
            );
        }
        self.expect("]")?;
        Ok(out)
    }

    /// `[ attribute (, attribute)* ]` and its line's end: the names.
    fn attributes(&mut self) -> Result<Vec<String>> {
        self.expect("[")?;
        let mut names = Vec::new();
        loop {
            names.push(
                self.expect_kind(Kind::Name, "an attribute")?
                    .text(self.src)
                    .to_owned(),
            );
            if self.at(":")? {
                self.bump()?;
                self.expect_kind(Kind::Str, "a string")?;
            } else if self.at("(")? {
                self.bump()?;
                while !self.at(")")? {
                    if self.peek()?.kind == Kind::Name {
                        self.bump()?;
                        self.expect("=")?;
                    }
                    self.expect_kind(Kind::Str, "a string")?;
                    if !self.at(")")? {
                        self.expect(",")?;
                    }
                }
                self.bump()?;
            }
            if !self.at(",")? {
                break;
            }
            self.bump()?;
        }
        self.expect("]")?;
        self.eol()?;
        Ok(names)
    }

    /// `@? NAME parameter* : dependencies eol body?`.
    fn recipe(&mut self, out: &mut Justfile, attributes: &[String]) -> Result<()> {
        let start = self.peek()?.span.start;
        if self.at("@")? {
            self.bump()?;
        }
        let name = self
            .expect_kind(Kind::Name, "a recipe name")?
            .text(self.src)
            .to_owned();
        let params = self.parameters()?;
        self.expect(":")?;
        self.dependencies()?;
        let header = Span::new(start, self.last_end);
        self.eol()?;
        let (shebang, lines, resume) = body::read(self.src, self.lexer.pos)?;
        self.lexer.pos = resume;
        let has = |attr: &str| attributes.iter().any(|a| a == attr);
        out.recipes.push(Recipe {
            name,
            header,
            shebang,
            lines,
            params,
            script: has("script"),
            positional: has("positional-arguments"),
        });
        Ok(())
    }

    /// `parameter* variadic?`: whether there was any.
    fn parameters(&mut self) -> Result<bool> {
        let mut any = false;
        loop {
            if self.at("*")? || self.at("+")? {
                self.bump()?;
            } else if !self.at("$")? && self.peek()?.kind != Kind::Name {
                return Ok(any);
            }
            if self.at("$")? {
                self.bump()?;
            }
            self.expect_kind(Kind::Name, "a parameter")?;
            if self.at("=")? {
                self.bump()?;
                self.value()?;
            }
            any = true;
        }
    }

    /// `dependency* (&& dependency+)?`.
    fn dependencies(&mut self) -> Result<()> {
        loop {
            let token = self.peek()?;
            match (token.kind, token.text(self.src)) {
                (Kind::Name, _) => self.target()?,
                (Kind::Punct, "&&") => {
                    self.bump()?;
                }
                (Kind::Punct, "*" | "(") => {
                    if self.at("*")? {
                        self.bump()?;
                    }
                    self.expect("(")?;
                    self.target()?;
                    while !self.at(")")? {
                        if self.at("*")? {
                            self.bump()?;
                            self.value()?;
                        } else {
                            self.expression()?;
                        }
                    }
                    self.expect(")")?;
                }
                _ => return Ok(()),
            }
        }
    }
}
