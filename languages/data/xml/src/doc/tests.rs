//! Unit tests for `doc.rs` (`src:C139`): an XML document read with
//! quick-xml into elements whose tags, content and pieces carry exact
//! spans, and the documents that are not well-formed.

use xenolith_lang_api::{Error, LangId, Span};

use super::{Element, Piece, parse};

fn root(src: &str) -> Element {
    parse(src)
        .unwrap_or_else(|e| panic!("{src:?}: {e}"))
        .root
        .unwrap_or_else(|| panic!("{src:?}: no root"))
}

/// The `i`th child element of `e`.
fn child(e: &Element, i: usize) -> &Element {
    e.children
        .get(i)
        .unwrap_or_else(|| panic!("no child {i} of {}", e.name))
}

fn of(span: Span, src: &str) -> &str {
    span.of(src).unwrap_or_else(|| panic!("{span:?} outside {src:?}"))
}

const PLIST: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
    "<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" ",
    "\"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n",
    "<plist version=\"1.0\">\n",
    "<dict>\n",
    "  <!-- a comment -->\n",
    "  <key>Label</key>\n",
    "  <string>a &amp; b &#65;<![CDATA[<raw>]]></string>\n",
    "  <key>Empty</key>\n",
    "  <true/>\n",
    "</dict>\n",
    "</plist>\n",
);

#[test]
fn elements_nest_with_their_names_in_order() {
    let plist = root(PLIST);
    assert_eq!(plist.name, "plist");
    let [dict] = plist.children.as_slice() else {
        panic!("{:#?}", plist.children);
    };
    let names: Vec<&str> = dict.children.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["key", "string", "key", "true"]);
}

#[test]
fn tags_and_content_have_exact_spans() {
    let plist = root(PLIST);
    let string = child(child(&plist, 0), 1);
    assert_eq!(of(string.start_tag, PLIST), "<string>");
    assert_eq!(string.end_tag.map(|t| of(t, PLIST)), Some("</string>"));
    assert_eq!(
        of(string.content, PLIST),
        "a &amp; b &#65;<![CDATA[<raw>]]>"
    );
    let empty = child(child(&plist, 0), 3);
    assert_eq!(of(empty.start_tag, PLIST), "<true/>");
    assert_eq!(empty.end_tag, None);
    assert!(empty.pieces.is_empty());
}

#[test]
fn content_is_split_into_text_references_cdata_and_markup() {
    let plist = root(PLIST);
    let string = child(child(&plist, 0), 1);
    let pieces: Vec<String> = string
        .pieces
        .iter()
        .map(|p| match p {
            Piece::Text(s) => format!("text {}", of(*s, PLIST)),
            Piece::Ref(s) => format!("ref {}", of(*s, PLIST)),
            Piece::CData(s) => format!("cdata {}", of(*s, PLIST)),
            Piece::Markup(kind) => format!("markup {kind}"),
        })
        .collect();
    assert_eq!(
        pieces,
        [
            "text a ",
            "ref &amp;",
            "text  b ",
            "ref &#65;",
            "cdata <raw>"
        ]
    );
    let dict = child(&plist, 0);
    assert!(dict.pieces.iter().any(|p| matches!(p, Piece::Markup("comment"))));
    assert!(dict.pieces.iter().any(|p| matches!(p, Piece::Markup("element"))));
}

#[test]
fn an_empty_document_has_no_root() {
    assert!(parse("").is_ok_and(|d| d.root.is_none()));
    assert!(parse("<?xml version=\"1.0\"?>\n<!-- only -->\n").is_ok_and(|d| d.root.is_none()));
}

#[test]
fn what_is_not_well_formed_is_a_parse_error() {
    for src in [
        "<a>",
        "<a></b>",
        "</a>",
        "<a></a><b/>",
        "<a></a>trailing",
        "text<a/>",
        "<a x=\"1\" x=\"2\"/>",
        "<a x=1/>",
        "<a>&#;</a>",
        "<a><![CDATA[open</a>",
    ] {
        match parse(src) {
            Err(Error::Parse { lang, .. }) => assert_eq!(lang, LangId::Xml),
            other => panic!("{src:?}: expected a parse error, got {other:?}"),
        }
    }
}
