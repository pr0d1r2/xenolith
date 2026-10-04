//! What the grammar says about a justfile: its recipes and the settings
//! that decide how their bodies run.
//!
//! Everything here is read off tree nodes (`languages:V2`,
//! `languages/api/src/site:V38`); the only text this module looks at is
//! the text a node already bounds.

use tree_sitter::{Node, Parser, Tree};
use xenolith_lang_api::{Error, LangId, Result, Span};

use crate::grammar;

#[cfg(test)]
mod tests;

/// The tree for `src`, refusing a file with any parse error: its spans
/// are the parser's guesses, and a site built on a guess rewrites the
/// wrong bytes (`languages:V78`). The engine then applies `[parse]
/// host_errors` to the file.
pub(crate) fn parse(src: &str) -> Result<Tree> {
    let mut parser = Parser::new();
    parser
        .set_language(&grammar::language())
        .map_err(|e| Error::parse(LangId::Just, e.to_string()))?;
    let tree = parser
        .parse(src, None)
        .ok_or_else(|| Error::parse(LangId::Just, "the parser returned no tree"))?;
    if tree.root_node().has_error() {
        return Err(Error::parse(
            LangId::Just,
            "the file is not valid just, so none of its recipes were read",
        ));
    }
    Ok(tree)
}

/// The text `node` bounds.
pub(crate) fn text<'s>(node: Node<'_>, src: &'s str) -> &'s str {
    src.get(node.byte_range()).unwrap_or_default()
}

/// The span `node` bounds.
pub(crate) fn span(node: Node<'_>) -> Span {
    Span::new(node.start_byte(), node.end_byte())
}

/// The span of a body line: the node's, less the `\r` of a CRLF line
/// break, which the grammar keeps inside the line and which is the
/// host's line break rather than the command's.
pub(crate) fn line_span(line: Node<'_>, src: &str) -> Span {
    let end = line.end_byte();
    let cr = end > line.start_byte() && src.as_bytes().get(end - 1) == Some(&b'\r');
    Span::new(line.start_byte(), if cr { end - 1 } else { end })
}

/// The text of [`line_span`].
pub(crate) fn line_text<'s>(line: Node<'_>, src: &'s str) -> &'s str {
    line_span(line, src).of(src).unwrap_or_default()
}

/// Every named child of `node`, in source order.
fn children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}

/// One recipe, as far as a site needs it.
#[derive(Debug, Clone)]
pub(crate) struct Recipe<'t> {
    /// The recipe's name, as written in its header.
    pub name: String,
    /// The header: `@name params: deps`.
    pub header: Node<'t>,
    /// The `#!` line opening the body, if any.
    pub shebang: Option<Node<'t>>,
    /// The body's lines, in order.
    pub lines: Vec<Node<'t>>,
    /// Whether the recipe takes parameters.
    pub params: bool,
    /// Whether a `[script]` attribute makes the body one script.
    pub script: bool,
    /// Whether a `[positional-arguments]` attribute passes its args to
    /// the shell.
    pub positional: bool,
}

impl Recipe<'_> {
    /// The body's bytes: from the start of the first line's LINE, its
    /// indent included, to the end of the last line. `None` for a recipe
    /// with no body.
    pub(crate) fn body(&self, src: &str) -> Option<Span> {
        let first = self.shebang.or_else(|| self.lines.first().copied())?;
        let last = self.lines.last().copied().or(self.shebang)?;
        let start = src
            .get(..first.start_byte())
            .and_then(|before| before.rfind('\n'))
            .map_or(0, |newline| newline + 1);
        Some(Span::new(start, line_span(last, src).end))
    }
}

/// Every recipe in the file, in source order.
pub(crate) fn recipes<'t>(tree: &'t Tree, src: &str) -> Vec<Recipe<'t>> {
    children(tree.root_node())
        .into_iter()
        .filter(|node| node.kind() == "recipe")
        .filter_map(|node| recipe(node, src))
        .collect()
}

fn recipe<'t>(node: Node<'t>, src: &str) -> Option<Recipe<'t>> {
    let parts = children(node);
    let header = parts
        .iter()
        .copied()
        .find(|n| n.kind() == "recipe_header")?;
    let name = header.child_by_field_name("name")?;
    let attributes: Vec<&str> = parts
        .iter()
        .filter(|n| n.kind() == "attribute")
        .flat_map(|n| children(*n))
        .filter(|n| n.kind() == "identifier")
        .map(|n| text(n, src))
        .collect();
    let body = parts.iter().copied().find(|n| n.kind() == "recipe_body");
    let shebang = body.and_then(|b| b.child_by_field_name("shebang"));
    let lines = body.map_or_else(Vec::new, |b| {
        children(b)
            .into_iter()
            .filter(|n| n.kind() == "recipe_line")
            .collect()
    });
    Some(Recipe {
        name: text(name, src).to_owned(),
        header,
        shebang,
        lines,
        params: children(header).iter().any(|n| n.kind() == "parameters"),
        script: attributes.contains(&"script"),
        positional: attributes.contains(&"positional-arguments"),
    })
}

/// The line's `@`/`-` prefix as written, or `""`.
pub(crate) fn prefix<'s>(line: Node<'_>, src: &'s str) -> &'s str {
    children(line)
        .into_iter()
        .find(|n| n.kind() == "recipe_line_prefix")
        .map_or("", |n| text(n, src))
}

/// The `{{...}}` interpolations of a line, in order: the grammar's
/// `interpolation` nodes that just's lexer also starts one at.
///
/// The grammar lexes `{{{{` -- just's escape for a literal `{{` -- by
/// context, and in `x{{{{y}}` it reads an interpolation `{{y}}` just
/// never evaluates (`languages/ci/just:B2`). just reads a line left to
/// right: `{{{{` is literal, `{{` opens an interpolation. Walking the
/// line that way and keeping only the nodes it lands on DROPS the
/// grammar's inventions. A `{{` just would open where the grammar has no
/// node is kept as a two-byte hole: the grammar missed an interpolation,
/// and a hole is refused by the extract direction where text would be
/// merged into a script as if just never evaluated it.
pub(crate) fn interpolations(line: Node<'_>, src: &str) -> Vec<Span> {
    let nodes: Vec<Span> = children(line)
        .into_iter()
        .filter(|n| n.kind() == "interpolation")
        .map(span)
        .collect();
    let bytes = src.as_bytes();
    let opens = |at: usize, n: usize| (at..at + n).all(|i| bytes.get(i) == Some(&b'{'));
    let mut out = Vec::new();
    let mut at = line.start_byte();
    while at < line.end_byte() {
        if opens(at, 4) {
            at += 4;
        } else if let Some(node) = nodes.iter().find(|n| n.start == at) {
            out.push(*node);
            at = node.end;
        } else if opens(at, 2) {
            out.push(Span::new(at, at + 2));
            at += 2;
        } else {
            at += 1;
        }
    }
    out
}

/// The settings that decide how a recipe line runs
/// (`languages/ci/just:V179`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Settings {
    /// `set shell := [...]`: each element's string node text, quotes
    /// included; `None` when the file has no such setting.
    pub shell: Option<Vec<String>>,
    /// `set windows-shell` or `set windows-powershell` is present.
    pub windows_shell: bool,
    /// `set positional-arguments`, unless set to `false`.
    pub positional: bool,
    /// The file imports another, whose settings it cannot see.
    pub imports: bool,
}

/// The file's settings.
pub(crate) fn settings(tree: &Tree, src: &str) -> Settings {
    let mut out = Settings::default();
    for node in children(tree.root_node()) {
        match node.kind() {
            "import" => out.imports = true,
            "setting" => setting(node, src, &mut out),
            _ => {}
        }
    }
    out
}

fn setting(node: Node<'_>, src: &str, out: &mut Settings) {
    let parts = children(node);
    let strings = || -> Vec<String> {
        parts
            .iter()
            .filter(|n| n.kind() == "string")
            .map(|n| text(*n, src).to_owned())
            .collect()
    };
    // `set shell := [...]` has its own rule, with `shell` a keyword rather
    // than an identifier.
    if node.child(1).is_some_and(|kw| kw.kind() == "shell") {
        out.shell = Some(strings());
        return;
    }
    let Some(name) = parts.iter().find(|n| n.kind() == "identifier") else {
        return;
    };
    let off = parts
        .iter()
        .any(|n| n.kind() == "boolean" && text(*n, src) == "false");
    match text(*name, src) {
        "windows-shell" => out.windows_shell = true,
        "windows-powershell" => out.windows_shell |= !off,
        "positional-arguments" => out.positional = !off,
        _ => {}
    }
}
