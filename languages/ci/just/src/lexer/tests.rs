//! Unit tests for `lexer.rs` (`src:C139`): just's tokens, as GRAMMAR.md
//! names them, each with its exact span.

use xenolith_lang_api::Error;

use super::{Kind, Lexer};

/// Every token of `src` up to EOF, as its kind and text.
fn tokens(src: &str) -> Vec<(Kind, &str)> {
    let mut lexer = Lexer::new(src, 0);
    let mut out = Vec::new();
    loop {
        let token = lexer.next().unwrap_or_else(|e| panic!("{src:?}: {e}"));
        if token.kind == Kind::Eof {
            return out;
        }
        out.push((token.kind, token.span.of(src).unwrap_or_default()));
    }
}

fn refused(src: &str) -> String {
    let mut lexer = Lexer::new(src, 0);
    loop {
        match lexer.next() {
            Err(Error::Parse { message, .. }) => return message,
            Err(other) => panic!("{src:?}: {other:?}"),
            Ok(token) if token.kind == Kind::Eof => panic!("{src:?}: lexed clean"),
            Ok(_) => {}
        }
    }
}

#[test]
fn names_operators_newlines_and_comments() {
    assert_eq!(
        tokens("a-b_1 := x == y != z =~ w !~ v && u || t ++ s :: r # c\r\n"),
        [
            (Kind::Name, "a-b_1"),
            (Kind::Punct, ":="),
            (Kind::Name, "x"),
            (Kind::Punct, "=="),
            (Kind::Name, "y"),
            (Kind::Punct, "!="),
            (Kind::Name, "z"),
            (Kind::Punct, "=~"),
            (Kind::Name, "w"),
            (Kind::Punct, "!~"),
            (Kind::Name, "v"),
            (Kind::Punct, "&&"),
            (Kind::Name, "u"),
            (Kind::Punct, "||"),
            (Kind::Name, "t"),
            (Kind::Punct, "++"),
            (Kind::Name, "s"),
            (Kind::Punct, "::"),
            (Kind::Name, "r"),
            (Kind::Comment, "# c"),
            (Kind::Newline, "\r\n"),
        ]
    );
}

#[test]
fn single_character_punctuation() {
    let got: Vec<&str> = tokens("( ) [ ] { } , : + / ! = * $ @ ?")
        .into_iter()
        .map(|(_, text)| text)
        .collect();
    assert_eq!(
        got,
        ["(", ")", "[", "]", "{", "}", ",", ":", "+", "/", "!", "=", "*", "$", "@", "?"]
    );
}

#[test]
fn every_string_kind_and_its_prefixes() {
    assert_eq!(
        tokens("'a\\d' \"b\\n\\\"c\" '''x\ny''' \"\"\"p\nq\"\"\" x'~' f\"{{n}}\""),
        [
            (Kind::Str, "'a\\d'"),
            (Kind::Str, "\"b\\n\\\"c\""),
            (Kind::Str, "'''x\ny'''"),
            (Kind::Str, "\"\"\"p\nq\"\"\""),
            (Kind::Str, "x'~'"),
            (Kind::Str, "f\"{{n}}\""),
        ]
    );
}

#[test]
fn a_name_that_merely_starts_like_a_prefix_is_a_name() {
    assert_eq!(tokens("xs fx"), [(Kind::Name, "xs"), (Kind::Name, "fx")]);
}

#[test]
fn backticks_single_and_indented() {
    assert_eq!(
        tokens("`date` ```\nls\n```"),
        [(Kind::Backtick, "`date`"), (Kind::Backtick, "```\nls\n```")]
    );
}

#[test]
fn what_just_rejects_is_a_parse_error_naming_where() {
    for (src, what) in [
        ("'open", "unterminated"),
        ("\"open", "unterminated"),
        ("'''open", "unterminated"),
        ("`open", "unterminated"),
        ("\"\\d\"", "escape"),
        ("x := ;", "unexpected"),
    ] {
        let message = refused(src);
        assert!(message.contains(what), "{src:?}: {message}");
        assert!(message.contains("1:"), "{src:?}: {message}");
    }
}

#[test]
fn a_cooked_string_takes_unicode_and_line_continuation_escapes() {
    assert_eq!(
        tokens("\"\\u{1F600}\\t\\r\\\\\\\nx\""),
        [(Kind::Str, "\"\\u{1F600}\\t\\r\\\\\\\nx\"")]
    );
}

#[test]
fn the_lexer_starts_where_it_is_told() {
    let mut lexer = Lexer::new("ab cd", 3);
    let token = lexer.next().unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(token.span.of("ab cd"), Some("cd"));
}
