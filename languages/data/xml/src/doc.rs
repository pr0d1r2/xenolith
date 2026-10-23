//! An XML document as elements with exact spans, read with quick-xml.
//!
//! quick-xml is a streaming reader: it hands over events (a start tag, a
//! run of text, an entity reference, a CDATA section) with their byte
//! positions and their RAW bytes, decoding nothing. That is the split
//! this crate wants -- [`crate::text`] decodes element text itself, by
//! XML's rules -- so this module only turns the events into a tree:
//! each element's name, start tag, end tag and content span, its child
//! elements, and its content as [`Piece`]s.
//!
//! A document that is not well-formed is [`Error::Parse`] and none of its
//! sites are read (`languages:V78`): an unclosed, mismatched or stray tag,
//! a second root or text outside the root, a repeated or unquoted
//! attribute, an entity reference that names nothing XML can name, or
//! an unclosed CDATA section.

use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use xenolith_lang_api::{Error, LangId, Result, Span};

#[cfg(test)]
mod tests;

/// A parsed document.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Document {
    /// The root element; `None` for a document with no element at all.
    pub root: Option<Element>,
}

/// One element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Element {
    /// Its name, as written.
    pub name: String,
    /// `<name ...>`, or the whole `<name .../>`.
    pub start_tag: Span,
    /// `</name>`; `None` for an empty-element tag.
    pub end_tag: Option<Span>,
    /// Between the tags; empty for an empty-element tag.
    pub content: Span,
    /// Child elements, in order.
    pub children: Vec<Element>,
    /// The content, in order.
    pub pieces: Vec<Piece>,
}

/// A piece of an element's content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Piece {
    /// Character data, as written.
    Text(Span),
    /// `&name;`, `&#N;` or `&#xN;`, as written.
    Ref(Span),
    /// The data of a CDATA section, between `<![CDATA[` and `]]>`.
    CData(Span),
    /// Anything that is not text: an `"element"`, a `"comment"` or a
    /// `"processing instruction"`.
    Markup(&'static str),
}

fn refuse(src: &str, at: u64, message: impl std::fmt::Display) -> Error {
    let at = usize::try_from(at).unwrap_or(src.len()).min(src.len());
    let before = src.get(..at).unwrap_or_default();
    let line = before.matches('\n').count() + 1;
    let col = before.len() - before.rfind('\n').map_or(0, |nl| nl + 1) + 1;
    Error::parse(LangId::Xml, format!("{line}:{col}: {message}"))
}

/// Whether `name` is something `&name;` may name: an XML name, `#N` or
/// `#xN`.
fn reference_name(name: &str) -> bool {
    if let Some(number) = name.strip_prefix('#') {
        return match number.strip_prefix('x') {
            Some(hex) => !hex.is_empty() && hex.chars().all(|c| c.is_ascii_hexdigit()),
            None => !number.is_empty() && number.chars().all(|c| c.is_ascii_digit()),
        };
    }
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_alphabetic() || c == '_' || c == ':')
        && chars.all(|c| c.is_alphanumeric() || matches!(c, '_' | ':' | '.' | '-'))
}

/// `src` read into a tree.
///
/// # Errors
///
/// [`Error::Parse`] when the document is not well-formed, naming where.
pub(crate) fn parse(src: &str) -> Result<Document> {
    let mut reader = Reader::from_str(src);
    let mut tree = Tree {
        src,
        stack: Vec::new(),
        root: None,
    };
    loop {
        let before = reader.buffer_position();
        let event = reader
            .read_event()
            .map_err(|e| refuse(src, reader.error_position(), e))?;
        let after = reader.buffer_position();
        let span = Span::new(position(before), position(after));
        match event {
            Event::Start(tag) => tree.open(&tag, span, before)?,
            Event::Empty(tag) => tree.empty(&tag, span, before)?,
            Event::End(_) => tree.close(span, before)?,
            Event::Text(_) => {
                if tree.stack.is_empty() {
                    let written = span.of(src).unwrap_or_default();
                    if !written.chars().all(char::is_whitespace) {
                        return Err(refuse(src, before, "text outside the root element"));
                    }
                } else {
                    tree.piece(Piece::Text(span));
                }
            }
            Event::GeneralRef(_) => {
                let written = span.of(src).unwrap_or_default();
                let name = written
                    .strip_prefix('&')
                    .and_then(|rest| rest.strip_suffix(';'))
                    .unwrap_or_default();
                if tree.stack.is_empty() || !reference_name(name) {
                    return Err(refuse(src, before, format!("bad reference `&{name};`")));
                }
                tree.piece(Piece::Ref(span));
            }
            Event::CData(_) => {
                if tree.stack.is_empty() {
                    return Err(refuse(src, before, "CDATA outside the root element"));
                }
                let data = Span::new(span.start + "<![CDATA[".len(), span.end - "]]>".len());
                tree.piece(Piece::CData(data));
            }
            Event::Comment(_) => tree.piece(Piece::Markup("comment")),
            Event::PI(_) => tree.piece(Piece::Markup("processing instruction")),
            Event::Decl(_) | Event::DocType(_) => {}
            Event::Eof => break,
        }
    }
    if !tree.stack.is_empty() {
        return Err(refuse(
            src,
            reader.buffer_position(),
            "an element is never closed",
        ));
    }
    Ok(Document { root: tree.root })
}

fn position(at: u64) -> usize {
    usize::try_from(at).unwrap_or(usize::MAX)
}

/// The tree being built: open elements, and the closed root.
struct Tree<'s> {
    src: &'s str,
    stack: Vec<Element>,
    root: Option<Element>,
}

impl Tree<'_> {
    /// `piece` into the innermost open element; outside every element a
    /// comment or a processing instruction is allowed and dropped.
    fn piece(&mut self, piece: Piece) {
        if let Some(open) = self.stack.last_mut() {
            open.pieces.push(piece);
        }
    }

    /// A new element from `tag`, its attributes checked.
    fn element(&mut self, tag: &BytesStart<'_>, span: Span, at: u64) -> Result<Element> {
        if self.stack.is_empty() && self.root.is_some() {
            return Err(refuse(self.src, at, "a second root element"));
        }
        for attribute in tag.attributes() {
            attribute.map_err(|e| refuse(self.src, at, e))?;
        }
        self.piece(Piece::Markup("element"));
        Ok(Element {
            name: tag.name().as_ref().to_owned(),
            start_tag: span,
            end_tag: None,
            content: Span::new(span.end, span.end),
            children: Vec::new(),
            pieces: Vec::new(),
        })
    }

    fn open(&mut self, tag: &BytesStart<'_>, span: Span, at: u64) -> Result<()> {
        let element = self.element(tag, span, at)?;
        self.stack.push(element);
        Ok(())
    }

    fn empty(&mut self, tag: &BytesStart<'_>, span: Span, at: u64) -> Result<()> {
        let element = self.element(tag, span, at)?;
        self.attach(element);
        Ok(())
    }

    /// The end tag at `span` closes the innermost open element (quick-xml
    /// has already checked the names match).
    fn close(&mut self, span: Span, at: u64) -> Result<()> {
        let mut element = self
            .stack
            .pop()
            .ok_or_else(|| refuse(self.src, at, "an end tag with no start tag"))?;
        element.end_tag = Some(span);
        element.content = Span::new(element.start_tag.end, span.start);
        self.attach(element);
        Ok(())
    }

    /// A closed element into its parent, or as the root.
    fn attach(&mut self, element: Element) {
        match self.stack.last_mut() {
            Some(parent) => parent.children.push(element),
            None => self.root = Some(element),
        }
    }
}
