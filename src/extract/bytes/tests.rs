use super::{equivalent, words};

#[test]
fn normalizes_line_endings_and_final_newline() {
    assert!(equivalent(b"one\r\ntwo\r\n", b"one two\n"));
}

#[test]
fn ignores_trailing_newline_variants() {
    assert!(equivalent(b"one two", b"one two\n\r\n"));
}

#[test]
fn ignores_a_utf8_bom() {
    assert!(equivalent("\u{feff}one two".as_bytes(), b"one two\n"));
}

#[test]
fn ignores_a_bom_before_crlf_text() {
    assert!(equivalent(b"\xef\xbb\xbfone\r\ntwo\r\n", b"one two"));
}

#[test]
fn compares_invalid_bytes_deterministically() {
    assert!(equivalent(b"one\xff\ntwo", b"one\xef\xbf\xbd two"));
    assert_ne!(words(b"one\xff"), words(b"two\xfe"));
}

#[test]
fn malformed_bytes_do_not_make_comparison_panic() {
    assert!(equivalent(b"\xff\xfe one", b"\xef\xbf\xbd\xef\xbf\xbd one"));
}
