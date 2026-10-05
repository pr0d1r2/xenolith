//! The tcl syntax check (`languages/shells/tcl:V198`): what Tcl's own
//! parser rejects in a script, found in Rust, so checking a file needs no
//! `tclsh`.
//!
//! Tcl parses a script one command at a time and runs each before it
//! parses the next, and a braced word -- a `proc` body, an `if` branch --
//! is only text until something runs it. So the syntax Tcl can reject in
//! a file is the WORD syntax of its top-level commands and of the `[...]`
//! substitutions in their words:
//!
//! - an unclosed `{`, `"`, `[`, `${` or `$name(`, or a backslash-newline
//!   ending the text: exactly what `info complete` (C:
//!   `Tcl_CommandComplete`) calls incomplete, and what the fleet's hook
//!   asked `tclsh` (`languages/shells/tcl:R193`);
//! - extra characters after a close-brace or a close-quote (`{a}b`),
//!   which Tcl reports when it reaches that command.
//!
//! Braced words are matched, not entered, with Tcl's two sharp edges kept
//! on purpose: braces count inside a comment (`# {` opens one), because a
//! braced word is found before anyone knows it holds a comment; and the
//! scan stops at the first error, because Tcl's parser does too.
//!
//! [`run`] is the command-line face: `xenolith-tcl-syntax FILE...`, one
//! `path:line:col: message` per file with an error, exit 1 if any, 2 when
//! a file cannot be read or none is named.

use std::ffi::OsString;
use std::io::Write;
use std::path::Path;

use xenolith_lang_api::{FileArg, Format, LintCmd};

#[cfg(test)]
mod tests;

/// A syntax error Tcl would report: where, and in Tcl's words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxError {
    /// 1-based line: of the unclosed opener, or of the extra character.
    pub line: usize,
    /// 1-based column on that line, in bytes.
    pub col: usize,
    /// Tcl's own words: `missing close-brace`, `missing "`,
    /// `extra characters after close-brace`, ...
    pub message: &'static str,
}

/// Why a scan stopped early.
enum Stop {
    /// The text ended inside the opener at this byte offset.
    Incomplete(usize, &'static str),
    /// A parse error that is not an unclosed opener, at this byte offset:
    /// Tcl stops parsing there, and so does the scan.
    Error(usize, &'static str),
    /// The text ended inside a `[...]`; the substitution that opened it
    /// turns this into [`Stop::Incomplete`] at its `[`.
    Unclosed,
}

/// `Ok` when Tcl parses `src` without a syntax error, else the first one.
///
/// # Errors
///
/// [`SyntaxError`] naming the unclosed opener or the extra character, at
/// its line and column.
pub fn check(src: &str) -> Result<(), SyntaxError> {
    let mut scan = Scan {
        src: src.as_bytes(),
        pos: 0,
    };
    let stop = match scan.script(false) {
        Ok(()) => trailing_continuation(src.as_bytes()),
        // Never at the top: only a nested script ends so.
        Err(Stop::Unclosed) => None,
        Err(Stop::Incomplete(at, message) | Stop::Error(at, message)) => Some((at, message)),
    };
    match stop {
        None => Ok(()),
        Some((at, message)) => {
            let before = src.as_bytes().get(..at).unwrap_or_default();
            // One more piece than newlines: the line the offset is on.
            let line = before.split(|b| *b == b'\n').count();
            let start = before
                .iter()
                .rposition(|b| *b == b'\n')
                .map_or(0, |nl| nl + 1);
            Err(SyntaxError {
                line,
                col: at - start + 1,
                message,
            })
        }
    }
}

/// The offset of a backslash-newline that ENDS the text, if one does: the
/// command it continues has not ended, so Tcl calls the script
/// incomplete. The backslash must itself be unescaped: `\\` + newline is
/// a literal backslash and a plain newline.
fn trailing_continuation(src: &[u8]) -> Option<(usize, &'static str)> {
    let body = src.strip_suffix(b"\n")?;
    let run = body.iter().rev().take_while(|b| **b == b'\\').count();
    (run % 2 == 1).then(|| (body.len() - 1, "missing line after backslash-newline"))
}

/// The check as the lint engine runs it (`src/lint:V8`): this crate's
/// binary, the file appended, its `file:line:col: message` lines kept as
/// they are. The one check of a tcl host file and of a tcl extract.
#[must_use]
pub fn lint_cmd() -> LintCmd {
    LintCmd {
        argv: vec!["xenolith-tcl-syntax".to_owned()],
        file_arg: FileArg::Append,
        format: Format::Raw,
    }
}

/// `xenolith-tcl-syntax FILE...`: every named file checked, each one with a
/// syntax error reported on `out` as `path:line:col: message`. The exit
/// code: 0 all clean, 1 any error, 2 no file named or one that cannot be
/// read (reported, and the rest still checked).
pub fn run(args: impl IntoIterator<Item = OsString>, out: &mut impl Write) -> u8 {
    let files: Vec<OsString> = args.into_iter().collect();
    if files.is_empty() {
        // A failed write to the report stream has nowhere left to go.
        let _ = writeln!(out, "usage: xenolith-tcl-syntax FILE...");
        return 2;
    }
    let mut code = 0;
    for file in &files {
        let path = Path::new(file);
        let src = match std::fs::read(path) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(e) => {
                let _ = writeln!(out, "{}: cannot read: {e}", path.display());
                code = 2;
                continue;
            }
        };
        if let Err(at) = check(&src) {
            let _ = writeln!(
                out,
                "{}:{}:{}: {}",
                path.display(),
                at.line,
                at.col,
                at.message
            );
            code = code.max(1);
        }
    }
    code
}

/// A cursor over the script's bytes. Every delimiter Tcl cares about is
/// ASCII, so bytes are enough, and UTF-8 in words passes through.
struct Scan<'s> {
    src: &'s [u8],
    pos: usize,
}

/// Tcl's word-separating whitespace: a newline is not, it ends a command.
fn blank(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\x0b' | b'\x0c' | b'\r')
}

/// A byte of a `$name`: letters, digits, `_` (`::` is handled apart).
fn name_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

impl Scan<'_> {
    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn at(&self, offset: usize) -> Option<u8> {
        self.src.get(self.pos + offset).copied()
    }

    /// Past a backslash and the byte it escapes (both, if there is one).
    fn escape(&mut self) {
        self.pos = (self.pos + 2).min(self.src.len());
    }

    /// Whether the next byte ends a word: whitespace (a backslash-newline
    /// is whitespace too), a command end, the end of text, or `]` closing
    /// a `[...]` this script is nested in.
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

    /// Commands until the end of text or, `nested`, the `]` closing the
    /// substitution (consumed). Ending the text while nested is the
    /// caller's to report: only it knows where the `[` was.
    fn script(&mut self, nested: bool) -> Result<(), Stop> {
        loop {
            self.skip_space(true);
            match self.peek() {
                None => return if nested { Err(Stop::Unclosed) } else { Ok(()) },
                Some(b']') if nested => {
                    self.pos += 1;
                    return Ok(());
                }
                Some(b'#') => self.comment(),
                Some(_) => self.command(nested)?,
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
    fn command(&mut self, nested: bool) -> Result<(), Stop> {
        loop {
            self.skip_space(false);
            match self.peek() {
                None | Some(b'\n' | b';') => return Ok(()),
                Some(b']') if nested => return Ok(()),
                Some(_) => self.word(nested)?,
            }
        }
    }

    fn word(&mut self, nested: bool) -> Result<(), Stop> {
        // `{*}` then more word: the expansion prefix, not a braced word.
        if self.src.get(self.pos..self.pos + 3) == Some(b"{*}".as_slice()) {
            self.pos += 3;
            if self.word_end(nested) {
                return Ok(());
            }
        }
        let (closed, after) = match self.peek() {
            Some(b'{') => (self.braced(), "extra characters after close-brace"),
            Some(b'"') => (self.quoted(), "extra characters after close-quote"),
            _ => return self.bare(nested),
        };
        closed?;
        if self.word_end(nested) {
            Ok(())
        } else {
            Err(Stop::Error(self.pos, after))
        }
    }

    /// `{...}`: nested braces counted, a backslash hides the next byte,
    /// nothing substituted.
    fn braced(&mut self) -> Result<(), Stop> {
        let open = self.pos;
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
                        return Ok(());
                    }
                }
                _ => self.pos += 1,
            }
        }
        Err(Stop::Incomplete(open, "missing close-brace"))
    }

    /// `"..."`, with its `$` and `[...]` substitutions.
    fn quoted(&mut self) -> Result<(), Stop> {
        let open = self.pos;
        self.pos += 1;
        while let Some(b) = self.peek() {
            if b == b'"' {
                self.pos += 1;
                return Ok(());
            }
            self.piece(b)?;
        }
        Err(Stop::Incomplete(open, "missing \""))
    }

    /// A bare word, to whatever ends it.
    fn bare(&mut self, nested: bool) -> Result<(), Stop> {
        while !self.word_end(nested) {
            if let Some(b) = self.peek() {
                self.piece(b)?;
            }
        }
        Ok(())
    }

    /// One piece of a word that substitutes: an escape, a `$`, a `[...]`,
    /// or a plain byte.
    fn piece(&mut self, b: u8) -> Result<(), Stop> {
        match b {
            b'\\' => self.escape(),
            b'$' => self.variable()?,
            b'[' => self.substitution()?,
            _ => self.pos += 1,
        }
        Ok(())
    }

    /// `[...]`: a nested script up to its `]`.
    fn substitution(&mut self) -> Result<(), Stop> {
        let open = self.pos;
        self.pos += 1;
        match self.script(true) {
            Err(Stop::Unclosed) => Err(Stop::Incomplete(open, "missing close-bracket")),
            other => other,
        }
    }

    /// `$name`, `$ns::name`, `$name(index)` or `${any}`; a `$` that starts
    /// no name is a plain `$`.
    fn variable(&mut self) -> Result<(), Stop> {
        let dollar = self.pos;
        self.pos += 1;
        if self.peek() == Some(b'{') {
            let rest = self.src.get(self.pos..).unwrap_or_default();
            return match rest.iter().position(|b| *b == b'}') {
                Some(close) => {
                    self.pos += close + 1;
                    Ok(())
                }
                None => Err(Stop::Incomplete(
                    dollar,
                    "missing close-brace for variable name",
                )),
            };
        }
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
        if self.peek() == Some(b'(') {
            let paren = self.pos;
            self.pos += 1;
            while let Some(b) = self.peek() {
                if b == b')' {
                    self.pos += 1;
                    return Ok(());
                }
                self.piece(b)?;
            }
            return Err(Stop::Incomplete(paren, "missing )"));
        }
        Ok(())
    }
}
