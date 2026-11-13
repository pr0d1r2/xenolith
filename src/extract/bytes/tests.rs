use super::{equivalent, words};

#[test]
fn normalizes_line_endings_and_final_newline() {
    assert!(equivalent(b"one\r\ntwo\r\n", b"one two\n"));
}

#[test]
fn ignores_a_utf8_bom() {
    assert!(equivalent("\u{feff}one two".as_bytes(), b"one two\n"));
}

#[test]
fn compares_invalid_bytes_deterministically() {
    assert!(equivalent(b"one\xff\ntwo", b"one\xef\xbf\xbd two"));
    assert_ne!(words(b"one\xff"), words(b"two\xfe"));
}
