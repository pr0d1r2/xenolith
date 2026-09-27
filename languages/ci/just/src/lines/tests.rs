//! Unit tests for recipe body text (`src:C139`, `languages/ci/just:V180`,
//! `languages/api/src/lens:V39`).

use xenolith_lang_api::{Delim, DelimKind, Error, Span};

use super::{Logical, escape, logical, merged, unescape, written};

fn delim(kind: DelimKind) -> Delim {
    Delim {
        kind,
        open: Span::new(0, 0),
        body: Span::new(0, 0),
        close: Span::new(0, 0),
    }
}

fn lines_of(raw: &str) -> Vec<Logical> {
    logical(raw).unwrap_or_else(|e| panic!("{raw:?}: {e}"))
}

fn line(prefix: &str, text: &str) -> Logical {
    Logical {
        prefix: prefix.to_owned(),
        text: text.to_owned(),
        continued: false,
    }
}

// --- unescape ---------------------------------------------------------------

#[test]
fn a_line_body_is_the_lines_just_runs() {
    let raw = "    @echo a\n\n    -rm -f x\n      indented\n    @-true";
    let body = unescape(&delim(DelimKind::JustRecipe), raw);
    assert_eq!(body.as_deref(), Ok("echo a\nrm -f x\n  indented\ntrue"));
}

#[test]
fn a_continued_line_is_joined_as_just_joins_it() {
    // Measured on just 1.51.0: `echo 'a \` + `  b'` runs `echo 'a b'`.
    let got = lines_of("    -cc -o out \\\n      main.c\n    echo 'a \\\n      b'");
    assert_eq!(
        got,
        [
            Logical {
                prefix: "-".to_owned(),
                text: "cc -o out main.c".to_owned(),
                continued: true,
            },
            Logical {
                prefix: String::new(),
                text: "echo 'a b'".to_owned(),
                continued: true,
            },
        ]
    );
}

#[test]
fn four_braces_are_two_and_crlf_is_a_line_break() {
    let body = unescape(
        &delim(DelimKind::JustRecipe),
        "\techo {{{{x}}\r\n\techo y\r",
    );
    assert_eq!(body.as_deref(), Ok("echo {{x}}\necho y"));
}

#[test]
fn a_line_indented_less_than_the_first_is_a_parse_error() {
    let got = unescape(&delim(DelimKind::JustRecipe), "    a\n  b");
    assert!(matches!(got, Err(Error::Parse { .. })), "{got:?}");
}

#[test]
fn a_shebang_body_is_one_script_with_its_blank_lines() {
    let raw = "    #!/usr/bin/env python3\n    import sys\n\n    @print({{{{}})";
    let body = unescape(&delim(DelimKind::JustShebangRecipe), raw);
    assert_eq!(
        body.as_deref(),
        Ok("#!/usr/bin/env python3\nimport sys\n\n@print({{}})")
    );
}

#[test]
fn another_hosts_delimiter_is_unsupported() {
    let got = unescape(&delim(DelimKind::ArgvString), "x");
    assert!(matches!(got, Err(Error::Unsupported { .. })), "{got:?}");
    let got = escape(&delim(DelimKind::NixIndented), "x");
    assert!(matches!(got, Err(Error::Unsupported { .. })), "{got:?}");
}

// --- escape -----------------------------------------------------------------

#[test]
fn escape_indents_and_doubles_braces() {
    let line = delim(DelimKind::JustRecipe);
    assert_eq!(
        escape(&line, "echo {{x}}\n  more").as_deref(),
        Ok("    echo {{{{x}}\n      more")
    );
    let script = delim(DelimKind::JustShebangRecipe);
    assert_eq!(
        escape(&script, "#!/bin/sh\n\n-x").as_deref(),
        Ok("    #!/bin/sh\n\n    -x")
    );
}

#[test]
fn escape_refuses_what_just_would_read_otherwise() {
    let line = delim(DelimKind::JustRecipe);
    for body in [" lead", "@echo", "ok\n-rm", "cc \\"] {
        let got = escape(&line, body);
        assert!(matches!(got, Err(Error::Parse { .. })), "{body:?}: {got:?}");
    }
}

#[test]
fn escape_then_unescape_is_the_identity_for_bodies_unescape_gives() {
    for (kind, body) in [
        (DelimKind::JustRecipe, "echo {{a}} {{{{b}}\nls"),
        (DelimKind::JustRecipe, "x"),
        (
            DelimKind::JustShebangRecipe,
            "#!/bin/bash\nset -e\n\n  y {{z}}",
        ),
    ] {
        let d = delim(kind);
        let back = escape(&d, body).and_then(|raw| unescape(&d, &raw));
        assert_eq!(back.as_deref(), Ok(body));
    }
}

// --- merged / written -------------------------------------------------------

#[test]
fn merged_appends_or_true_to_dash_lines_and_notes_all_quiet() {
    let got = merged(&[line("-", "rm x"), line("", "ls")]);
    assert_eq!(got, Ok(("rm x || true\nls".to_owned(), false)));
    let quiet = merged(&[line("@", "a"), line("@-", "b")]);
    assert_eq!(quiet, Ok(("a\nb || true".to_owned(), true)));
}

#[test]
fn merged_refuses_what_written_could_not_give_back() {
    let continued = Logical {
        continued: true,
        ..line("", "a b")
    };
    for body in [
        vec![line("@", "a"), line("", "b")],
        vec![continued],
        vec![line("-@", "a")],
        vec![line("-", "a # note")],
        vec![line("-", " ")],
        vec![line("", "grep x || true")],
    ] {
        assert!(merged(&body).is_err(), "{body:?}");
    }
}

#[test]
fn written_is_merged_read_backwards() {
    let got = written("rm x || true\nls\n\n", "    ", false, "\n");
    assert_eq!(got.as_deref(), Ok("-rm x\n    ls"));
    let quiet = written("a\nb || true\n", "\t", true, "\r\n");
    assert_eq!(quiet.as_deref(), Ok("@a\r\n\t@-b"));
    let braces = written("echo {{x}}", "  ", false, "\n");
    assert_eq!(braces.as_deref(), Ok("echo {{{{x}}"));
}

#[test]
fn written_refuses_lines_just_would_misread_and_an_empty_body() {
    for body in ["-rm", "@echo", "cc \\", "", "\n  \n"] {
        let got = written(body, "  ", false, "\n");
        assert!(matches!(got, Err(Error::Parse { .. })), "{body:?}: {got:?}");
    }
}
