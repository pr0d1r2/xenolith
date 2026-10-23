//! Tcl as a host: the files it claims (`languages/shells/tcl:V195`) and
//! the programs its `exec` and `spawn` hand other interpreters
//! (`languages/shells/tcl:V196`).
//!
//! A site is a delimiter AND a sink context
//! (`languages/api/src/site:V38`): the same `{a | b}` is a site after
//! `exec sh -c` and inert data after `puts`. Both halves come from the
//! tree -- the command node and its words -- and which word holds a
//! program is `sinks`'s to say.
//!
//! The tree is this crate's own Tcl word parser ([`script`]): Tcl reads a
//! braced argument literally, and so does the walk. Which braced words
//! are scripts -- a `proc` body, an `if` branch, a `namespace eval` body
//! -- is decided here, by command, and only those are parsed and searched
//! (`languages/shells/tcl` §I). Every other braced argument is OPAQUE
//! data: `{ls $(pwd)}` handed to `sh -c` is shell, never read as Tcl.
//! A text Tcl's parser would reject, at the top or in a body the walk
//! reads, fails the whole file (`languages:V78`).

use std::path::Path;

use xenolith_lang_api::{
    Delim, DelimKind, Error, Host, Invoke, LangId, LintCmd, LoadRef, Result, Site, Span, shebang,
};

use crate::script::{self, Command, Part, Script, Word, WordKind};
use crate::sinks::{self, Runner};
use crate::syntax;

#[cfg(test)]
mod tests;

/// Tcl as a host.
///
/// A unit struct: everything it answers comes from the text it is handed
/// (`languages/api:V36`), so the registry can hold one `&'static dyn Host`
/// for it (`src/registry:V41`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TclHost;

/// Extensions that make a file tcl whatever its first line says
/// (`languages/shells/tcl:V195`); `exp` is the expect dialect.
pub(crate) const EXTENSIONS: &[&str] = &["exp", "tcl", "tk"];

/// Interpreters whose shebang makes a file tcl; `expect` is the dialect.
pub(crate) const INTERPRETERS: &[&str] = &["expect", "tclsh", "wish"];

/// The dialect of the expect extension and interpreter.
pub(crate) const EXPECT: &str = "expect";

/// The interpreter a shebang line resolves to, version stripped
/// (`tclsh8.6` is `tclsh`), when it is one of [`INTERPRETERS`].
fn shebang_interpreter(head: &str) -> Option<&'static str> {
    let line = shebang::parse(head)?;
    let resolved = line.resolved_interpreter();
    let name = resolved.rsplit('/').next().unwrap_or(resolved);
    let bare = name.trim_end_matches(|c: char| c.is_ascii_digit() || c == '.');
    INTERPRETERS.iter().copied().find(|known| *known == bare)
}

/// The dialect of a tcl file: `Some("expect")` for `*.exp` or an expect
/// shebang, `None` for plain tcl (`languages/shells/tcl:V195`). expect is
/// a dialect, not a language: the same grammar reads both, and only the
/// commands in scope differ (`languages/shells:V132`).
#[must_use]
pub fn dialect(path: &Path, head: &str) -> Option<&'static str> {
    let by_extension = path.extension().is_some_and(|ext| ext == "exp");
    (by_extension || shebang_interpreter(head) == Some(EXPECT)).then_some(EXPECT)
}

impl Host for TclHost {
    fn id(&self) -> LangId {
        LangId::Tcl
    }

    /// `*.tcl`, `*.tk`, `*.exp`, or a shebang resolving to `tclsh`,
    /// `wish` or `expect` (`languages/shells/tcl:V195`).
    ///
    /// The shebang is read here rather than through
    /// `shebang::resolves_to`, whose table does not name tcl: widening
    /// that table changes which guest other hosts see behind a shebang,
    /// which is theirs to decide.
    fn claims(&self, path: &Path, head: &str) -> bool {
        let by_extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| EXTENSIONS.contains(&ext));
        by_extension || shebang_interpreter(head).is_some()
    }

    /// Every `-c` word and `<<` value an `exec` or `spawn` hands an
    /// interpreter, sorted by span (`languages/shells/tcl:V196`).
    ///
    /// A syntax error -- at the top, or in a body the walk reads as a
    /// script -- fails the whole file: spans after it are a guess, and a
    /// site built on a guess rewrites the wrong bytes (`languages:V78`).
    fn sites(&self, src: &str) -> Result<Vec<Site>> {
        let top = script::parse(src)?;
        let mut walk = Walk::default();
        walk.script(&top, src)?;
        walk.sites.sort_by_key(|site| site.delim.open);
        Ok(walk.sites)
    }

    /// Not offered: no load idiom is decided for a tcl host
    /// (`languages/shells/tcl` §I), and an empty list would read as
    /// "nothing loaded here" (`languages/api:V37`).
    fn loads(&self, _src: &str) -> Result<Vec<LoadRef>> {
        Err(Error::unsupported(LangId::Tcl, "loads"))
    }

    /// Not offered; see [`TclHost::loads`].
    fn rewrite(&self, _src: &str, _site: &Site, _invoke: &Invoke, _path: &Path) -> Result<String> {
        Err(Error::unsupported(LangId::Tcl, "rewrite"))
    }

    /// Not offered; see [`TclHost::loads`].
    fn inline(&self, _src: &str, _load: &LoadRef, _body: &str) -> Result<String> {
        Err(Error::unsupported(LangId::Tcl, "inline"))
    }

    /// The body as the interpreter receives it
    /// (`languages/api/src/lens:V39`): verbatim, because this host
    /// reports only words whose bytes ARE the argument -- a braced word
    /// with no backslash-newline, a quoted or bare word with no backslash
    /// escape. Holes arrive already replaced by the engine.
    fn unescape(&self, delim: &Delim, raw: &str) -> Result<String> {
        match delim.kind {
            DelimKind::ArgvString => Ok(raw.to_owned()),
            _ => Err(Error::unsupported(LangId::Tcl, "unescape")),
        }
    }

    /// This crate's own syntax check, `xenolith-tcl-syntax`: what Tcl's
    /// parser rejects, found in Rust with no `tclsh` on PATH
    /// (`languages/shells/tcl:V198`).
    fn checks(&self) -> Vec<LintCmd> {
        vec![syntax::lint_cmd()]
    }

    /// None: a syntax error has no mechanical fix
    /// (`languages/shells/tcl:V198`).
    fn fixers(&self) -> Vec<LintCmd> {
        Vec::new()
    }
}

/// The text of a word that is one plain run of bytes, else `None`: a
/// braced, quoted, substituted, escaped or expanded word is never a flag
/// or a command name.
fn plain<'s>(word: &Word, src: &'s str) -> Option<&'s str> {
    match &word.kind {
        WordKind::Bare { parts } if !word.expand => match parts.as_slice() {
            [Part::Text(span)] => span.of(src),
            _ => None,
        },
        _ => None,
    }
}

/// The delimiter and holes of a program word, or `None` for a word whose
/// bytes are not the argument Tcl passes.
fn program_word(word: &Word, src: &str) -> Option<(Delim, Vec<Span>)> {
    if word.expand {
        return None;
    }
    match &word.kind {
        WordKind::Braced { open, body, close } => {
            // Tcl turns backslash-newline into a space even in braces.
            if body.of(src).is_none_or(|text| text.contains("\\\n")) {
                return None;
            }
            let delim = Delim {
                kind: DelimKind::ArgvString,
                open: *open,
                body: *body,
                close: *close,
            };
            Some((delim, Vec::new()))
        }
        WordKind::Quoted { open, close, parts } => {
            let delim = Delim {
                kind: DelimKind::ArgvString,
                open: *open,
                body: Span::new(open.end, close.start),
                close: *close,
            };
            Some((delim, holes(parts)?))
        }
        // A bare word, maybe concatenated (`foo$bar`): no delimiter of its
        // own, so both ends are empty spans around the whole word.
        WordKind::Bare { parts } => {
            let (start, end) = (word.span.start, word.span.end);
            let delim = Delim {
                kind: DelimKind::ArgvString,
                open: Span::new(start, start),
                body: Span::new(start, end),
                close: Span::new(end, end),
            };
            Some((delim, holes(parts)?))
        }
    }
}

/// The substitutions among `parts` -- Tcl's `$var` and `[cmd]`, host
/// interpolations the guest never sees as written
/// (`languages/api/src/holes:V40`) -- or `None` when a backslash escape
/// makes the word's bytes differ from the argument.
fn holes(parts: &[Part]) -> Option<Vec<Span>> {
    let mut out = Vec::new();
    for part in parts {
        match part {
            Part::Escape(_) => return None,
            Part::Var { span, .. } | Part::Cmd { span, .. } => out.push(*span),
            Part::Text(_) => {}
        }
    }
    Some(out)
}

/// How a command reads one of its words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    /// A script Tcl runs: a braced one is parsed and searched.
    Script,
    /// An expression: its `[cmd]` and `$var` run, braced or not.
    Expr,
    /// Data: a braced one is opaque.
    Data,
}

/// The role of each word of `command` (the name included), by the
/// commands whose bodies Tcl runs (`languages/shells/tcl` §I): `proc`,
/// `if`/`elseif`/`else`, `while`, `foreach`, `catch`, `try` (its `on`,
/// `trap` and `finally` bodies), `namespace eval` and `expr`. Any other
/// command's words are data.
fn roles(command: &Command, src: &str) -> Vec<Role> {
    let words = &command.words;
    let mut roles = vec![Role::Data; words.len()];
    let word = |i: usize| words.get(i).and_then(|w| plain(w, src));
    let mut set = |i: usize, role: Role| {
        if let Some(slot) = roles.get_mut(i) {
            *slot = role;
        }
    };
    match word(0) {
        Some("proc") => set(3, Role::Script),
        Some("while") => {
            set(1, Role::Expr);
            set(2, Role::Script);
        }
        Some("foreach") => set(words.len().saturating_sub(1), Role::Script),
        Some("catch") => set(1, Role::Script),
        Some("expr") => (1..words.len()).for_each(|i| set(i, Role::Expr)),
        Some("namespace") if word(1) == Some("eval") => {
            (3..words.len()).for_each(|i| set(i, Role::Script));
        }
        Some("if") => {
            let mut i = 1;
            set(i, Role::Expr);
            i += 1;
            loop {
                if word(i) == Some("then") {
                    i += 1;
                }
                set(i, Role::Script);
                i += 1;
                match word(i) {
                    Some("elseif") => {
                        set(i + 1, Role::Expr);
                        i += 2;
                    }
                    Some("else") => {
                        set(i + 1, Role::Script);
                        break;
                    }
                    _ => {
                        // `if c b c2`: a last word with no `else` is a body.
                        set(i, Role::Script);
                        break;
                    }
                }
            }
        }
        Some("try") => {
            set(1, Role::Script);
            let mut i = 2;
            while i < words.len() {
                match word(i) {
                    Some("on" | "trap") => {
                        set(i + 3, Role::Script);
                        i += 4;
                    }
                    Some("finally") => {
                        set(i + 1, Role::Script);
                        i += 2;
                    }
                    _ => i += 1,
                }
            }
        }
        _ => {}
    }
    roles
}

/// What the walk collects: the sites, and the inside of every braced word
/// it treated as data rather than script.
#[derive(Default)]
struct Walk {
    sites: Vec<Site>,
    opaque: Vec<Span>,
}

impl Walk {
    /// Every site in a script.
    fn script(&mut self, script: &Script, src: &str) -> Result<()> {
        for command in &script.commands {
            self.command(command, src)?;
        }
        Ok(())
    }

    /// One command: its site, if it is one, then what its words run.
    fn command(&mut self, command: &Command, src: &str) -> Result<()> {
        if let Some(site) = site(command, src) {
            self.sites.push(site);
        }
        for (word, role) in command.words.iter().zip(roles(command, src)) {
            self.word(word, role, src)?;
        }
        Ok(())
    }

    /// One word, read in its role.
    fn word(&mut self, word: &Word, role: Role, src: &str) -> Result<()> {
        match (&word.kind, role) {
            (WordKind::Braced { body, .. }, Role::Script) => {
                let inner = script::parse_at(src, *body)?;
                self.script(&inner, src)
            }
            (WordKind::Braced { body, .. }, Role::Expr) => {
                self.parts(&script::expression(src, *body)?, src)
            }
            (WordKind::Braced { body, .. }, Role::Data) => {
                self.opaque.push(*body);
                Ok(())
            }
            (WordKind::Quoted { parts, .. } | WordKind::Bare { parts }, _) => {
                self.parts(parts, src)
            }
        }
    }

    /// The scripts a word's substitutions run: each `[cmd]`, and any in a
    /// variable's index.
    fn parts(&mut self, parts: &[Part], src: &str) -> Result<()> {
        for part in parts {
            match part {
                Part::Cmd { script, .. } => self.script(script, src)?,
                Part::Var { index, .. } => self.parts(index, src)?,
                Part::Text(_) | Part::Escape(_) => {}
            }
        }
        Ok(())
    }
}

/// The site of an `exec`/`spawn` command, if its argv holds a program
/// (`languages/shells/tcl:V196`).
fn site(command: &Command, src: &str) -> Option<Site> {
    let (name, rest) = command.words.split_first()?;
    let runner = Runner::of(plain(name, src)?)?;
    let (first, rest) = rest.split_first()?;
    let interpreter = sinks::interpreter(plain(first, src)?);
    let args: Vec<Option<&str>> = rest.iter().map(|w| plain(w, src)).collect();
    let program = sinks::program(runner, interpreter, &args)?;
    let (delim, holes) = program_word(rest.get(program.at)?, src)?;
    Some(Site {
        sink: format!("{} {} {}", runner.as_str(), interpreter.name, program.how),
        guest: program.guest,
        env: program.env,
        delim,
        holes,
    })
}
