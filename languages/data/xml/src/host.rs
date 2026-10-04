//! XML as a host (`languages/data/xml:T192`).
//!
//! The host claims `*.xml`, and a `*.plist` only when it is text XML
//! (`languages/data/xml:V188`): a binary plist starts `bplist00`, is not
//! text at all, and is never offered to a parser that would report it as
//! broken XML (`src/check:V13`). Other XML extensions -- `.svg`, `.xsd`,
//! `.xhtml` -- wait for a sink that needs them.
//!
//! Its one sink is the launchd job ([`crate::launchd`]). The body the
//! shell receives is the element's DECODED text ([`crate::text`]), and
//! the file itself is checked with `xmllint --noout`, the command the
//! fleet already runs (`languages/data/xml:R186`).
//!
//! The extract direction is refused, and with it `loads` and `inline`:
//! where a load may point is `languages/data/xml:T190`, still open.
//! launchd runs a job from `/` unless its plist sets `WorkingDirectory`,
//! so the host-relative path every other host writes
//! (`languages/api/src/lens:V66`) would name the wrong file. The engine
//! turns the refusal into a `Judgment` direction (`src/check:B12`).

use std::path::Path;

use tree_sitter::{Node, Parser, Tree};
use xenolith_lang_api::{
    Delim, Error, FileArg, Format, Host, Invoke, LangId, LintCmd, LoadRef, Result, Site,
};

use crate::{launchd, text};

#[cfg(test)]
mod tests;

/// What a text plist's first line starts with (`languages/data/xml:V188`).
const PLIST_HEADS: &[&str] = &["<?xml", "<!DOCTYPE plist", "<plist"];

/// The xml host.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct XmlHost;

/// `src` parsed as an XML document.
pub(crate) fn parse(src: &str) -> Result<Tree> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_xml::LANGUAGE_XML.into())
        .map_err(|e| Error::parse(LangId::Xml, e.to_string()))?;
    parser
        .parse(src, None)
        .ok_or_else(|| Error::parse(LangId::Xml, "the parser returned no tree"))
}

/// The source text of `node`.
pub(crate) fn text<'s>(node: Node<'_>, src: &'s str) -> &'s str {
    src.get(node.byte_range()).unwrap_or_default()
}

fn refused(operation: &'static str) -> Error {
    Error::unsupported(LangId::Xml, operation)
}

impl Host for XmlHost {
    fn id(&self) -> LangId {
        LangId::Xml
    }

    /// `*.xml`; `*.plist` when its first line is text XML, past a
    /// byte-order mark and leading blanks (`languages/data/xml:V188`).
    fn claims(&self, path: &Path, head: &str) -> bool {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("xml") => true,
            Some("plist") => {
                let head = head.trim_start_matches('\u{feff}').trim_start();
                PLIST_HEADS.iter().any(|start| head.starts_with(start))
            }
            _ => false,
        }
    }

    /// The launchd site, if any (`languages/data/xml:V189`).
    ///
    /// ANY parse error fails the whole file, as the shell host does:
    /// spans inside and after an `ERROR` node are the parser's guess
    /// (`languages:V78`), and `xmllint` names the error precisely.
    fn sites(&self, src: &str) -> Result<Vec<Site>> {
        let tree = parse(src)?;
        if tree.root_node().has_error() {
            return Err(Error::parse(
                LangId::Xml,
                "the file is not well-formed XML, so none of its sites were read",
            ));
        }
        Ok(launchd::sites(&tree, src))
    }

    /// Not offered: there is no load idiom to read back until
    /// `languages/data/xml:T190` decides one, and an empty list would read
    /// as "nothing loaded here" (`languages/api:V37`).
    fn loads(&self, _src: &str) -> Result<Vec<LoadRef>> {
        Err(refused("loads"))
    }

    /// Refused while `languages/data/xml:T190` is open: a load relative to
    /// the plist runs from `/` under launchd, so the extract direction is
    /// a `Judgment`, never a mechanical rewrite.
    fn rewrite(&self, _src: &str, _site: &Site, _invoke: &Invoke, _path: &Path) -> Result<String> {
        Err(refused("rewrite"))
    }

    /// Refused with [`XmlHost::rewrite`]: there is no load to inline.
    fn inline(&self, _src: &str, _load: &LoadRef, _body: &str) -> Result<String> {
        Err(refused("inline"))
    }

    /// [`text::unescape`]: entities and references decoded, CDATA
    /// verbatim (`languages/api/src/lens:V39`).
    fn unescape(&self, delim: &Delim, raw: &str) -> Result<String> {
        text::unescape(delim, raw)
    }

    /// [`text::escape`]: the inverse.
    fn escape(&self, delim: &Delim, body: &str) -> Result<String> {
        text::escape(delim, body)
    }

    /// `xmllint --noout FILE`: well-formedness, the fleet hook's check
    /// (`languages/data/xml:R186`). `plutil -lint` is macOS only
    /// (`.:C3`).
    fn checks(&self) -> Vec<LintCmd> {
        vec![LintCmd {
            argv: vec!["xmllint".to_owned(), "--noout".to_owned()],
            file_arg: FileArg::Append,
            format: Format::Raw,
        }]
    }

    /// None, as a statement rather than a gap: `xmllint` reports and
    /// never rewrites, and a reformatter nobody asked for would churn
    /// every plist it touched.
    fn fixers(&self) -> Vec<LintCmd> {
        Vec::new()
    }
}
