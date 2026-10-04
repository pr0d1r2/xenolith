//! Element text, both directions (`languages/data/xml` §I).
//!
//! [`unescape`] turns the raw content of a `<string>` element into the
//! text an XML processor hands the program -- launchd's argv word -- and
//! [`escape`] is its inverse (`languages/api/src/lens:V39`). XML's rules,
//! which is all this module encodes:
//!
//! - the five predefined entities `&lt;` `&gt;` `&amp;` `&quot;`
//!   `&apos;`, and character references `&#N;` / `&#xN;`, each naming a
//!   character XML 1.0 allows; any other entity is declared by a DTD this
//!   crate never reads, so its value is unknown and the text is refused;
//! - a CDATA section is verbatim;
//! - a literal `\r\n` or lone `\r` is one `\n` (XML 1.0 §2.11), while a
//!   REFERENCED `&#13;` stays a carriage return -- which is why [`escape`]
//!   writes one that way;
//! - text holds no markup: an element, comment or processing instruction
//!   inside a `<string>` is not an argv word.
//!
//! References and CDATA are found by the GRAMMAR, never by scanning for
//! `&` (`languages/api/src/site:V38`): the raw text is re-parsed inside a
//! one-element document, and the tree says which bytes are which.

use tree_sitter::Node;
use xenolith_lang_api::{Delim, DelimKind, Error, LangId, Result};

use crate::host::{parse, text};

#[cfg(test)]
mod tests;

/// The element wrapping raw text for the re-parse.
const OPEN: &str = "<x>";
const CLOSE: &str = "</x>";

/// Whether `ch` may appear in an XML 1.0 document (the `Char`
/// production). Surrogates are no `char` at all.
pub(crate) fn xml_char(ch: char) -> bool {
    matches!(ch, '\t' | '\n' | '\r' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}')
        || ch >= '\u{10000}'
}

fn refuse(message: impl Into<String>) -> Error {
    Error::parse(LangId::Xml, message)
}

fn argv_only(delim: &Delim, operation: &'static str) -> Result<()> {
    match delim.kind {
        DelimKind::ArgvString => Ok(()),
        _ => Err(Error::unsupported(LangId::Xml, operation)),
    }
}

/// `\r\n` and a lone `\r` as `\n`, as an XML processor reads literal
/// text.
fn fold_breaks(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// The value of a predefined entity.
fn entity(name: &str) -> Option<char> {
    match name {
        "lt" => Some('<'),
        "gt" => Some('>'),
        "amp" => Some('&'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        _ => None,
    }
}

/// The character `&#N;` or `&#xN;` names, when XML allows it.
fn reference(written: &str) -> Option<char> {
    let inner = written.strip_prefix("&#")?.strip_suffix(';')?;
    let code = match inner.strip_prefix('x') {
        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
        None => inner.parse().ok()?,
    };
    char::from_u32(code).filter(|&ch| xml_char(ch))
}

/// The text of one content child, or why it is not text.
fn piece(node: Node<'_>, src: &str) -> Result<String> {
    let written = text(node, src);
    match node.kind() {
        "CharData" => Ok(fold_breaks(written)),
        "EntityRef" => written
            .strip_prefix('&')
            .and_then(|rest| rest.strip_suffix(';'))
            .and_then(entity)
            .map(String::from)
            .ok_or_else(|| refuse(format!("entity {written} has no value outside a DTD"))),
        "CharRef" => reference(written)
            .map(String::from)
            .ok_or_else(|| refuse(format!("{written} names no XML character"))),
        "CDSect" => {
            let mut cursor = node.walk();
            let data = node
                .named_children(&mut cursor)
                .find(|child| child.kind() == "CData")
                .map_or("", |data| text(data, src));
            Ok(fold_breaks(data))
        }
        other => Err(refuse(format!("markup ({other}) inside element text"))),
    }
}

/// The decoded text of an element's `content` node: every child must be
/// text -- character data, a reference or a CDATA section.
pub(crate) fn decode(content: Node<'_>, src: &str) -> Result<String> {
    let mut cursor = content.walk();
    content
        .children(&mut cursor)
        .map(|child| piece(child, src))
        .collect()
}

/// Whether every child of `content` is text: the shape [`decode`] reads,
/// whatever the entities turn out to mean.
pub(crate) fn is_text(content: Node<'_>) -> bool {
    let mut cursor = content.walk();
    content.children(&mut cursor).all(|child| {
        matches!(
            child.kind(),
            "CharData" | "EntityRef" | "CharRef" | "CDSect"
        )
    })
}

/// The text an XML processor reads from `raw`, the content between the
/// delimiters of `delim` (`languages/api/src/lens:V39`).
///
/// # Errors
///
/// [`Error::Unsupported`] for a delimiter other than
/// [`DelimKind::ArgvString`], and [`Error::Parse`] when `raw` is not
/// well-formed element text, holds markup, or names an entity or a
/// character this crate cannot resolve.
pub fn unescape(delim: &Delim, raw: &str) -> Result<String> {
    argv_only(delim, "unescape")?;
    let wrapped = format!("{OPEN}{raw}{CLOSE}");
    let tree = parse(&wrapped)?;
    let root = tree.root_node();
    if root.has_error() {
        return Err(refuse("the text is not well-formed XML"));
    }
    let mut cursor = root.walk();
    let elements: Vec<Node<'_>> = root
        .named_children(&mut cursor)
        .filter(|child| child.kind() == "element")
        .collect();
    let [element] = elements.as_slice() else {
        return Err(refuse("the text closes its element"));
    };
    let (start, end) = (OPEN.len(), OPEN.len() + raw.len());
    let mut cursor = element.walk();
    let content = element
        .named_children(&mut cursor)
        .find(|child| child.kind() == "content");
    match content {
        None if raw.is_empty() => Ok(String::new()),
        Some(content) if content.start_byte() == start && content.end_byte() == end => {
            decode(content, &wrapped)
        }
        _ => Err(refuse("the text closes its element")),
    }
}

/// `body` written as element text that [`unescape`] reads back
/// unchanged: `&`, `<` and `>` as entities, a carriage return as `&#13;`
/// (a literal one would fold to `\n`), everything else as it is.
///
/// # Errors
///
/// [`Error::Unsupported`] for a delimiter other than
/// [`DelimKind::ArgvString`], and [`Error::Parse`] for a character XML
/// 1.0 cannot hold at all, literal or referenced.
pub fn escape(delim: &Delim, body: &str) -> Result<String> {
    argv_only(delim, "escape")?;
    let mut out = String::with_capacity(body.len());
    for ch in body.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\r' => out.push_str("&#13;"),
            ch if xml_char(ch) => out.push(ch),
            ch => {
                return Err(refuse(format!(
                    "U+{:04X} cannot appear in XML 1.0",
                    u32::from(ch)
                )));
            }
        }
    }
    Ok(out)
}
