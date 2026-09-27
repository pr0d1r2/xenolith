//! The launchd sink (`languages/data/xml:V189`).
//!
//! A launchd job is a property list whose top dict names what to run. With
//! `ProgramArguments` and no `Program` key, argv[0] IS the executable, so
//! an array reading `<sh|bash|zsh|dash> -c <script> …` runs `<script>` as
//! a shell program -- the argv form of `languages/shells/shell:V139`, and
//! the one shape this module turns into a site. Its env is read from ITS
//! argv: the dialect is argv[0]'s basename, and with `-c` alone as
//! argv[1] no option is set.
//!
//! Every layer comes from the tree, and dropping any of them leaves inert
//! data (`languages:V2`): the same array below the top dict, in a
//! document whose root is not `<plist>`, beside a `Program` key, or under
//! any other key. A shape V189 leaves open -- a flag bundle such as
//! `-lc`, a repeated key -- is no site either: a site built on a guess
//! about what launchd runs is worse than none.

use tree_sitter::{Node, Tree};
use xenolith_lang_api::{Delim, DelimKind, GuestEnv, LangId, Site, Span};

use crate::text::{decode, is_text};

#[cfg(test)]
mod tests;

/// The shells whose `-c` argument is a script (`languages/data/xml:V189`),
/// by argv[0]'s basename.
const SHELLS: &[&str] = &["bash", "dash", "sh", "zsh"];

/// The sink a site reports: the plist key that holds it.
const SINK: &str = "ProgramArguments";

fn span(node: Node<'_>) -> Span {
    Span::new(node.start_byte(), node.end_byte())
}

/// The tag of an element: its start tag or its empty-element tag.
fn tag(element: Node<'_>) -> Option<Node<'_>> {
    element
        .child(0)
        .filter(|tag| matches!(tag.kind(), "STag" | "EmptyElemTag"))
}

/// The name of an element, as written.
fn name<'s>(element: Node<'_>, src: &'s str) -> Option<&'s str> {
    let tag = tag(element)?;
    let mut cursor = tag.walk();
    let name = tag
        .named_children(&mut cursor)
        .find(|child| child.kind() == "Name")?;
    src.get(name.byte_range())
}

/// The `content` node of an element, absent when it has none.
fn content(element: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = element.walk();
    element
        .named_children(&mut cursor)
        .find(|child| child.kind() == "content")
}

/// The child elements of `element`, in order. Comments and whitespace
/// between them are not elements and are skipped.
fn children(element: Node<'_>) -> Vec<Node<'_>> {
    let Some(content) = content(element) else {
        return Vec::new();
    };
    let mut cursor = content.walk();
    content
        .named_children(&mut cursor)
        .filter(|child| child.kind() == "element")
        .collect()
}

/// The decoded text of an element holding text only; `None` for markup
/// or an entity with no value. `<x/>` and `<x></x>` both read as empty.
fn string_value(element: Node<'_>, src: &str) -> Option<String> {
    match content(element) {
        None => Some(String::new()),
        Some(content) => decode(content, src).ok(),
    }
}

/// The top dict: the only element of a document whose root is `<plist>`.
fn top_dict<'t>(root: Node<'t>, src: &str) -> Option<Node<'t>> {
    let mut cursor = root.walk();
    let plist = root
        .named_children(&mut cursor)
        .find(|child| child.kind() == "element")
        .filter(|element| name(*element, src) == Some("plist"))?;
    match children(plist).as_slice() {
        [dict] if name(*dict, src) == Some("dict") => Some(*dict),
        _ => None,
    }
}

/// The dict's entries as `(key, value)`, or `None` when it is not a
/// strict alternation of `<key>` and value.
fn entries<'t>(dict: Node<'t>, src: &str) -> Option<Vec<(String, Node<'t>)>> {
    let elements = children(dict);
    let mut out = Vec::new();
    for pair in elements.chunks(2) {
        let [key, value] = pair else {
            return None;
        };
        if name(*key, src) != Some("key") {
            return None;
        }
        out.push((string_value(*key, src)?, *value));
    }
    Some(out)
}

/// The script element of a `ProgramArguments` array, and argv[0]'s
/// basename, when the array reads `<shell> -c <script> …`.
fn script<'t>(array: Node<'t>, src: &str) -> Option<(Node<'t>, String)> {
    let argv = children(array);
    if argv.iter().any(|arg| name(*arg, src) != Some("string")) {
        return None;
    }
    let [program, flag, script, ..] = argv.as_slice() else {
        return None;
    };
    let program = string_value(*program, src)?;
    let shell = program.rsplit('/').next().unwrap_or(&program);
    if !SHELLS.contains(&shell) || string_value(*flag, src)? != "-c" {
        return None;
    }
    Some((*script, shell.to_owned()))
}

/// The site of a script element: its start tag, its text and its end
/// tag. `<string/>` has no text to report, and markup inside is no argv
/// word.
fn site(script: Node<'_>, dialect: String) -> Option<Site> {
    let open = script.child(0).filter(|tag| tag.kind() == "STag")?;
    let close = script.child(script.child_count().checked_sub(1)?)?;
    if close.kind() != "ETag" || !content(script).is_none_or(is_text) {
        return None;
    }
    Some(Site {
        sink: SINK.to_owned(),
        guest: LangId::Shell,
        env: GuestEnv {
            dialect: Some(dialect),
            options: Vec::new(),
        },
        delim: Delim {
            kind: DelimKind::ArgvString,
            open: span(open),
            body: Span::new(open.end_byte(), close.start_byte()),
            close: span(close),
        },
        holes: Vec::new(),
    })
}

/// The launchd site of `tree`, if any: at most one, the top dict's
/// `ProgramArguments` script (`languages/data/xml:V189`).
pub(crate) fn sites(tree: &Tree, src: &str) -> Vec<Site> {
    let found = || {
        let entries = entries(top_dict(tree.root_node(), src)?, src)?;
        if entries.iter().any(|(key, _)| key == "Program") {
            return None;
        }
        let arrays: Vec<Node<'_>> = entries
            .iter()
            .filter(|(key, _)| key == SINK)
            .map(|(_, value)| *value)
            .collect();
        let [array] = arrays.as_slice() else {
            return None;
        };
        if name(*array, src) != Some("array") {
            return None;
        }
        let (script, dialect) = script(*array, src)?;
        site(script, dialect)
    };
    found().into_iter().collect()
}
