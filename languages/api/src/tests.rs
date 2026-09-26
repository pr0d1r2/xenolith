//! Unit tests for the crate root (`src:C139`): `LangId`, `Span` and
//! `Error`, the only items here with function bodies.
//!
//! `tests/contract.rs` pins the contract's SHAPE from outside, the way a
//! language crate sees it; these pin each function's own branches and
//! edges, which is where an off-by-one in a span or a near-miss accepted
//! as a language name would hide.

use super::{Error, LangId, Span};

// ---------------------------------------------------------------------
// LangId
// ---------------------------------------------------------------------

#[test]
fn every_name_parses_back_to_its_variant() {
    for &id in LangId::ALL {
        assert_eq!(LangId::from_name(id.as_str()), Some(id), "{id:?}");
    }
}

#[test]
fn names_are_distinct_lowercase_ascii() {
    let mut names: Vec<&str> = LangId::ALL.iter().map(|id| id.as_str()).collect();
    for name in &names {
        assert!(!name.is_empty());
        assert!(
            name.bytes().all(|b| b.is_ascii_lowercase()),
            "{name} must be usable as `lang-{name}` and a config key"
        );
    }
    let count = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), count, "two variants share a name");
}

#[test]
fn from_name_is_exact_match_only() {
    // Near-misses are mistakes worth an error, not a language nobody
    // notices was never scanned.
    for near in ["Nix", "NIX", " nix", "nix ", "bash", "javascript", "sh", ""] {
        assert_eq!(LangId::from_name(near), None, "{near:?}");
    }
}

#[test]
fn display_is_the_name() {
    for &id in LangId::ALL {
        assert_eq!(id.to_string(), id.as_str());
    }
}

#[test]
fn the_declaration_order_is_the_name_order() {
    // The registry and every report iterate in `Ord` (`src:V41`), and a
    // reader scanning a list expects alphabetical.
    let ids: Vec<LangId> = LangId::ALL.to_vec();
    let mut by_ord = ids.clone();
    by_ord.sort();
    assert_eq!(ids, by_ord);
    let names: Vec<&str> = ids.iter().map(|id| id.as_str()).collect();
    let mut by_name = names.clone();
    by_name.sort_unstable();
    assert_eq!(names, by_name);
}

// ---------------------------------------------------------------------
// Span
// ---------------------------------------------------------------------

#[test]
fn new_stores_start_and_end() {
    let span = Span::new(3, 9);
    assert_eq!((span.start, span.end), (3, 9));
}

#[test]
fn len_is_end_minus_start() {
    assert_eq!(Span::new(3, 9).len(), 6);
    assert_eq!(Span::new(0, 0).len(), 0);
}

#[test]
fn len_of_an_inverted_span_saturates_at_zero() {
    assert_eq!(Span::new(9, 3).len(), 0);
    assert_eq!(Span::new(usize::MAX, 0).len(), 0);
}

#[test]
fn is_empty_for_zero_width_and_inverted_spans_only() {
    assert!(Span::new(4, 4).is_empty());
    assert!(Span::new(5, 4).is_empty());
    assert!(!Span::new(4, 5).is_empty());
}

#[test]
fn of_returns_the_covered_text() {
    let src = "let x = ''echo hi'';";
    assert_eq!(Span::new(10, 17).of(src), Some("echo hi"));
    assert_eq!(Span::new(0, src.len()).of(src), Some(src));
}

#[test]
fn of_a_zero_width_span_is_empty_text() {
    assert_eq!(Span::new(2, 2).of("abc"), Some(""));
    assert_eq!(Span::new(3, 3).of("abc"), Some(""));
}

#[test]
fn of_is_none_past_the_end() {
    assert_eq!(Span::new(0, 4).of("abc"), None);
    assert_eq!(Span::new(4, 4).of("abc"), None);
}

#[test]
fn of_is_none_for_an_inverted_span() {
    assert_eq!(Span::new(2, 1).of("abc"), None);
}

#[test]
fn of_is_none_off_a_char_boundary() {
    // `ż` is two bytes; a span ending inside it is a bug in whoever made
    // the span, and a panic would lose the rest of the scan.
    let src = "żółw";
    assert_eq!(Span::new(0, 1).of(src), None);
    assert_eq!(Span::new(1, 2).of(src), None);
    assert_eq!(Span::new(0, 2).of(src), Some("ż"));
}

#[test]
fn spans_order_by_start_then_end() {
    let mut spans = vec![Span::new(5, 6), Span::new(1, 9), Span::new(1, 2)];
    spans.sort();
    assert_eq!(
        spans,
        vec![Span::new(1, 2), Span::new(1, 9), Span::new(5, 6)]
    );
}

// ---------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------

#[test]
fn parse_builds_the_parse_variant() {
    assert_eq!(
        Error::parse(LangId::Nix, "unexpected `}`"),
        Error::Parse {
            lang: LangId::Nix,
            message: "unexpected `}`".to_owned(),
        }
    );
}

#[test]
fn unsupported_builds_the_unsupported_variant() {
    assert_eq!(
        Error::unsupported(LangId::Pkl, "inline"),
        Error::Unsupported {
            lang: LangId::Pkl,
            operation: "inline",
        }
    );
}

#[test]
fn lang_names_the_language_for_both_variants() {
    assert_eq!(Error::parse(LangId::Shell, "x").lang(), LangId::Shell);
    assert_eq!(
        Error::unsupported(LangId::Yaml, "loads").lang(),
        LangId::Yaml
    );
}

#[test]
fn a_parse_error_displays_language_and_message() {
    assert_eq!(
        Error::parse(LangId::Nix, "unexpected `}`").to_string(),
        "nix: parse failed: unexpected `}`"
    );
}

#[test]
fn an_empty_parse_message_still_names_the_failure() {
    assert_eq!(
        Error::parse(LangId::Nix, "").to_string(),
        "nix: parse failed: "
    );
}

#[test]
fn an_unsupported_error_displays_language_and_operation() {
    assert_eq!(
        Error::unsupported(LangId::Pkl, "inline").to_string(),
        "pkl: does not support `inline`"
    );
}

#[test]
fn error_is_a_std_error() {
    let boxed: Box<dyn std::error::Error> = Box::new(Error::unsupported(LangId::Jq, "rewrite"));
    assert_eq!(boxed.to_string(), "jq: does not support `rewrite`");
    assert!(boxed.source().is_none());
}
