//! Unit tests for `script.rs` (`src:C139`): Tcl's word syntax as a tree,
//! every span exact, and valid Tcl the vendored tree-sitter grammar
//! rejected (`languages/shells/tcl:B1`, `languages/shells/tcl:B2`, and a
//! parenthesis in a word) read like any other.

use xenolith_lang_api::{Error, LangId, Span};

use super::{Command, Part, Script, Word, WordKind, parse, parse_at};

fn script(src: &str) -> Script {
    parse(src).unwrap_or_else(|e| panic!("{src:?}: {e}"))
}

fn of(span: Span, src: &str) -> &str {
    span.of(src)
        .unwrap_or_else(|| panic!("{span:?} outside {src:?}"))
}

/// Every word of every command, as the bytes it spans.
fn words(src: &str) -> Vec<Vec<String>> {
    script(src)
        .commands
        .iter()
        .map(|c| c.words.iter().map(|w| of(w.span, src).to_owned()).collect())
        .collect()
}

fn only_command(src: &str) -> Command {
    match script(src).commands.as_slice() {
        [only] => only.clone(),
        other => panic!("{src:?}: expected one command, got {other:#?}"),
    }
}

fn word(src: &str, at: usize) -> Word {
    only_command(src)
        .words
        .get(at)
        .cloned()
        .unwrap_or_else(|| panic!("{src:?}: no word {at}"))
}

fn clean(src: &str) -> bool {
    parse(src).is_ok()
}

#[test]
fn commands_end_at_newlines_and_semicolons_and_comments_are_skipped() {
    assert_eq!(
        words("puts a; puts b\n# exec sh -c {x}\nset x 1\n"),
        [vec!["puts", "a"], vec!["puts", "b"], vec!["set", "x", "1"]]
    );
}

#[test]
fn a_word_spans_its_delimiters() {
    assert_eq!(
        words("puts foo$bar [x]y \"q\" {b} $z\n"),
        [vec!["puts", "foo$bar", "[x]y", "\"q\"", "{b}", "$z"]]
    );
}

#[test]
fn a_braced_word_holds_its_body_verbatim() {
    let src = "exec sh -c {a | $(b) [c]}\n";
    let WordKind::Braced { open, body, close } = word(src, 3).kind else {
        panic!("not braced");
    };
    assert_eq!(of(open, src), "{");
    assert_eq!(of(body, src), "a | $(b) [c]");
    assert_eq!(of(close, src), "}");
}

#[test]
fn a_quoted_word_holds_its_substitutions() {
    let src = "puts \"echo $x [pwd] \\n ${y} $a(b)\"\n";
    let WordKind::Quoted { open, close, parts } = word(src, 1).kind else {
        panic!("not quoted");
    };
    assert_eq!(of(open, src), "\"");
    assert_eq!(of(close, src), "\"");
    let subs: Vec<&str> = parts
        .iter()
        .filter_map(|p| match p {
            Part::Var { span, .. } | Part::Cmd { span, .. } => Some(of(*span, src)),
            _ => None,
        })
        .collect();
    assert_eq!(subs, ["$x", "[pwd]", "${y}", "$a(b)"]);
    assert!(
        parts
            .iter()
            .any(|p| matches!(p, Part::Escape(s) if of(*s, src) == "\\n"))
    );
}

#[test]
fn a_command_substitution_holds_a_script() {
    let src = "set x [exec sh -c {a}]\n";
    let WordKind::Bare { parts } = word(src, 2).kind else {
        panic!("not bare");
    };
    let [Part::Cmd { span, script }] = parts.as_slice() else {
        panic!("{parts:#?}");
    };
    assert_eq!(of(*span, src), "[exec sh -c {a}]");
    let inner: Vec<&str> = script
        .commands
        .iter()
        .flat_map(|c| c.words.iter().map(|w| of(w.span, src)))
        .collect();
    assert_eq!(inner, ["exec", "sh", "-c", "{a}"]);
}

#[test]
fn an_array_index_substitutes_and_a_lone_dollar_is_text() {
    let src = "puts $a([pwd]) $ x\n";
    let WordKind::Bare { parts } = word(src, 1).kind else {
        panic!("not bare");
    };
    let [Part::Var { index, .. }] = parts.as_slice() else {
        panic!("{parts:#?}");
    };
    assert!(index.iter().any(|p| matches!(p, Part::Cmd { .. })));
    let WordKind::Bare { parts } = word(src, 2).kind else {
        panic!("not bare");
    };
    assert!(matches!(parts.as_slice(), [Part::Text(_)]), "{parts:#?}");
}

#[test]
fn the_expansion_prefix_is_marked() {
    let w = word("exec sh -c {*}$cmd\n", 3);
    assert!(w.expand);
    let w = word("puts {*}\n", 1);
    assert!(!w.expand, "a bare `{{*}}` is a braced word holding `*`");
}

#[test]
fn a_bare_expansion_prefix_may_end_the_text_and_a_comment_may_hold_escapes() {
    let w = word("puts {*}", 1);
    assert!(!w.expand);
    assert_eq!(
        words("# a \\\n still comment\nputs b\n"),
        [vec!["puts", "b"]]
    );
}

#[test]
fn a_backslash_newline_separates_words() {
    assert_eq!(words("puts a \\\n  b\n"), [vec!["puts", "a", "b"]]);
}

#[test]
fn parse_at_reads_a_body_with_spans_into_the_whole_text() {
    let src = "proc f {} {\n  exec sh -c {x}\n}\n";
    let WordKind::Braced { body, .. } = word(src, 3).kind else {
        panic!("not braced");
    };
    let inner = parse_at(src, body).unwrap_or_else(|e| panic!("{e}"));
    let spans: Vec<&str> = inner
        .commands
        .iter()
        .flat_map(|c| c.words.iter().map(|w| of(w.span, src)))
        .collect();
    assert_eq!(spans, ["exec", "sh", "-c", "{x}"]);
}

#[test]
fn a_body_outside_the_text_is_refused() {
    assert!(matches!(
        parse_at("puts a", Span::new(3, 40)),
        Err(Error::Parse { .. })
    ));
}

#[test]
fn the_last_word_may_end_the_text_and_braces_hide_escaped_braces() {
    assert_eq!(words("puts a"), [vec!["puts", "a"]]);
    assert_eq!(
        words("puts {a\\}b} $ns::v"),
        [vec!["puts", "{a\\}b}", "$ns::v"]]
    );
}

#[test]
fn an_error_on_the_first_line_of_a_body_counts_its_column_in_the_whole_text() {
    let src = "proc f {} {puts {a}b}\n";
    let WordKind::Braced { body, .. } = word(src, 3).kind else {
        panic!("not braced");
    };
    match parse_at(src, body) {
        Err(Error::Parse { message, .. }) => assert!(message.starts_with("1:20:"), "{message}"),
        other => panic!("expected a parse error, got {other:?}"),
    }
}

#[test]
fn what_tcl_rejects_is_a_parse_error_naming_where() {
    for src in [
        "exec sh -c {a | b\n",
        "puts \"a\n",
        "set x [pwd\n",
        "set x {a}b\n",
        "puts \"a\"b\n",
    ] {
        match parse(src) {
            Err(Error::Parse { lang, message }) => {
                assert_eq!(lang, LangId::Tcl);
                assert!(message.contains("1:"), "{src:?}: {message}");
            }
            other => panic!("{src:?}: expected a parse error, got {other:?}"),
        }
    }
}

#[test]
fn an_error_in_a_body_is_reported_at_its_place_in_the_whole_text() {
    let src = "proc f {} {\n  puts {a}b\n}\n";
    let WordKind::Braced { body, .. } = word(src, 3).kind else {
        panic!("not braced");
    };
    match parse_at(src, body) {
        Err(Error::Parse { message, .. }) => assert!(message.contains("2:"), "{message}"),
        other => panic!("expected a parse error, got {other:?}"),
    }
}

#[test]
fn valid_tcl_the_vendored_grammar_rejected_is_clean() {
    // `languages/shells/tcl:B1`, open cases included.
    assert!(clean("expect \"$ \"\n"));
    assert!(clean("puts hi; set x 1\n"));
    assert!(clean("set x [expr {1+2}]\n"));
    assert!(clean("set x $y(z)\nset a(x) $b($k)\nputs foo$a(b)\n"));
    assert!(clean("puts \"a]b\"\n"));
    assert!(clean("if {1} then {puts a} elseif {0} then {puts b}\n"));
    assert!(clean("set x a(b)\n"));
    assert!(clean("set x [f]($y)\n"));
    assert!(clean("expr {0x1F}\n"));
    // `languages/shells/tcl:B2`.
    assert!(clean("set x [regexp -line {^ping: } $out]\n"));
    assert!(clean("regexp -all -inline {\\S+} $line\n"));
    assert!(clean("regexp {a} {b}\n"));
    // A parenthesis in a word is text to Tcl.
    assert!(clean("set x {(a)}\n"));
    assert!(clean("puts (a)\n"));
    assert!(clean("set p {^(\\d+)$}\n"));
    assert!(clean("proc f {} { puts (a) }\n"));
}
