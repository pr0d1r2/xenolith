//! Unit tests for the merge refusals (`src:C139`,
//! `languages/ci/just:V180`, `languages/ci/just:B1`).

use super::{Token, changes_state, escapes_errexit, sequences, tokens};

fn word(w: &str) -> Token {
    Token::Word(w.to_owned())
}

#[test]
fn tokens_keep_quotes_escapes_and_substitutions_in_their_word() {
    assert_eq!(
        tokens("echo 'a; b' \"c && d\" e\\;f $(g; h) `i|j`;k"),
        [
            word("echo"),
            word("'a; b'"),
            word("\"c && d\""),
            word("e\\;f"),
            word("$(g; h)"),
            word("`i|j`"),
            Token::Op(";"),
            word("k"),
        ]
    );
}

#[test]
fn tokens_split_operators_and_stop_at_a_comment() {
    assert_eq!(
        tokens("a&&b||c|d&e (f) # g; cd"),
        [
            word("a"),
            Token::Op("&&"),
            word("b"),
            Token::Op("||"),
            word("c"),
            Token::Op("|"),
            word("d"),
            Token::Op("&"),
            word("e"),
            Token::Op("("),
            word("f"),
            Token::Op(")"),
        ]
    );
    assert_eq!(tokens("x#y"), [word("x#y")]);
}

#[test]
fn state_changing_lines_are_found_in_any_command_position() {
    for line in [
        "cd sub",
        "export FOO=1",
        "FOO=1",
        "FOO=1 BAR+=2",
        "set -x",
        "true && cd sub",
        "if true; then cd sub; fi",
        "( cd sub )",
        "! unset X",
        ". ./env.sh",
        "source env.sh",
        "exit 0",
        "exec make",
        "f() { echo; }",
        "function f { echo; }",
        "alias ll='ls -l'",
    ] {
        assert!(changes_state(line), "{line:?}");
    }
}

#[test]
fn ordinary_commands_do_not_change_state() {
    for line in [
        "cargo build",
        "FOO=1 make",
        "make CC=gcc",
        "echo cd set export",
        "echo 'cd x'; ls",
        "grep -q x file || echo missing",
        "echo $(cd sub && pwd)",
        "# cd sub",
    ] {
        assert!(!changes_state(line), "{line:?}");
    }
}

#[test]
fn sequences_are_semicolons_outside_quotes() {
    assert!(sequences("a; b"));
    assert!(sequences("for f in *; do echo $f; done"));
    assert!(!sequences("echo 'a; b'"));
    assert!(!sequences("a && b"));
}

#[test]
fn and_lists_and_negations_escape_errexit() {
    assert!(escapes_errexit("test -f x && rm x"));
    assert!(escapes_errexit("! grep -q x f"));
    assert!(escapes_errexit("FOO=1 ! false"));
    assert!(!escapes_errexit("a || b"));
    assert!(!escapes_errexit("a | b"));
    assert!(!escapes_errexit("echo '&&' !"));
    assert!(!escapes_errexit("[ ! -f x ]"));
}
