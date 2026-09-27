//! Holes carried by text (`languages/nix:V174`): a site whose `${…}`
//! holes `holes::bind` named as params loads through
//!
//! ```nix
//! builtins.replaceStrings [ "__NAME__" … ] [ "<hole>" … ] (<V53 load>)
//! ```
//!
//! and its extract holds `__NAME__` wherever a hole was. The load is a
//! string operation over what `readFile` read -- no derivation is built
//! to evaluate it, unlike `replaceVars` (import from derivation), and
//! each hole's string context, the store paths it names, stays on the
//! result. What nix runs is the text the `''…''` string evaluated to,
//! so the site's semantics are unchanged: a hole in single quotes or a
//! heredoc is substituted as text there, as it was interpolated before.
//!
//! The one way this can go wrong is a `__NAME__` the author wrote: the
//! substitution fills it too. So every body is PROVEN before it is
//! used: substituting a plain marker for each pattern must give exactly
//! the body with a marker where each hole was ([`body`]).

use rnix::{SyntaxKind, SyntaxNode};
use xenolith_lang_api::holes::{self, Hole, Param};
use xenolith_lang_api::{Delim, Error, GuestEnv, LangId, Result, Site, Span};

use crate::{delim, loads, span, unescape};

#[cfg(test)]
mod tests;

/// The function the load is wrapped in.
const REPLACE_STRINGS: &[&str] = &["builtins", "replaceStrings"];

/// A refusal naming what was refused.
const fn refuse(operation: &'static str) -> Error {
    Error::unsupported(LangId::Nix, operation)
}

/// `__NAME__`, or `None` for a name that is not a plain word: it goes
/// between the quotes of a `"…"` string as written, and into the extract
/// as one shell word. Not nix's usual `@NAME@`: tree-sitter-bash cannot
/// read `@NAME@` as a command name, and a hole is most often one
/// (`${pkgs.hello}/bin/hello`), so `xnl check` would call the extract
/// unparseable.
pub(crate) fn pattern(name: &str) -> Option<String> {
    let plain = !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_');
    plain.then(|| format!("__{name}__"))
}

/// The name a pattern of [`pattern`]'s making stands for.
pub(crate) fn name_of(pattern: &str) -> Option<&str> {
    let name = pattern.strip_prefix("__")?.strip_suffix("__")?;
    (self::pattern(name).as_deref() == Some(pattern)).then_some(name)
}

/// nix's `builtins.replaceStrings from to s`: left to right, the first
/// pattern in list order that matches at a position wins, and what it
/// put in is never scanned again. Empty patterns are skipped; nothing
/// here writes one.
pub(crate) fn replace_strings(s: &str, from: &[String], to: &[String]) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    'scan: while !rest.is_empty() {
        for (pattern, with) in from.iter().zip(to) {
            if !pattern.is_empty()
                && let Some(after) = rest.strip_prefix(pattern.as_str())
            {
                out.push_str(with);
                rest = after;
                continue 'scan;
            }
        }
        let mut chars = rest.chars();
        if let Some(ch) = chars.next() {
            out.push(ch);
        }
        rest = chars.as_str();
    }
    out
}

/// The load text: `builtins.replaceStrings [ "__A__" … ] [ "${a}" … ]
/// <load>`, `load` already parenthesised.
pub(crate) fn call(pairs: &[(String, String)], load: &str) -> String {
    let list = |items: Vec<&str>| {
        let quoted: Vec<String> = items.iter().map(|item| format!("\"{item}\"")).collect();
        format!("[ {} ]", quoted.join(" "))
    };
    let patterns = list(pairs.iter().map(|(p, _)| p.as_str()).collect());
    let holes = list(pairs.iter().map(|(_, h)| h.as_str()).collect());
    format!("{} {patterns} {holes} {load}", REPLACE_STRINGS.join("."))
}

/// The raw body between `body`'s bounds with every occurrence of hole
/// `i` replaced by `with[i]`.
fn marked(src: &str, body: Span, holes: &[Hole], with: &[String]) -> String {
    let mut spans: Vec<(Span, &str)> = holes
        .iter()
        .zip(with)
        .flat_map(|(hole, text)| hole.spans.iter().map(move |s| (*s, text.as_str())))
        .collect();
    spans.sort_unstable();
    let mut out = String::new();
    let mut at = body.start;
    for (hole, text) in spans {
        out.push_str(src.get(at..hole.start).unwrap_or_default());
        out.push_str(text);
        at = hole.end;
    }
    out.push_str(src.get(at..body.end).unwrap_or_default());
    out
}

/// The body a string holds with each hole written `__NAME__` -- `names`
/// in the order of `holes` -- proven to read back through
/// `replaceStrings`: filling each pattern with a plain marker gives the
/// body with that marker where each hole was, which is what nix
/// evaluates with the hole's value there.
///
/// # Errors
///
/// [`Error::Unsupported`] for a name that is no plain word, or a body
/// the substitution would change elsewhere too (a `__NAME__` of the
/// author's); whatever `unescape` fails with.
pub(crate) fn body(src: &str, delim: &Delim, holes: &[Hole], names: &[String]) -> Result<String> {
    let patterns = names
        .iter()
        .map(|name| pattern(name))
        .collect::<Option<Vec<String>>>()
        .ok_or(refuse(
            "rewrite of a hole whose param name is no plain word",
        ))?;
    let markers: Vec<String> = (0..holes.len()).map(holes::marker).collect();
    let text = unescape::unescape(&delim.kind, &marked(src, delim.body, holes, &patterns))?;
    let expect = unescape::unescape(&delim.kind, &marked(src, delim.body, holes, &markers))?;
    if replace_strings(&text, &patterns, &markers) != expect {
        return Err(refuse(
            "rewrite of holes `replaceStrings` would not put back",
        ));
    }
    Ok(text)
}

/// Whether every hole of `site` inside its body is one of `holes`' own
/// occurrences -- a hole `collect` skipped would stay in the body as
/// `${…}`, host syntax in the extract (`languages/api/src/holes:V40`).
fn covered(site: &Site, holes: &[Hole]) -> bool {
    let body = site.delim.body;
    site.holes
        .iter()
        .filter(|h| h.start >= body.start && h.end <= body.end)
        .all(|h| {
            holes
                .iter()
                .flat_map(|found| &found.spans)
                .any(|s| s.start == h.start)
        })
}

/// The extract body of `site` under `params`, and the (pattern, hole)
/// pairs its load passes, in the params' order.
///
/// # Errors
///
/// [`Error::Unsupported`] when the params are not the site's holes as
/// `holes::collect` finds them, and whatever [`body`] refuses.
pub(crate) fn bind(
    src: &str,
    site: &Site,
    params: &[Param],
) -> Result<(String, Vec<(String, String)>)> {
    let holes = holes::collect(src, site);
    let named = holes.len() == params.len()
        && holes.iter().zip(params).all(|(h, p)| h.text() == p.hole)
        && covered(site, &holes);
    if !named {
        return Err(refuse("rewrite of holes the params do not name"));
    }
    let named_as: Vec<String> = params.iter().map(|p| p.name.clone()).collect();
    let text = body(src, &site.delim, &holes, &named_as)?;
    let pairs = params
        .iter()
        .filter_map(|p| Some((pattern(&p.name)?, p.hole.clone())))
        .collect();
    Ok((text, pairs))
}

/// The pairs a `"…"` string of `inline`'s making reads back as, with the
/// holes it holds: `raw` must be the body `body` returns for them.
///
/// # Errors
///
/// [`Error::Parse`] when the string's holes are not the pairs' holes or
/// its text would not read back as `expected`.
pub(crate) fn read_back(
    src: &str,
    string: &SyntaxNode,
    pairs: &[(String, String)],
    expected: &str,
) -> Result<()> {
    let unreadable = || Error::parse(LangId::Nix, "the inlined string would not read back");
    let (delim, spans) = delim(string).ok_or_else(unreadable)?;
    let site = Site {
        sink: String::new(),
        guest: LangId::Shell,
        env: GuestEnv::default(),
        delim,
        holes: spans,
    };
    let found = holes::collect(src, &site);
    if !covered(&site, &found) {
        return Err(unreadable());
    }
    let names = found
        .iter()
        .map(|hole| {
            let text = hole.text();
            let (pattern, _) = pairs.iter().find(|(_, h)| *h == text)?;
            name_of(pattern).map(str::to_owned)
        })
        .collect::<Option<Vec<String>>>()
        .ok_or_else(unreadable)?;
    match body(src, &site.delim, &found, &names) {
        Ok(text) if text == expected => Ok(()),
        _ => Err(unreadable()),
    }
}

/// The call `load` -- a V53 load's application node -- is the last
/// argument of, when that call is one `rewrite_bound` writes: its span
/// is then the load's (`languages/nix:V174`).
pub(crate) fn wrapper(load: &SyntaxNode) -> Option<SyntaxNode> {
    let paren = load
        .parent()
        .filter(|p| p.kind() == SyntaxKind::NODE_PAREN)?;
    let outer = paren
        .parent()
        .filter(|p| p.kind() == SyntaxKind::NODE_APPLY)?;
    let (_, inner) = parts(&outer)?;
    (inner == *load).then_some(outer)
}

/// The (pattern, hole) pairs of a call `rewrite_bound` writes, and the
/// load inside it, or `None` for any other expression: `outer` must be
/// `builtins.replaceStrings <patterns> <holes> (<apply>)`, with as many
/// `"__NAME__"` patterns as holes, and each hole a `"…"` string holding
/// one `${…}` and at most a path tail after it -- the text that means
/// the same inside a `''…''` string, where `inline` puts it back.
pub(crate) fn parts(outer: &SyntaxNode) -> Option<(Vec<(String, String)>, SyntaxNode)> {
    if outer.kind() != SyntaxKind::NODE_APPLY {
        return None;
    }
    let (with_holes, argument) = two(outer)?;
    let inner = only_child(&argument, SyntaxKind::NODE_PAREN)?;
    if inner.kind() != SyntaxKind::NODE_APPLY {
        return None;
    }
    let (with_patterns, holes) = two(&with_holes)?;
    let (function, patterns) = two(&with_patterns)?;
    if with_holes.kind() != SyntaxKind::NODE_APPLY
        || with_patterns.kind() != SyntaxKind::NODE_APPLY
        || loads::dotted(&function)? != REPLACE_STRINGS
    {
        return None;
    }
    let patterns = list(&patterns, |s| {
        let text = plain(s)?;
        name_of(&text)?;
        Some(text)
    })?;
    let holes = list(&holes, hole)?;
    if patterns.is_empty() || patterns.len() != holes.len() {
        return None;
    }
    Some((patterns.into_iter().zip(holes).collect(), inner))
}

/// The two children of an application: function and argument.
fn two(node: &SyntaxNode) -> Option<(SyntaxNode, SyntaxNode)> {
    let mut children = node.children();
    let pair = (children.next()?, children.next()?);
    children.next().is_none().then_some(pair)
}

/// The one child of a node of `kind`.
fn only_child(node: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if node.kind() != kind {
        return None;
    }
    let mut children = node.children();
    let child = children.next()?;
    children.next().is_none().then_some(child)
}

/// Each element of a list literal read by `read`, or `None` when any
/// element is not what `read` accepts.
fn list(node: &SyntaxNode, read: impl Fn(&SyntaxNode) -> Option<String>) -> Option<Vec<String>> {
    if node.kind() != SyntaxKind::NODE_LIST {
        return None;
    }
    node.children().map(|item| read(&item)).collect()
}

/// The content of a `"…"` string with no hole and no escape.
fn plain(string: &SyntaxNode) -> Option<String> {
    let raw = quoted(string)?;
    let simple = !raw.contains('\\') && !raw.contains("${");
    simple.then_some(raw)
}

/// The inside of a `"…"` string node, as written.
fn quoted(string: &SyntaxNode) -> Option<String> {
    if string.kind() != SyntaxKind::NODE_STRING {
        return None;
    }
    let text = string.text().to_string();
    let raw = text.strip_prefix('"')?.strip_suffix('"')?;
    Some(raw.to_owned())
}

/// A hole as `rewrite_bound` writes one: a `"…"` string that is one
/// `${…}` right after its quote, then at most a path tail.
fn hole(string: &SyntaxNode) -> Option<String> {
    let raw = quoted(string)?;
    let mut holes = string
        .children()
        .filter(|c| c.kind() == SyntaxKind::NODE_INTERPOL);
    let only = holes.next()?;
    if holes.next().is_some() {
        return None;
    }
    let at = span(only.text_range());
    let start = span(string.text_range()).start + 1;
    let tail = raw.get(at.end - start..)?;
    let path = |ch: char| ch.is_ascii_alphanumeric() || "._+/-".contains(ch);
    (at.start == start && tail.chars().all(path)).then_some(raw)
}
