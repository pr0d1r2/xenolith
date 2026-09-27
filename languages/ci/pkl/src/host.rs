//! Pkl as a host: hk steps holding shell (`languages/ci/pkl:T13`).
//!
//! A site is a delimiter AND a sink context (`languages/api/src/site:V38`),
//! and in pkl the context has three layers, all read from the tree:
//!
//! 1. the module `amends` hk's `Config.pkl` -- it is a hk config, so the
//!    property names below mean what hk says they mean;
//! 2. the property sits directly in a step ENTRY, `["name"] { … }`,
//!    whose key is a plain string -- that name is the step, and the
//!    report points back to it;
//! 3. the property is one of hk's command properties, [`SINKS`]
//!    (`languages/ci/pkl` §I).
//!
//! Drop any layer and a `"""…"""` is inert data (`languages:V2`): the
//! same shell in a `message`, in a top-level `local`, in a module that is
//! not a hk config, or in a comment -- which the grammar never hands us
//! as a string at all.
//!
//! The delimiter is the pkl multi-line string, `"""…"""` or `#"""…"""#`
//! (`languages/api/src/site` §I). A single-line `"…"` has no delimiter
//! kind in the api, so it is never a site here; it is where a LOAD lives,
//! `sh ./scripts/hk/<name>.sh {{files}}` -- run by the interpreter the
//! extract's prelude names, `sh` under hk's default step shell
//! (`languages/ci/pkl:V52`, `languages/ci/pkl:V172`).

use std::path::{Path, PathBuf};

use tree_sitter::{Node, Parser, Tree};
use xenolith_lang_api::{
    Delim, DelimKind, Error, Host, Invoke, LangId, LintCmd, LoadRef, Placement, Result, Site, Span,
};

use crate::{grammar, placement, shell, string};

#[cfg(test)]
mod tests;

/// hk step properties that hold a command (`languages/ci/pkl` §I), in name
/// order.
pub const SINKS: &[&str] = &["check", "check_diff", "check_list_files", "fix", "shell"];

/// hk's template for the files a step runs on. A load forwards it, so an
/// extracted script receives the same file list the inline body did
/// (`languages/ci/pkl:V52`).
const FILES: &str = "{{files}}";

/// Interpreters a load may name, and the extensions their scripts carry.
/// Recognised, never emitted: the argv of a load is the guest's
/// `invoke` (`languages/api:V35`), and this list only lets `loads` read
/// one back.
const INTERPRETERS: &[&str] = &["bash", "sh", "zsh"];
const SCRIPT_EXTENSIONS: &[&str] = &["bash", "sh", "zsh"];

/// The pkl host.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PklHost;

/// One hk command property: which step, which property, and its value.
struct Sink<'t> {
    step: String,
    property: String,
    value: Node<'t>,
    /// The step's `["name"] { … }` entry and its body, where hk's shell
    /// for it is read (`languages/ci/pkl:V172`).
    entry: Node<'t>,
    body: Node<'t>,
}

pub(crate) fn parse(src: &str) -> Result<Tree> {
    let mut parser = Parser::new();
    parser
        .set_language(&grammar::language())
        .map_err(|e| Error::parse(LangId::Pkl, e.to_string()))?;
    parser
        .parse(src, None)
        .ok_or_else(|| Error::parse(LangId::Pkl, "the parser returned no tree"))
}

pub(crate) fn text<'s>(node: Node<'_>, src: &'s str) -> &'s str {
    src.get(node.byte_range()).unwrap_or_default()
}

fn span(node: Node<'_>) -> Span {
    Span::new(node.start_byte(), node.end_byte())
}

/// The content of a single-line string with no escapes and no
/// interpolation, or `None` for any other node.
pub(crate) fn plain_string<'s>(node: Node<'_>, src: &'s str) -> Option<&'s str> {
    if !matches!(node.kind(), "slStringLiteralExpr" | "stringConstant") {
        return None;
    }
    let mut cursor = node.walk();
    let mut parts = node.named_children(&mut cursor);
    match (parts.next(), parts.next()) {
        (Some(part), None) if part.kind() == "slStringLiteralPart" => Some(text(part, src)),
        _ => None,
    }
}

/// Layer 1: whether the module amends hk's `Config.pkl` -- a package URI
/// (`…/hk@1.2.0#/Config.pkl`) or a vendored copy (`pkl/Config.pkl`).
fn is_hk_config(root: Node<'_>, src: &str) -> bool {
    let mut cursor = root.walk();
    let Some(header) = root
        .named_children(&mut cursor)
        .find(|child| child.kind() == "moduleHeader")
    else {
        return false;
    };
    let mut cursor = header.walk();
    header
        .named_children(&mut cursor)
        .filter(|clause| clause.kind() == "extendsOrAmendsClause")
        .filter(|clause| clause.child(0).is_some_and(|kw| kw.kind() == "amends"))
        .filter_map(|clause| {
            let mut cursor = clause.walk();
            clause
                .named_children(&mut cursor)
                .find_map(|child| plain_string(child, src))
        })
        .any(|uri| uri.rsplit(['/', '#', ':']).next() == Some("Config.pkl"))
}

/// Layers 2 and 3 for one `objectProperty`: the step name and property
/// name when it is a hk command property inside a step entry.
fn as_sink<'t>(property: Node<'t>, src: &str) -> Option<Sink<'t>> {
    let mut cursor = property.walk();
    let named: Vec<Node<'t>> = property.named_children(&mut cursor).collect();
    let [name, .., value] = named.as_slice() else {
        return None;
    };
    // `local check = …` inside a step is a private helper, not the step's
    // command.
    if name.kind() != "identifier" || !SINKS.contains(&text(*name, src)) {
        return None;
    }
    let body = property.parent().filter(|p| p.kind() == "objectBody")?;
    let entry = body.parent().filter(|e| e.kind() == "objectEntry")?;
    let key = entry.named_child(0)?;
    let step = plain_string(key, src).filter(|step| !step.is_empty())?;
    Some(Sink {
        step: step.to_owned(),
        property: text(*name, src).to_owned(),
        value: *value,
        entry,
        body,
    })
}

/// Every hk command property in the tree, in source order.
///
/// ERROR regions are not entered (`languages:V78`): spans inside them are
/// the parser's best guess, and a site built on a guess rewrites the
/// wrong bytes.
fn sinks<'t>(tree: &'t Tree, src: &str) -> Vec<Sink<'t>> {
    fn walk<'t>(node: Node<'t>, src: &str, out: &mut Vec<Sink<'t>>) {
        if node.is_error() || node.is_missing() {
            return;
        }
        if node.kind() == "objectProperty"
            && let Some(sink) = as_sink(node, src).filter(|sink| !sink.value.has_error())
        {
            out.push(sink);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            walk(child, src, out);
        }
    }

    let root = tree.root_node();
    let mut out = Vec::new();
    if is_hk_config(root, src) {
        walk(root, src, &mut out);
    }
    out
}

/// The site a multi-line sink value makes, or `None` for any other value.
fn site(sink: &Sink<'_>, src: &str) -> Option<Site> {
    let literal = sink.value;
    if literal.kind() != "mlStringLiteralExpr" {
        return None;
    }
    let open = literal.child(0)?;
    let close = literal.child(literal.child_count().checked_sub(1)?)?;
    let pounds = text(open, src).chars().filter(|&ch| ch == '#').count();
    let mut cursor = literal.walk();
    let holes = literal
        .named_children(&mut cursor)
        .filter(|child| child.kind() == "stringInterpolation")
        .map(span)
        .collect();
    Some(Site {
        sink: format!("{}.{}", sink.step, sink.property),
        guest: LangId::Shell,
        // The shell hk runs this step under, never the guest's default,
        // whose strict line the inline step did not have
        // (`languages/ci/pkl:V172`).
        env: shell::env(sink.entry, sink.body, src),
        delim: Delim {
            kind: DelimKind::PklMultiline { pounds },
            open: span(open),
            body: Span::new(open.end_byte(), close.start_byte()),
            close: span(close),
        },
        holes,
    })
}

/// The load a single-line sink value holds: `<interpreter> <script>`,
/// optionally followed by `{{files}}`, and nothing else.
fn load(sink: &Sink<'_>, src: &str) -> Option<LoadRef> {
    let command = plain_string(sink.value, src)?;
    let words: Vec<&str> = command.split_whitespace().collect();
    let [interpreter, script, rest @ ..] = words.as_slice() else {
        return None;
    };
    if !INTERPRETERS.contains(interpreter) || !matches!(rest, [] | [FILES]) {
        return None;
    }
    let path = PathBuf::from(script);
    let extension = path.extension().and_then(|ext| ext.to_str())?;
    SCRIPT_EXTENSIONS.contains(&extension).then(|| LoadRef {
        span: span(sink.value),
        path,
        guest: LangId::Shell,
    })
}

/// Whether an argv word can go into a load as it is: no shell quoting
/// and no pkl escaping needed, so `loads` reads back exactly what
/// `rewrite` wrote.
fn plain_word(word: &str) -> bool {
    !word.is_empty()
        && word
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || "/._-+,:@=%".contains(ch))
}

/// The line break the host uses where `at` sits: the one ending its
/// line, else the one before it, else `\n` for a one-line file
/// (`languages/ci/pkl:B2`). The literal `inline` writes breaks its lines the
/// same way, so a CRLF config stays CRLF -- and evaluates the same, a
/// CRLF in a pkl string being one `\n` (`languages/ci/pkl:B1`).
fn line_break(src: &str, at: Span) -> &'static str {
    let crlf = |newline: usize| newline > 0 && src.as_bytes().get(newline - 1) == Some(&b'\r');
    let after = src.get(at.end..).and_then(|rest| rest.find('\n'));
    let before = src.get(..at.start).and_then(|head| head.rfind('\n'));
    match (after.map(|offset| at.end + offset), before) {
        (Some(newline), _) | (None, Some(newline)) if crlf(newline) => "\r\n",
        _ => "\n",
    }
}

/// `src` with `[at.start, at.end)` replaced by `with`.
fn splice(src: &str, at: Span, with: &str) -> Result<String> {
    let before = src.get(..at.start);
    let after = src.get(at.end..);
    let (Some(before), Some(after)) = (before, after) else {
        return Err(Error::parse(LangId::Pkl, "span outside the source"));
    };
    Ok(format!("{before}{with}{after}"))
}

impl Host for PklHost {
    fn id(&self) -> LangId {
        LangId::Pkl
    }

    /// `*.pkl`, and `PklProject`, which is pkl without the extension.
    /// No shebang: pkl files are evaluated, not executed.
    fn claims(&self, path: &Path, _head: &str) -> bool {
        path.extension().is_some_and(|ext| ext == "pkl")
            || path.file_name().is_some_and(|name| name == "PklProject")
    }

    fn sites(&self, src: &str) -> Result<Vec<Site>> {
        let tree = parse(src)?;
        Ok(sinks(&tree, src)
            .iter()
            .filter_map(|sink| site(sink, src))
            .collect())
    }

    fn loads(&self, src: &str) -> Result<Vec<LoadRef>> {
        let tree = parse(src)?;
        Ok(sinks(&tree, src)
            .iter()
            .filter_map(|sink| load(sink, src))
            .collect())
    }

    /// The multi-line string becomes `"<invoke argv> {{files}}"`
    /// (`languages/ci/pkl:V52`): hk passes the step's files through, so the
    /// script sees what the inline body saw.
    fn rewrite(&self, src: &str, site: &Site, invoke: &Invoke, _path: &Path) -> Result<String> {
        if !self.sites(src)?.contains(site) {
            return Err(Error::parse(
                LangId::Pkl,
                format!("no site `{}` at bytes {:?}", site.sink, site.delim.open),
            ));
        }
        // A `\(…)` is pkl: in the script it would be text. Holes go through
        // `rewrite_bound`, and this host has no load that passes a param
        // yet, so the api's default refuses them (`languages/ci/pkl:V171`,
        // `languages/api/src/holes:V40`).
        if !site.holes.is_empty() {
            return Err(Error::unsupported(
                LangId::Pkl,
                "rewrite of a string with holes",
            ));
        }
        // A word needing quotes would need shell quoting inside pkl
        // escaping, and `loads` could no longer read the result back:
        // refusing is better than a load `xnl graph` cannot follow.
        if invoke.argv.is_empty() || !invoke.argv.iter().all(|word| plain_word(word)) {
            return Err(Error::unsupported(LangId::Pkl, "rewrite"));
        }
        let command = format!("\"{} {FILES}\"", invoke.argv.join(" "));
        splice(
            src,
            Span::new(site.delim.open.start, site.delim.close.end),
            &command,
        )
    }

    /// The load's string becomes a multi-line literal holding `body`,
    /// indented one step past the property's line, its lines broken the
    /// way the host's are (`languages/ci/pkl:B2`).
    fn inline(&self, src: &str, load: &LoadRef, body: &str) -> Result<String> {
        if !self.loads(src)?.contains(load) {
            return Err(Error::parse(
                LangId::Pkl,
                format!("no load of {} at {:?}", load.path.display(), load.span),
            ));
        }
        let line_start = src
            .get(..load.span.start)
            .and_then(|before| before.rfind('\n'))
            .map_or(0, |newline| newline + 1);
        let line = src.get(line_start..load.span.start).unwrap_or_default();
        let indent: String = line
            .chars()
            .take_while(|&ch| ch == ' ' || ch == '\t')
            .collect();
        let literal = string::multiline(body, &format!("{indent}  "));
        splice(
            src,
            load.span,
            &literal.replace('\n', line_break(src, load.span)),
        )
    }

    /// [`string::unescape`]: the closing line's indent stripped and the
    /// escapes decoded, as pkl evaluates the literal
    /// (`languages/api/src/lens:V39`).
    fn unescape(&self, delim: &Delim, raw: &str) -> Result<String> {
        string::unescape(delim, raw)
    }

    /// [`string::escape`]: the inverse, under the delimiter's own `#`
    /// count (`languages/ci/pkl:V171`).
    fn escape(&self, delim: &Delim, body: &str) -> Result<String> {
        string::escape(delim, body)
    }

    /// None yet, as a statement rather than a gap: the api lists
    /// `pkl format --diff` for pkl hosts with a `?` (`languages/api` §I),
    /// and a check this crate is unsure of would be a finding nobody can
    /// trust.
    fn checks(&self) -> Vec<LintCmd> {
        Vec::new()
    }

    /// None yet, for the same reason as [`PklHost::checks`].
    fn fixers(&self) -> Vec<LintCmd> {
        Vec::new()
    }

    /// `scripts/hk/<step>`, the step key kebab-cased
    /// (`languages/ci/pkl:V52`); `rewrite` then forwards `{{files}}` to it.
    fn placement(&self, site: &Site) -> Result<Placement> {
        Ok(placement::placement(site))
    }
}
