//! Unit tests for the crate root (`src:C139`): each function's own
//! branches, including the two private line helpers every text function
//! is built on.
//!
//! The vendored nix-shebang vectors (`tests/vectors.rs`) prove the PORT;
//! these prove the pieces, so a vector that starts failing points at one
//! function rather than at "the shebang crate".

use super::{
    ENV, Prelude, Shebang, first_line, get, has, parse, rest_after_first_line, strip,
    strip_preamble, strip_strict, wrap,
};

fn parsed(text: &str) -> Shebang {
    parse(text).unwrap_or_else(|| panic!("expected a shebang in {text:?}"))
}

fn bash_strict() -> Prelude {
    Prelude {
        shebang: Some(Shebang::env("bash")),
        strict: Some("set -euo pipefail".to_owned()),
    }
}

// ---------------------------------------------------------------------
// first_line / rest_after_first_line: the two halves of every split
// ---------------------------------------------------------------------

#[test]
fn first_line_stops_before_the_first_newline() {
    assert_eq!(first_line("a\nb\nc"), "a");
    assert_eq!(first_line("\nb"), "");
}

#[test]
fn first_line_of_text_without_a_newline_is_all_of_it() {
    assert_eq!(first_line("abc"), "abc");
    assert_eq!(first_line(""), "");
}

#[test]
fn rest_after_first_line_consumes_the_newline() {
    assert_eq!(rest_after_first_line("a\nb\nc"), "b\nc");
    assert_eq!(rest_after_first_line("a\n"), "");
}

#[test]
fn rest_after_a_final_line_without_newline_is_empty() {
    // What makes `strip("#!/bin/bash")` an empty body rather than the
    // shebang handed back unchanged.
    assert_eq!(rest_after_first_line("abc"), "");
    assert_eq!(rest_after_first_line(""), "");
}

#[test]
fn the_two_halves_rejoin_to_the_input() {
    for text in ["a\nb", "a\n", "\n", "x\ny\nz\n", "multi\n\nblank"] {
        let joined = format!("{}\n{}", first_line(text), rest_after_first_line(text));
        assert_eq!(joined, text, "{text:?}");
    }
}

#[test]
fn the_split_is_on_a_byte_newline_not_a_char_count() {
    // A multi-byte first line must not land a slice mid-character.
    assert_eq!(first_line("żółw\nb"), "żółw");
    assert_eq!(rest_after_first_line("żółw\nb"), "b");
}

// ---------------------------------------------------------------------
// has / get
// ---------------------------------------------------------------------

#[test]
fn has_is_true_only_for_hash_bang_at_byte_zero() {
    assert!(has("#!/bin/sh\n"));
    assert!(has("#!"));
    assert!(!has(" #!/bin/sh"));
    assert!(!has("\n#!/bin/sh"));
    assert!(!has("# comment"));
    assert!(!has(""));
}

#[test]
fn get_returns_the_line_without_its_newline() {
    assert_eq!(get("#!/bin/sh\necho hi\n"), Some("#!/bin/sh"));
    assert_eq!(get("#!/bin/sh"), Some("#!/bin/sh"));
}

#[test]
fn get_is_none_without_a_shebang() {
    assert_eq!(get("echo hi\n"), None);
    assert_eq!(get(""), None);
}

// ---------------------------------------------------------------------
// parse
// ---------------------------------------------------------------------

#[test]
fn parse_splits_interpreter_and_args() {
    let s = parsed("#!/usr/bin/awk -f\nBEGIN {}\n");
    assert_eq!(s.interpreter, "/usr/bin/awk");
    assert_eq!(s.args, vec!["-f".to_owned()]);
    assert!(!s.is_env);
}

#[test]
fn parse_marks_env_only_for_the_exact_env_path() {
    assert!(parsed("#!/usr/bin/env bash").is_env);
    assert!(!parsed("#!/bin/env bash").is_env);
    assert!(!parsed("#!/usr/bin/envy bash").is_env);
}

#[test]
fn parse_collapses_repeated_whitespace() {
    // An empty argument would become the resolved interpreter.
    let s = parsed("#!/usr/bin/env   bash \t -x  \n");
    assert_eq!(s.args, vec!["bash".to_owned(), "-x".to_owned()]);
}

#[test]
fn parse_allows_a_space_after_the_hash_bang() {
    let s = parsed("#! /bin/sh\n");
    assert_eq!(s.interpreter, "/bin/sh");
    assert!(s.args.is_empty());
}

#[test]
fn parse_is_none_for_a_bare_hash_bang() {
    // `#!` with nothing after it names no interpreter; a `Shebang` with an
    // empty one would resolve every such file to "".
    assert_eq!(parse("#!\n"), None);
    assert_eq!(parse("#!   \necho\n"), None);
}

#[test]
fn parse_is_none_without_a_shebang() {
    assert_eq!(parse("echo hi\n"), None);
    assert_eq!(parse(""), None);
}

#[test]
fn parse_reads_only_the_first_line() {
    let s = parsed("#!/bin/sh\n#!/bin/bash -x\n");
    assert_eq!(s.interpreter, "/bin/sh");
    assert!(s.args.is_empty());
}

// ---------------------------------------------------------------------
// Shebang constructors, resolved_interpreter, line
// ---------------------------------------------------------------------

#[test]
fn env_builds_the_portable_form() {
    let s = Shebang::env("python3");
    assert_eq!(s.interpreter, ENV);
    assert_eq!(s.args, vec!["python3".to_owned()]);
    assert!(s.is_env);
    assert_eq!(s.line(), "#!/usr/bin/env python3");
}

#[test]
fn absolute_has_no_args_and_detects_env() {
    let s = Shebang::absolute("/bin/sh");
    assert!(s.args.is_empty());
    assert!(!s.is_env);
    assert!(Shebang::absolute(ENV).is_env);
}

#[test]
fn with_args_keeps_argument_order_and_detects_env() {
    let s = Shebang::with_args("/usr/bin/awk", &["-f", "-v"]);
    assert_eq!(s.args, vec!["-f".to_owned(), "-v".to_owned()]);
    assert!(!s.is_env);
    assert!(Shebang::with_args(ENV, &["-S", "jq", "-f"]).is_env);
}

#[test]
fn resolved_interpreter_of_a_direct_shebang_is_the_interpreter() {
    assert_eq!(
        Shebang::with_args("/bin/bash", &["-e"]).resolved_interpreter(),
        "/bin/bash"
    );
}

#[test]
fn resolved_interpreter_of_env_is_its_first_argument() {
    assert_eq!(
        Shebang::with_args(ENV, &["bash", "-e"]).resolved_interpreter(),
        "bash"
    );
}

#[test]
fn resolved_interpreter_skips_env_dash_s() {
    assert_eq!(
        Shebang::with_args(ENV, &["-S", "jq", "-f"]).resolved_interpreter(),
        "jq"
    );
}

#[test]
fn resolved_interpreter_of_a_lone_dash_s_is_the_flag() {
    // Nothing follows `-S`, so there is no better answer than the flag;
    // the caller's table does not know it and answers `None`.
    assert_eq!(
        Shebang::with_args(ENV, &["-S"]).resolved_interpreter(),
        "-S"
    );
}

#[test]
fn resolved_interpreter_of_a_bare_env_is_env_itself() {
    assert_eq!(Shebang::absolute(ENV).resolved_interpreter(), ENV);
}

#[test]
fn line_renders_what_parse_reads() {
    for text in [
        "#!/bin/sh",
        "#!/usr/bin/env bash",
        "#!/usr/bin/env -S jq -f",
        "#!/usr/bin/awk -f",
    ] {
        assert_eq!(parsed(text).line(), text);
    }
}

// ---------------------------------------------------------------------
// strip
// ---------------------------------------------------------------------

#[test]
fn strip_removes_only_the_shebang_line() {
    assert_eq!(strip("#!/bin/sh\necho a\necho b\n"), "echo a\necho b\n");
}

#[test]
fn strip_leaves_text_without_a_shebang_unchanged() {
    assert_eq!(strip("echo a\n"), "echo a\n");
    assert_eq!(strip(""), "");
}

#[test]
fn strip_of_a_shebang_alone_is_empty() {
    assert_eq!(strip("#!/bin/sh"), "");
    assert_eq!(strip("#!/bin/sh\n"), "");
}

// ---------------------------------------------------------------------
// strip_strict
// ---------------------------------------------------------------------

#[test]
fn strip_strict_removes_the_shebang_and_the_exact_strict_line() {
    let text = "#!/usr/bin/env bash\nset -euo pipefail\necho hi\n";
    assert_eq!(strip_strict(text, &bash_strict()), "echo hi\n");
}

#[test]
fn strip_strict_keeps_a_strict_line_that_differs() {
    let text = "#!/usr/bin/env bash\nset -eu\necho hi\n";
    assert_eq!(strip_strict(text, &bash_strict()), "set -eu\necho hi\n");
}

#[test]
fn strip_strict_keeps_a_strict_line_that_is_not_first() {
    // Further down it is the script's own code.
    let text = "#!/usr/bin/env bash\necho a\nset -euo pipefail\n";
    assert_eq!(
        strip_strict(text, &bash_strict()),
        "echo a\nset -euo pipefail\n"
    );
}

#[test]
fn strip_strict_without_a_prelude_shebang_keeps_a_body_shebang() {
    // The measured bug the doc comment records: a guest body that itself
    // starts with `#!` must survive the round trip.
    let prelude = Prelude {
        shebang: None,
        strict: None,
    };
    let text = "#!/usr/bin/env python3\nprint(1)\n";
    assert_eq!(strip_strict(text, &prelude), text);
}

#[test]
fn strip_strict_removes_any_shebang_when_the_prelude_declares_one() {
    // The extract may have been edited since; the line is the prelude's.
    let text = "#!/bin/bash\nset -euo pipefail\necho hi\n";
    assert_eq!(strip_strict(text, &bash_strict()), "echo hi\n");
}

#[test]
fn strip_strict_with_a_shebang_prelude_and_bare_text() {
    assert_eq!(strip_strict("echo hi\n", &bash_strict()), "echo hi\n");
    assert_eq!(
        strip_strict("set -euo pipefail\necho hi\n", &bash_strict()),
        "echo hi\n"
    );
}

#[test]
fn strip_strict_with_a_strict_only_prelude() {
    let prelude = Prelude {
        shebang: None,
        strict: Some("set -e".to_owned()),
    };
    assert_eq!(strip_strict("set -e\necho\n", &prelude), "echo\n");
    // With no shebang declared, a file shebang is body, so the strict
    // line below it is not at the top and stays.
    assert_eq!(
        strip_strict("#!/bin/sh\nset -e\necho\n", &prelude),
        "#!/bin/sh\nset -e\necho\n"
    );
}

#[test]
fn strip_strict_of_the_strict_line_alone_is_empty() {
    assert_eq!(
        strip_strict("#!/usr/bin/env bash\nset -euo pipefail", &bash_strict()),
        ""
    );
}

// ---------------------------------------------------------------------
// strip_preamble
// ---------------------------------------------------------------------

#[test]
fn strip_preamble_removes_any_leading_set_dash_line() {
    for strict in ["set -e", "set -eu", "set -euo pipefail", "set -o errexit"] {
        let text = format!("#!/bin/sh\n{strict}\necho\n");
        assert_eq!(strip_preamble(&text), "echo\n", "{strict}");
    }
}

#[test]
fn strip_preamble_works_without_a_shebang() {
    assert_eq!(strip_preamble("set -e\necho\n"), "echo\n");
}

#[test]
fn strip_preamble_keeps_a_first_line_that_is_not_set_dash() {
    for first in [
        "echo set -e",
        "set +e",
        "  set -e",
        "setopt err_exit",
        "set",
    ] {
        let text = format!("{first}\necho\n");
        assert_eq!(strip_preamble(&text), text, "{first}");
    }
}

#[test]
fn strip_preamble_removes_only_one_line() {
    assert_eq!(strip_preamble("set -e\nset -u\necho\n"), "set -u\necho\n");
}

// ---------------------------------------------------------------------
// wrap
// ---------------------------------------------------------------------

#[test]
fn wrap_with_an_empty_prelude_is_the_body() {
    assert_eq!(wrap("echo\n", &Prelude::default()), "echo\n");
    assert_eq!(wrap("", &Prelude::default()), "");
}

#[test]
fn wrap_writes_shebang_then_strict_then_body_verbatim() {
    assert_eq!(
        wrap("echo  hi", &bash_strict()),
        "#!/usr/bin/env bash\nset -euo pipefail\necho  hi"
    );
}

#[test]
fn wrap_with_only_a_shebang_or_only_a_strict_line() {
    let shebang_only = Prelude {
        shebang: Some(Shebang::absolute("/bin/sh")),
        strict: None,
    };
    assert_eq!(wrap("echo\n", &shebang_only), "#!/bin/sh\necho\n");
    let strict_only = Prelude {
        shebang: None,
        strict: Some("set -e".to_owned()),
    };
    assert_eq!(wrap("echo\n", &strict_only), "set -e\necho\n");
}

#[test]
fn strip_strict_inverts_wrap_for_each_prelude_shape() {
    let preludes = [
        Prelude::default(),
        bash_strict(),
        Prelude {
            shebang: Some(Shebang::absolute("/bin/sh")),
            strict: None,
        },
        Prelude {
            shebang: None,
            strict: Some("set -e".to_owned()),
        },
    ];
    for prelude in &preludes {
        for body in ["", "echo\n", "#!/bin/sh\necho", "set -e\necho\n"] {
            let wrapped = wrap(body, prelude);
            assert_eq!(
                strip_strict(&wrapped, prelude),
                body,
                "{prelude:?} {body:?}"
            );
        }
    }
}
