//! Element text, both directions: the mirror of `src/text.rs`
//! (`src:C139`).
//!
//! What is pinned: [`unescape`] decodes what an XML processor decodes
//! -- the five predefined entities, decimal and hex character
//! references, CDATA verbatim, line breaks folded to `\n` -- and refuses
//! what it cannot read rather than passing bytes through
//! (`languages/api/src/lens:V39`); [`escape`] is its inverse.

use xenolith_lang_api::{Delim, DelimKind, Error, LangId, Span};

use super::{escape, unescape, xml_char};

fn argv() -> Delim {
    Delim {
        kind: DelimKind::ArgvString,
        open: Span::new(0, 0),
        body: Span::new(0, 0),
        close: Span::new(0, 0),
    }
}

fn read(raw: &str) -> String {
    unescape(&argv(), raw).unwrap_or_else(|e| panic!("unescape {raw:?}: {e}"))
}

fn is_parse_error(result: &Result<String, Error>) -> bool {
    matches!(
        result,
        Err(Error::Parse {
            lang: LangId::Xml,
            ..
        })
    )
}

// --- unescape ----------------------------------------------------------

#[test]
fn plain_text_reads_as_itself() {
    assert_eq!(read("echo hi"), "echo hi");
    assert_eq!(read(""), "");
    assert_eq!(read("  indented\n\ttabbed  "), "  indented\n\ttabbed  ");
}

#[test]
fn the_five_predefined_entities_are_decoded() {
    assert_eq!(read("a &amp;&amp; b"), "a && b");
    assert_eq!(read("&lt;in &gt;out"), "<in >out");
    assert_eq!(read("&quot;x&quot; &apos;y&apos;"), "\"x\" 'y'");
}

#[test]
fn character_references_are_decoded_in_both_bases() {
    assert_eq!(read("&#65;&#x42;&#x4a;"), "ABJ");
    // `&#X...;` is not XML: the hex marker is a lowercase `x`.
    assert!(is_parse_error(&unescape(&argv(), "&#X43;")));
    assert_eq!(read("&#x17C;"), "ż");
    // A referenced carriage return survives: only a LITERAL one is a
    // line break the processor folds.
    assert_eq!(read("a&#13;b"), "a\rb");
    assert_eq!(read("a&#xD;&#xA;b"), "a\r\nb");
}

#[test]
fn cdata_is_verbatim() {
    assert_eq!(read("<![CDATA[a && b < c]]>"), "a && b < c");
    assert_eq!(read("<![CDATA[&amp;]]>"), "&amp;");
    assert_eq!(read("x<![CDATA[]]>y"), "xy");
    assert_eq!(read("<![CDATA[a]]>&amp;<![CDATA[b]]>"), "a&b");
}

#[test]
fn literal_line_breaks_fold_to_line_feed() {
    assert_eq!(read("a\r\nb"), "a\nb");
    assert_eq!(read("a\rb"), "a\nb");
    assert_eq!(read("a\r\n\r\nb\n"), "a\n\nb\n");
    assert_eq!(read("<![CDATA[a\r\nb]]>"), "a\nb");
}

#[test]
fn a_greater_than_sign_needs_no_escape_in_text() {
    assert_eq!(read("a > b"), "a > b");
}

#[test]
fn an_entity_the_processor_cannot_resolve_is_refused() {
    // Defined only by a DTD this crate never reads, so its value is
    // unknown: reading `&nbsp;` as text would hand the guest bytes no
    // processor produces.
    assert!(is_parse_error(&unescape(&argv(), "a&nbsp;b")));
}

#[test]
fn a_reference_to_a_character_xml_forbids_is_refused() {
    assert!(is_parse_error(&unescape(&argv(), "&#1;")));
    assert!(is_parse_error(&unescape(&argv(), "&#xFFFE;")));
    assert!(is_parse_error(&unescape(&argv(), "&#xD800;")));
    assert!(is_parse_error(&unescape(&argv(), "&#x110000;")));
}

#[test]
fn markup_inside_the_text_is_refused() {
    assert!(is_parse_error(&unescape(&argv(), "a<b/>c")));
    assert!(is_parse_error(&unescape(&argv(), "a<!-- c -->b")));
    assert!(is_parse_error(&unescape(&argv(), "a<?pi x?>b")));
}

#[test]
fn text_that_is_not_well_formed_is_refused() {
    assert!(is_parse_error(&unescape(&argv(), "a < b")));
    assert!(is_parse_error(&unescape(&argv(), "a & b")));
    assert!(is_parse_error(&unescape(&argv(), "a</x><x>b")));
}

#[test]
fn a_delimiter_this_host_never_writes_is_unsupported() {
    let nix = Delim {
        kind: DelimKind::NixIndented,
        ..argv()
    };
    assert!(matches!(
        unescape(&nix, "x"),
        Err(Error::Unsupported {
            lang: LangId::Xml,
            ..
        })
    ));
    assert!(matches!(
        escape(&nix, "x"),
        Err(Error::Unsupported {
            lang: LangId::Xml,
            ..
        })
    ));
}

// --- escape ------------------------------------------------------------

#[test]
fn escape_writes_the_three_markup_characters_as_entities() {
    assert_eq!(
        escape(&argv(), "a && b < c > d").as_deref(),
        Ok("a &amp;&amp; b &lt; c &gt; d")
    );
    assert_eq!(escape(&argv(), "\"q\" 'a'").as_deref(), Ok("\"q\" 'a'"));
}

#[test]
fn escape_writes_a_carriage_return_as_a_reference() {
    // A literal one would fold to `\n` on the way back.
    assert_eq!(escape(&argv(), "a\rb\r\n").as_deref(), Ok("a&#13;b&#13;\n"));
}

#[test]
fn escape_refuses_a_character_xml_cannot_hold() {
    let result = escape(&argv(), "bell \u{7}");
    assert!(is_parse_error(&result), "{result:?}");
}

#[test]
fn escape_then_unescape_is_the_identity() {
    for body in [
        "",
        "echo hi\n",
        "cd /tmp && ls | wc -l",
        "a < b > c & d; ]]> e",
        "tab\there\r\nand\rcr",
        "&amp; already escaped",
        "<![CDATA[x]]>",
        "zażółć",
    ] {
        let written = escape(&argv(), body).unwrap_or_else(|e| panic!("escape {body:?}: {e}"));
        assert_eq!(read(&written), body, "{written:?}");
    }
}

#[test]
fn xml_chars_are_those_of_xml_1_0() {
    for ok in [
        '\t',
        '\n',
        '\r',
        ' ',
        'a',
        '\u{D7FF}',
        '\u{E000}',
        '\u{FFFD}',
        '\u{10000}',
    ] {
        assert!(xml_char(ok), "{ok:?}");
    }
    for bad in ['\0', '\u{7}', '\u{1F}', '\u{FFFE}', '\u{FFFF}'] {
        assert!(!xml_char(bad), "{bad:?}");
    }
}
