//! `wrap` and `strip_strict` are inverses (`languages/api/src/lens:V63`).
//!
//! The law is what makes an extract reversible ON DISK rather than only
//! in memory: the extract file is `wrap(body, prelude)`, and inlining
//! reads the file back through `strip_strict(text, prelude)`. If those two
//! disagree by one newline, a round trip silently rewrites a file it was
//! supposed to restore.

use xenolith_shebang::{Prelude, Shebang, strip_strict, wrap};

fn preludes() -> Vec<Prelude> {
    vec![
        // bash, the common case
        Prelude {
            shebang: Some(Shebang::env("bash")),
            strict: Some("set -euo pipefail".to_owned()),
        },
        // sh, no strict line: `set -o pipefail` is not POSIX
        Prelude {
            shebang: Some(Shebang::absolute("/bin/sh")),
            strict: None,
        },
        // a guest whose interpreter takes an argument
        Prelude {
            shebang: Some(Shebang::with_args("/usr/bin/awk", &["-f"])),
            strict: None,
        },
        // nothing at all: a guest that is not executable (sql, css)
        Prelude {
            shebang: None,
            strict: None,
        },
    ]
}

fn bodies() -> Vec<&'static str> {
    vec![
        "echo hello\n",
        "echo hello",
        "",
        "\n",
        "# a comment\n\necho two\n",
        "set -euo pipefail\n",      // body that LOOKS like a strict line
        "#!/bin/sh\necho nested\n", // body that LOOKS like a shebang
    ]
}

#[test]
fn strip_strict_undoes_wrap_for_every_prelude_and_body() {
    for prelude in preludes() {
        for body in bodies() {
            let wrapped = wrap(body, &prelude);
            assert_eq!(
                strip_strict(&wrapped, &prelude),
                body,
                "round trip broke for prelude {prelude:?} and body {body:?}"
            );
        }
    }
}

#[test]
fn wrap_puts_the_shebang_first_and_the_strict_line_second() {
    let prelude = Prelude {
        shebang: Some(Shebang::env("bash")),
        strict: Some("set -euo pipefail".to_owned()),
    };
    assert_eq!(
        wrap("echo hi\n", &prelude),
        "#!/usr/bin/env bash\nset -euo pipefail\necho hi\n"
    );
}

#[test]
fn an_empty_prelude_leaves_the_body_untouched() {
    let prelude = Prelude {
        shebang: None,
        strict: None,
    };
    assert_eq!(wrap("SELECT 1;\n", &prelude), "SELECT 1;\n");
}

#[test]
fn a_shebang_renders_the_way_it_parsed() {
    assert_eq!(Shebang::env("bash").line(), "#!/usr/bin/env bash");
    assert_eq!(Shebang::absolute("/bin/sh").line(), "#!/bin/sh");
    assert_eq!(
        Shebang::with_args("/usr/bin/awk", &["-f"]).line(),
        "#!/usr/bin/awk -f"
    );
}

#[test]
fn an_env_shebang_resolves_to_its_first_argument() {
    let sb = Shebang::env("python3");
    assert!(sb.is_env);
    assert_eq!(sb.resolved_interpreter(), "python3");

    let sb = Shebang::absolute("/bin/bash");
    assert!(!sb.is_env);
    assert_eq!(sb.resolved_interpreter(), "/bin/bash");
}
