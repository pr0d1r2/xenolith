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
//! One property of this grammar shapes the whole walk. tree-sitter-tcl
//! parses EVERY braced word as a script, while Tcl reads a braced
//! argument literally: `{ls $(pwd)}` handed to `sh -c` is shell, and the
//! grammar's errors inside it say nothing about the Tcl file. So a braced
//! word the walk does not treat as a script -- an argument of an ordinary
//! command, a site's body -- is OPAQUE: never searched for sites, and a
//! parse error wholly inside its braces is the grammar misreading data.
//! Any other error fails the file (`languages:V78`).

use std::path::Path;

use tree_sitter::Node;
use xenolith_lang_api::{
    Delim, DelimKind, Error, Host, Invoke, LangId, LintCmd, LoadRef, Result, Site, Span, shebang,
};

use crate::grammar;
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
    /// A parse error outside every opaque braced word fails the whole
    /// file: spans inside and after an `ERROR` node are the parser's
    /// guess, and a site built on a guess rewrites the wrong bytes
    /// (`languages:V78`).
    fn sites(&self, src: &str) -> Result<Vec<Site>> {
        let tree = grammar::parse(src)?;
        let root = tree.root_node();
        let mut walk = Walk::default();
        walk.node(root, src);
        if root.has_error() && !errors_contained(root, &walk.opaque) {
            return Err(Error::parse(
                LangId::Tcl,
                "the file is not valid tcl to this grammar, so none of its sites were read",
            ));
        }
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

fn text<'s>(node: Node<'_>, src: &'s str) -> &'s str {
    src.get(node.byte_range()).unwrap_or_default()
}

fn span(node: Node<'_>) -> Span {
    Span::new(node.start_byte(), node.end_byte())
}

/// Every child of `node`, in order.
fn children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.children(&mut cursor).collect()
}

/// The words of a `word_list`: the grammar concatenates adjacent pieces
/// (`foo$bar`) as sibling nodes with no wrapper, so a word is a run of
/// siblings with no gap between them.
pub(crate) fn words<'t>(list: &[Node<'t>]) -> Vec<Vec<Node<'t>>> {
    let mut out: Vec<Vec<Node<'t>>> = Vec::new();
    for node in list {
        match out.last_mut() {
            Some(word)
                if word
                    .last()
                    .is_some_and(|l| l.end_byte() == node.start_byte()) =>
            {
                word.push(*node);
            }
            _ => out.push(vec![*node]),
        }
    }
    out
}

/// The text of a word that is one plain `simple_word`, else `None`: a
/// braced, quoted or substituted word is never a flag or a command name.
fn plain<'s>(word: &[Node<'_>], src: &'s str) -> Option<&'s str> {
    match word {
        [only] if only.kind() == "simple_word" => Some(text(*only, src)),
        _ => None,
    }
}

/// The inside of a braced word, between braces the grammar really found
/// (neither `MISSING`), or `None`.
fn braced_inside(node: Node<'_>) -> Option<(Node<'_>, Node<'_>)> {
    let parts = children(node);
    let open = parts
        .first()
        .filter(|n| n.kind() == "{" && !n.is_missing())?;
    let close = parts
        .last()
        .filter(|n| n.kind() == "}" && !n.is_missing())?;
    (open.id() != close.id()).then_some((*open, *close))
}

/// The delimiter and holes of a program word, or `None` for a word whose
/// bytes are not the argument Tcl passes.
fn program_word(word: &[Node<'_>], src: &str) -> Option<(Delim, Vec<Span>)> {
    let first = word.first()?;
    let last = word.last()?;
    if let [only] = word {
        match only.kind() {
            "braced_word" => {
                let (open, close) = braced_inside(*only)?;
                let body = Span::new(open.end_byte(), close.start_byte());
                // Tcl turns backslash-newline into a space even in braces.
                if body.of(src)?.contains("\\\n") {
                    return None;
                }
                let delim = Delim {
                    kind: DelimKind::ArgvString,
                    open: span(open),
                    body,
                    close: span(close),
                };
                return Some((delim, Vec::new()));
            }
            "quoted_word" => {
                let parts = children(*only);
                let open = parts.first().filter(|n| n.kind() == "\"")?;
                let close = parts
                    .last()
                    .filter(|n| n.kind() == "\"" && !n.is_missing())?;
                if open.id() == close.id() {
                    return None;
                }
                let holes = holes(&parts, src)?;
                let delim = Delim {
                    kind: DelimKind::ArgvString,
                    open: span(*open),
                    body: Span::new(open.end_byte(), close.start_byte()),
                    close: span(*close),
                };
                return Some((delim, holes));
            }
            _ => {}
        }
    }
    // A bare word, maybe concatenated (`foo$bar`): no delimiter of its
    // own, so both ends are empty spans around the whole word.
    let bare = word.iter().all(|n| {
        matches!(
            n.kind(),
            "simple_word" | "variable_substitution" | "command_substitution"
        )
    });
    if !bare {
        return None;
    }
    let holes = holes(word, src)?;
    let (start, end) = (first.start_byte(), last.end_byte());
    let delim = Delim {
        kind: DelimKind::ArgvString,
        open: Span::new(start, start),
        body: Span::new(start, end),
        close: Span::new(end, end),
    };
    Some((delim, holes))
}

/// The substitutions among `parts` -- Tcl's `$var` and `[cmd]`, host
/// interpolations the guest never sees as written
/// (`languages/api/src/holes:V40`) -- or `None` when a backslash escape
/// makes the word's bytes differ from the argument.
fn holes(parts: &[Node<'_>], src: &str) -> Option<Vec<Span>> {
    let mut out = Vec::new();
    for part in parts {
        match part.kind() {
            "escaped_character" => return None,
            "variable_substitution" | "command_substitution" => out.push(hole(*part, src)),
            _ => {}
        }
    }
    Some(out)
}

/// A hole's span: its node, minus leading whitespace. The grammar starts
/// a `[cmd]` that follows other text in a `"..."` at the space before it
/// (measured: in `"echo $x [pwd]"` the substitution spans ` [pwd]`), and
/// the space is the guest's text, not the host's interpolation.
fn hole(node: Node<'_>, src: &str) -> Span {
    let whole = text(node, src);
    let lead = whole.len() - whole.trim_start().len();
    Span::new(node.start_byte() + lead, node.end_byte())
}

/// What the walk collects: the sites, and the inside of every braced word
/// it treated as data rather than script.
#[derive(Default)]
struct Walk {
    sites: Vec<Site>,
    opaque: Vec<Span>,
}

impl Walk {
    /// Every site at or under `node`, which is in a SCRIPT position.
    fn node(&mut self, node: Node<'_>, src: &str) {
        match node.kind() {
            "comment" => {}
            "command" => self.command(node, src),
            "namespace" => self.namespace(node, src),
            "braced_word_simple" => self.opaque(node),
            _ => {
                for child in children(node) {
                    self.node(child, src);
                }
            }
        }
    }

    /// A braced word read as data: remembered, never entered.
    fn opaque(&mut self, node: Node<'_>) {
        if let Some((open, close)) = braced_inside(node) {
            self.opaque
                .push(Span::new(open.end_byte(), close.start_byte()));
        }
    }

    /// An argument of a command: a braced one is data, anything else may
    /// hold a `[cmd]` Tcl runs first.
    fn argument(&mut self, node: Node<'_>, src: &str) {
        if node.kind() == "braced_word" {
            self.opaque(node);
        } else {
            self.node(node, src);
        }
    }

    /// An ordinary command: its site, if it is one, then its words.
    fn command(&mut self, node: Node<'_>, src: &str) {
        let mut cursor = node.walk();
        let name: Vec<Node<'_>> = node.children_by_field_name("name", &mut cursor).collect();
        let list: Vec<Node<'_>> = node
            .child_by_field_name("arguments")
            .map(children)
            .unwrap_or_default();
        if let Some(site) = site(&name, &list, src) {
            self.sites.push(site);
        }
        for part in name.iter().chain(&list) {
            self.argument(*part, src);
        }
    }

    /// `namespace eval NAME {...}` runs its body; any other subcommand's
    /// braced words are data.
    fn namespace(&mut self, node: Node<'_>, src: &str) {
        let list: Vec<Node<'_>> = children(node)
            .into_iter()
            .filter(|n| n.kind() == "word_list")
            .flat_map(children)
            .collect();
        let eval = words(&list).first().and_then(|w| plain(w, src)) == Some("eval");
        for part in list {
            if eval {
                self.node(part, src);
            } else {
                self.argument(part, src);
            }
        }
    }
}

/// The site of `exec`/`spawn` with these name and argument nodes, if the
/// argv holds a program (`languages/shells/tcl:V196`).
fn site(name: &[Node<'_>], list: &[Node<'_>], src: &str) -> Option<Site> {
    let runner = Runner::of(plain(name, src)?)?;
    let words = words(list);
    let (first, rest) = words.split_first()?;
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

/// Whether every `ERROR` and `MISSING` node under `node` lies inside one
/// of `opaque` -- braced data the grammar misread as script.
fn errors_contained(node: Node<'_>, opaque: &[Span]) -> bool {
    if node.is_error() || node.is_missing() {
        return opaque
            .iter()
            .any(|o| o.start <= node.start_byte() && node.end_byte() <= o.end);
    }
    if !node.has_error() {
        return true;
    }
    children(node)
        .into_iter()
        .all(|child| errors_contained(child, opaque))
}
