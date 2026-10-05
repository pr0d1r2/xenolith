//! Shell's side of holes as params (`languages/api/src/holes:V40`): the
//! names a body already uses, where a hole's marker may become a
//! reference to its env param, and finding those references again.
//!
//! Every answer comes from the tree-sitter-bash AST, never from quote
//! counting: whether `XNL_HOLE_0_` sits in `'...'`, a heredoc or a plain
//! word is exactly what the grammar already resolved
//! (`languages/api/src/site:V38`).
//!
//! The expanding contexts are an ALLOWLIST, conservative on purpose: a
//! marker must be a plain word -- a command, an argument, an assignment
//! value, a redirect target -- or the text of a `"..."` string. Anywhere
//! else a reference is either read literally (`'...'`, a heredoc, a
//! comment) or means something else once quoted (`$((...))`, a `case`
//! pattern, `[[ ]]`), and the hole stays a judgement.

use tree_sitter::{Node, Parser, Tree};
use xenolith_lang_api::holes::{Bound, Param};
use xenolith_lang_api::{Error, LangId, Result, Span};

#[cfg(test)]
mod tests;

/// Ancestors under which no marker becomes a reference: literal text,
/// arithmetic, patterns, and `${...}` operands.
const LITERAL: &[&str] = &[
    "raw_string",
    "ansi_c_string",
    "translated_string",
    "heredoc_body",
    "heredoc_redirect",
    "comment",
    "arithmetic_expansion",
    "expansion",
    "test_command",
    "regex",
    "extglob_pattern",
    "brace_expression",
];

/// Parents whose word children are NAMES or PATTERNS, not values: a
/// function name, `export NAME`, `unset NAME`, a `case` pattern.
const NAMING: &[&str] = &[
    "function_definition",
    "declaration_command",
    "unset_command",
    "case_item",
];

/// Node kinds that are a word's worth of text, as opposed to a statement.
const WORDISH: &[&str] = &[
    "word",
    "concatenation",
    "string",
    "raw_string",
    "simple_expansion",
    "expansion",
    "number",
    "extglob_pattern",
];

/// How a marker's context wants its reference written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Form {
    /// A plain word: `"$NAME"`.
    Word,
    /// Inside `"..."`: `${NAME}`.
    Quoted,
}

/// Parse `body` as bash, refusing any tree with an error in it.
fn parse(body: &str) -> Result<Tree> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_bash::LANGUAGE.into())
        .map_err(|e| Error::parse(LangId::Shell, format!("cannot load the bash grammar: {e}")))?;
    let tree = parser
        .parse(body, None)
        .ok_or_else(|| Error::parse(LangId::Shell, "the bash parser returned no tree"))?;
    if tree.root_node().has_error() {
        return Err(Error::parse(
            LangId::Shell,
            "this body is not valid shell, so its holes cannot become params",
        ));
    }
    Ok(tree)
}

/// Every node under `node`, depth first, in source order.
fn walk<'t>(node: Node<'t>, visit: &mut impl FnMut(Node<'t>) -> bool) {
    if !visit(node) {
        return;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        walk(child, visit);
    }
}

/// The text a node covers.
fn text<'s>(node: Node<'_>, src: &'s str) -> &'s str {
    src.get(node.byte_range()).unwrap_or_default()
}

/// Whether `word` could be a variable name.
fn identifier(word: &str) -> bool {
    word.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && word.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Every variable name `body` reads or assigns, sorted and deduplicated
/// (`languages/api/src/holes:V40`): the grammar's variable names, plus
/// the operands of `read`, which assigns names the grammar calls words.
pub(crate) fn vars(body: &str) -> Result<Vec<String>> {
    let tree = parse(body)?;
    let mut names: Vec<String> = Vec::new();
    walk(tree.root_node(), &mut |node| {
        match node.kind() {
            // `$1` is a variable_name too, and no param can be named `1`.
            "variable_name" if identifier(text(node, body)) => {
                names.push(text(node, body).to_owned());
            }
            "command" if read_command(node, body) => {
                let mut cursor = node.walk();
                names.extend(
                    node.children_by_field_name("argument", &mut cursor)
                        .map(|arg| text(arg, body))
                        .filter(|word| identifier(word))
                        .map(str::to_owned),
                );
            }
            _ => {}
        }
        true
    });
    names.sort_unstable();
    names.dedup();
    Ok(names)
}

/// Whether `command` runs `read`.
fn read_command(command: Node<'_>, body: &str) -> bool {
    command
        .child_by_field_name("name")
        .is_some_and(|name| text(name, body) == "read")
}

/// `body` with every param's marker replaced by its reference, or the
/// first param whose marker sits where the shell would not expand one
/// (`languages/api/src/holes:V40`).
pub(crate) fn params(body: &str, params: &[Param]) -> Result<Bound> {
    let tree = parse(body)?;
    let root = tree.root_node();
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    for param in params {
        for (start, found) in body.match_indices(param.marker.as_str()) {
            let end = start + found.len();
            let reference = match form(root, body, start, end) {
                Some(Form::Word) => format!("\"${}\"", param.name),
                Some(Form::Quoted) => format!("${{{}}}", param.name),
                None => return Ok(Bound::Unexpanded(param.name.clone())),
            };
            edits.push((start, end, reference));
        }
    }
    edits.sort_unstable();
    let mut out = String::with_capacity(body.len());
    let mut at = 0;
    for (start, end, reference) in edits {
        out.push_str(body.get(at..start).unwrap_or_default());
        out.push_str(&reference);
        at = end;
    }
    out.push_str(body.get(at..).unwrap_or_default());
    Ok(Bound::Body(out))
}

/// The form a reference takes at `[start, end)`, or `None` when the shell
/// does not expand one there.
fn form(root: Node<'_>, body: &str, start: usize, end: usize) -> Option<Form> {
    let node = root.named_descendant_for_byte_range(start, end)?;
    if node.start_byte() > start || node.end_byte() < end {
        return None;
    }
    let form = match node.kind() {
        "word" => Form::Word,
        "string_content" if node.parent().is_some_and(|p| plain_string(p, body)) => Form::Quoted,
        _ => return None,
    };
    let mut child = node;
    while let Some(parent) = child.parent() {
        let kind = parent.kind();
        if LITERAL.contains(&kind) || (NAMING.contains(&kind) && WORDISH.contains(&child.kind())) {
            return None;
        }
        child = parent;
    }
    Some(form)
}

/// Whether `node` is a `"..."` string and not bash's `$"..."`, whose text is
/// a message-catalog key: the grammar reads the `$` as a word of its own,
/// so the byte before the quote tells them apart.
fn plain_string(node: Node<'_>, body: &str) -> bool {
    let before = node
        .start_byte()
        .checked_sub(1)
        .and_then(|i| body.as_bytes().get(i));
    node.kind() == "string" && before != Some(&b'$')
}

/// The references to any of `names` in `body`, sorted by span, each
/// covering what [`params`] wrote: `"$NAME"` whole, `${NAME}` and a bare
/// `$NAME` as the expansion. `${NAME:-x}` and the like are not
/// references `params` writes, so they are not found.
pub(crate) fn param_refs(body: &str, names: &[String]) -> Result<Vec<(Span, String)>> {
    let tree = parse(body)?;
    let wanted = |name: &str| names.iter().any(|n| n == name);
    let mut refs: Vec<(Span, String)> = Vec::new();
    walk(tree.root_node(), &mut |node| {
        let span = Span::new(node.start_byte(), node.end_byte());
        match node.kind() {
            "string" => {
                let inner = text(node, body)
                    .strip_prefix("\"$")
                    .and_then(|rest| rest.strip_suffix('"'))
                    .filter(|name| identifier(name) && wanted(name));
                if let Some(name) = inner {
                    refs.push((span, name.to_owned()));
                    return false;
                }
                true
            }
            "simple_expansion" => {
                let name = text(node, body).trim_start_matches('$');
                if wanted(name) {
                    refs.push((span, name.to_owned()));
                }
                false
            }
            "expansion" => {
                let name = text(node, body)
                    .strip_prefix("${")
                    .and_then(|rest| rest.strip_suffix('}'))
                    .filter(|name| wanted(name));
                if let Some(name) = name {
                    refs.push((span, name.to_owned()));
                }
                false
            }
            _ => true,
        }
    });
    refs.sort_unstable();
    Ok(refs)
}
