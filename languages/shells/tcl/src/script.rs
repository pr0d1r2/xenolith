//! Tcl's word syntax as a tree: commands, their words, and the
//! substitutions inside each word, every one with its byte span.
//!
//! Tcl has almost no grammar. A script is commands separated by newlines
//! and `;`; a command is words; a word is braced (`{...}`, literal),
//! quoted (`"..."`, substituting) or bare (substituting), optionally
//! behind the `{*}` expansion prefix; substitutions are `\x` escapes,
//! `$name` / `${name}` / `$name(index)` and `[script]`. Everything else --
//! `if`, `proc`, `regexp`, `expr` -- is an ordinary command whose words
//! some command later reads as a script or an expression. So this module
//! parses words and nothing else, and which braced word is a script is
//! the host's to decide (`languages/shells/tcl` §I).
//!
//! A parenthesis means nothing outside a variable's index, so `{(a)}`,
//! `a(b)` and `(a)` are plain text here (`languages/shells/tcl:B1`).
//!
//! Validity is not decided twice. Every text is first put through
//! [`syntax::check`], the Rust reading of Tcl's own parser that is
//! fuzzed against `tclsh` (`languages/shells/tcl:R208`), and only text it
//! accepts is turned into a tree. A text it rejects is
//! [`Error::Parse`], naming the line and column in the WHOLE source.

use xenolith_lang_api::{Error, LangId, Result, Span};

use crate::syntax;

mod expression;
#[cfg(test)]
mod tests;

pub use self::expression::expression;

/// Commands, in order. Comments and empty commands are not kept.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Script {
    /// The commands of the script.
    pub commands: Vec<Command>,
}

/// One command: its words, the first being its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    /// The words, in order.
    pub words: Vec<Word>,
}

/// One word of a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word {
    /// The whole word, `{*}` prefix included.
    pub span: Span,
    /// Whether the word carries the `{*}` expansion prefix.
    pub expand: bool,
    /// How the word is delimited, and what it holds.
    pub kind: WordKind,
}

/// The three kinds of word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WordKind {
    /// `{...}`: the body, verbatim.
    Braced {
        /// The `{`.
        open: Span,
        /// Between the braces.
        body: Span,
        /// The `}`.
        close: Span,
    },
    /// `"..."`: substitutions inside.
    Quoted {
        /// The opening `"`.
        open: Span,
        /// The closing `"`.
        close: Span,
        /// What lies between them.
        parts: Vec<Part>,
    },
    /// A word with no delimiter of its own.
    Bare {
        /// Its pieces, in order.
        parts: Vec<Part>,
    },
}

/// A piece of a quoted or bare word, or of an array index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Part {
    /// Bytes that stand for themselves.
    Text(Span),
    /// A backslash and what it escapes.
    Escape(Span),
    /// `$name`, `$ns::name`, `${name}` or `$name(index)`.
    Var {
        /// The whole substitution.
        span: Span,
        /// The index's pieces; empty without one.
        index: Vec<Part>,
    },
    /// `[...]`: a script whose result replaces it.
    Cmd {
        /// The whole substitution, brackets included.
        span: Span,
        /// The script inside.
        script: Script,
    },
}

/// `src` as a script.
///
/// # Errors
///
/// [`Error::Parse`] when Tcl's parser would reject it.
pub fn parse(src: &str) -> Result<Script> {
    parse_at(src, Span::new(0, src.len()))
}

/// The text at `body` within `src` as a script, its spans into `src`:
/// the body of a `proc`, `if` or `[...]` read where it stands.
///
/// # Errors
///
/// [`Error::Parse`] when Tcl's parser would reject the body, with the
/// line and column in `src`.
pub fn parse_at(src: &str, body: Span) -> Result<Script> {
    validate(src, body)?;
    let mut scan = Scan::new(src, body);
    Ok(scan.script(false))
}

/// [`syntax::check`] over the text at `body`, its error placed in `src`.
fn validate(src: &str, body: Span) -> Result<()> {
    let text = body
        .of(src)
        .ok_or_else(|| Error::parse(LangId::Tcl, format!("{body:?} lies outside the text")))?;
    syntax::check(text).map_err(|e| {
        let (line, col) = place(src, body.start, e.line, e.col);
        Error::parse(LangId::Tcl, format!("{line}:{col}: {}", e.message))
    })
}

/// A line and column within a body starting at byte `start` of `src`, as
/// a line and column of `src`.
fn place(src: &str, start: usize, line: usize, col: usize) -> (usize, usize) {
    let before = src.as_bytes().get(..start).unwrap_or_default();
    let lines_before = src
        .get(..start)
        .map_or(0, |text| text.matches('\n').count());
    if line > 1 {
        return (lines_before + line, col);
    }
    let line_start = before
        .iter()
        .rposition(|b| *b == b'\n')
        .map_or(0, |nl| nl + 1);
    (lines_before + 1, start - line_start + col)
}

/// Tcl's word-separating whitespace: a newline is not, it ends a command.
fn blank(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\x0b' | b'\x0c' | b'\r')
}

/// A byte of a `$name`: letters, digits, `_` (`::` is handled apart).
fn name_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// A cursor over one body of the source. Every delimiter Tcl cares about
/// is ASCII, so bytes are enough, and UTF-8 in words passes through.
///
/// The text has been through [`syntax::check`] by the time a tree is
/// built from it, so every opener closes before `end`; the scan still
/// never reads past `end`, and stops a word or a script there.
struct Scan<'s> {
    src: &'s [u8],
    pos: usize,
    end: usize,
}

impl<'s> Scan<'s> {
    fn new(src: &'s str, body: Span) -> Scan<'s> {
        let end = body.end.min(src.len());
        Scan {
            src: src.as_bytes(),
            pos: body.start.min(end),
            end,
        }
    }

    fn peek(&self) -> Option<u8> {
        (self.pos < self.end).then(|| self.src.get(self.pos).copied())?
    }

    fn at(&self, offset: usize) -> Option<u8> {
        let i = self.pos + offset;
        (i < self.end).then(|| self.src.get(i).copied())?
    }

    /// Past a backslash and the byte it escapes (both, if there is one).
    fn escape(&mut self) {
        self.pos = (self.pos + 2).min(self.end);
    }

    /// Whether the next byte ends a word: whitespace (a backslash-newline
    /// is whitespace too), a command end, the end of the body, or `]`
    /// closing a `[...]` this script is nested in.
    fn word_end(&self, nested: bool) -> bool {
        match self.peek() {
            None => true,
            Some(b'\\') => self.at(1) == Some(b'\n'),
            Some(b) => blank(b) || b == b'\n' || b == b';' || (nested && b == b']'),
        }
    }

    /// Past whitespace, and past newlines and `;` too when `commands`.
    fn skip_space(&mut self, commands: bool) {
        while let Some(b) = self.peek() {
            if blank(b) || (commands && (b == b'\n' || b == b';')) {
                self.pos += 1;
            } else if b == b'\\' && self.at(1) == Some(b'\n') {
                self.escape();
            } else {
                break;
            }
        }
    }

    /// Commands until the end of the body or, `nested`, the `]` closing
    /// the substitution (consumed).
    fn script(&mut self, nested: bool) -> Script {
        let mut out = Script::default();
        loop {
            self.skip_space(true);
            match self.peek() {
                None => return out,
                Some(b']') if nested => {
                    self.pos += 1;
                    return out;
                }
                Some(b'#') => self.comment(),
                Some(_) => {
                    let command = self.command(nested);
                    if !command.words.is_empty() {
                        out.commands.push(command);
                    }
                }
            }
        }
    }

    /// A comment runs to a newline no backslash escapes.
    fn comment(&mut self) {
        while let Some(b) = self.peek() {
            match b {
                b'\n' => return,
                b'\\' => self.escape(),
                _ => self.pos += 1,
            }
        }
    }

    /// The words of one command, up to (not past) what ends it.
    fn command(&mut self, nested: bool) -> Command {
        let mut words = Vec::new();
        loop {
            self.skip_space(false);
            match self.peek() {
                None | Some(b'\n' | b';') => return Command { words },
                Some(b']') if nested => return Command { words },
                // A word always consumes at least its first byte: blanks,
                // command ends and a nested `]` are matched above.
                Some(_) => words.push(self.word(nested)),
            }
        }
    }

    fn word(&mut self, nested: bool) -> Word {
        let start = self.pos;
        let mut expand = false;
        // `{*}` then more word: the expansion prefix, not a braced word.
        if self.src.get(self.pos..self.pos + 3) == Some(b"{*}".as_slice())
            && self.pos + 3 <= self.end
        {
            self.pos += 3;
            if self.word_end(nested) {
                self.pos = start;
            } else {
                expand = true;
            }
        }
        let kind = match self.peek() {
            Some(b'{') => self.braced(),
            Some(b'"') => self.quoted(),
            _ => WordKind::Bare {
                parts: self.pieces(|s| s.word_end(nested), false),
            },
        };
        Word {
            span: Span::new(start, self.pos),
            expand,
            kind,
        }
    }

    /// `{...}`: nested braces counted, a backslash hides the next byte,
    /// nothing substituted.
    fn braced(&mut self) -> WordKind {
        let open = self.pos;
        self.skip_braced();
        let close = self.pos.saturating_sub(1).max(open + 1);
        WordKind::Braced {
            open: Span::new(open, open + 1),
            body: Span::new(open + 1, close),
            close: Span::new(close, self.pos.max(close)),
        }
    }

    /// Past a `{...}` starting at the cursor; `false` if it never closes.
    fn skip_braced(&mut self) -> bool {
        self.pos += 1;
        let mut depth = 1usize;
        while let Some(b) = self.peek() {
            match b {
                b'\\' => self.escape(),
                b'{' => {
                    depth += 1;
                    self.pos += 1;
                }
                b'}' => {
                    depth -= 1;
                    self.pos += 1;
                    if depth == 0 {
                        return true;
                    }
                }
                _ => self.pos += 1,
            }
        }
        false
    }

    /// `"..."`, with its substitutions.
    fn quoted(&mut self) -> WordKind {
        let open = self.pos;
        self.pos += 1;
        let parts = self.pieces(|s| s.peek() == Some(b'"'), false);
        let close = self.pos;
        // Checked text closes every quote before the body ends.
        self.pos = (self.pos + 1).min(self.end);
        WordKind::Quoted {
            open: Span::new(open, open + 1),
            close: Span::new(close, self.pos),
            parts,
        }
    }

    /// Pieces until `stop` holds (or the body ends): runs of plain bytes
    /// merged into one [`Part::Text`], escapes, `$` and `[...]`. Inside
    /// an index, `paren` stops at the `)` that closes it.
    fn pieces(&mut self, stop: impl Fn(&Scan<'_>) -> bool, paren: bool) -> Vec<Part> {
        let mut parts = Vec::new();
        let mut text: Option<usize> = None;
        while self.peek().is_some() && !stop(self) {
            if paren && self.peek() == Some(b')') {
                break;
            }
            let at = self.pos;
            let piece = match self.peek() {
                Some(b'\\') => {
                    self.escape();
                    Some(Part::Escape(Span::new(at, self.pos)))
                }
                Some(b'$' | b'[') => self.substitution(),
                _ => None,
            };
            if let Some(part) = piece {
                if let Some(from) = text.take() {
                    parts.push(Part::Text(Span::new(from, at)));
                }
                parts.push(part);
            } else {
                // No substitution started here: a plain byte, or a `$`
                // that names nothing.
                self.pos += 1;
                text.get_or_insert(at);
            }
        }
        if let Some(from) = text {
            parts.push(Part::Text(Span::new(from, self.pos)));
        }
        parts
    }

    /// A `$` or `[` substitution at the cursor, or `None` for a `$` that
    /// starts no variable (plain text; the cursor is left on it).
    fn substitution(&mut self) -> Option<Part> {
        let start = self.pos;
        if self.peek() != Some(b'[') {
            return self.variable();
        }
        self.pos += 1;
        let script = self.script(true);
        Some(Part::Cmd {
            span: Span::new(start, self.pos),
            script,
        })
    }

    /// `$name`, `$ns::name`, `$name(index)` or `${any}`; a `$` that starts
    /// no name is plain, and the cursor stays on it.
    fn variable(&mut self) -> Option<Part> {
        let dollar = self.pos;
        self.pos += 1;
        if self.peek() == Some(b'{') {
            while let Some(b) = self.peek() {
                self.pos += 1;
                if b == b'}' {
                    break;
                }
            }
            return Some(Part::Var {
                span: Span::new(dollar, self.pos),
                index: Vec::new(),
            });
        }
        let name = self.pos;
        loop {
            match self.peek() {
                Some(b) if name_byte(b) => self.pos += 1,
                Some(b':') if self.at(1) == Some(b':') => {
                    while self.peek() == Some(b':') {
                        self.pos += 1;
                    }
                }
                _ => break,
            }
        }
        // `$(x)` is element `x` of the array named "": only a `$` followed
        // by neither a name nor `(` is plain.
        let mut index = Vec::new();
        if self.peek() == Some(b'(') {
            self.pos += 1;
            index = self.pieces(|_| false, true);
            // Checked text closes every index before the body ends.
            self.pos = (self.pos + 1).min(self.end);
        } else if self.pos == name {
            self.pos = dollar;
            return None;
        }
        Some(Part::Var {
            span: Span::new(dollar, self.pos),
            index,
        })
    }
}
