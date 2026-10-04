//! Shell as a HOST: which files it claims (`languages/shells/shell:T135`).
//!
//! A claim is the decision to parse a file with tree-sitter-bash, and a
//! wrong one is not an error anyone sees: a `*.bats` file parses as bash
//! and its `@test` blocks read as scripts with control flow
//! (`languages/shells/shell:V137`). So the negative cases matter as much as the
//! positive ones (`tests:V15`), and bats is the one named by the spec.
//!
//! Its SITES (`languages/shells/shell:T15`) are pinned case by case in
//! `tests/fixtures/`; this file holds what a fixture line cannot say --
//! the delimiter kinds, the unescaped body, and a file that does not
//! parse.

use std::path::Path;

use xenolith_lang_api::{Delim, DelimKind, Error, Guest, GuestEnv, Host, LangId, Site, Span};
use xenolith_lang_shell::{ShellGuest, ShellHost};

fn claims(path: &str, head: &str) -> bool {
    ShellHost.claims(Path::new(path), head)
}

fn sites(src: &str) -> Vec<Site> {
    ShellHost
        .sites(src)
        .unwrap_or_else(|e| panic!("sites failed: {e}"))
}

/// The one site in `src`.
fn only(src: &str) -> Site {
    let found = sites(src);
    assert_eq!(found.len(), 1, "{found:#?}");
    found.into_iter().next().unwrap_or_else(|| unreachable!())
}

/// The body of `site` as its guest reads it.
fn body(src: &str, site: &Site) -> String {
    let raw = site.delim.body.of(src).unwrap_or_default();
    ShellHost
        .unescape(&site.delim, raw)
        .unwrap_or_else(|e| panic!("unescape failed: {e}"))
}

#[test]
fn the_host_is_shell() {
    assert_eq!(ShellHost.id(), LangId::Shell);
}

#[test]
fn shell_files_are_claimed_by_extension() {
    assert!(claims("scripts/guard/x.sh", ""));
    assert!(claims("x.bash", ""));
    // The extension decides, whatever the first line says.
    assert!(claims("scripts/x.sh", "set -euo pipefail"));
}

#[test]
fn envrc_is_claimed_by_name() {
    assert!(claims(".envrc", "use flake"));
    assert!(claims("sub/dir/.envrc", ""));
}

#[test]
fn a_file_is_claimed_by_a_shell_shebang_alone() {
    assert!(claims("bin/tool", "#!/usr/bin/env bash"));
    assert!(claims("bin/tool", "#!/bin/sh"));
    assert!(claims("bin/tool", "#!/bin/dash -e"));
    assert!(claims("bin/tool", "#!/bin/ksh"));
}

#[test]
fn a_zsh_file_is_never_claimed() {
    // `languages/shells/shell:V310`: the host's checks cannot see which
    // file they are given, and shellcheck refuses zsh outright (SC1071),
    // so a claimed zsh file failed `xnl lint` whatever it said; the bash
    // grammar rejects zsh-only syntax, so `xnl check` failed it too. The
    // file stays an extract, whose zsh dialect gets `zsh -n`.
    for head in [
        "#!/usr/bin/env zsh",
        "#!/bin/zsh -f",
        "#!/usr/bin/env -S zsh -eu",
        "#!/opt/homebrew/bin/zsh\nsetopt err_exit",
    ] {
        assert!(!claims("bin/tool", head), "{head:?}");
        assert!(!claims("bin/tool.zsh", head), "{head:?}");
        // The shebang outranks the extension, as it does for the tools.
        assert!(!claims("scripts/x.sh", head), "{head:?}");
    }
    // A shell merely named after zsh is not zsh, and with no shebang the
    // extension still decides.
    assert!(!claims("x.zsh", ""));
    assert!(claims("x.sh", "# zsh-compatible\nset -eu"));
    assert!(claims("x.bash", "#!/usr/bin/env bash\nzsh -c 'print hi'"));
}

#[test]
fn every_shell_dialect_but_zsh_is_claimed_by_its_shebang() {
    // The same table `shebang::guest_of` reads: whatever new sh-family
    // name it learns is claimed, and zsh is the one exception (V310).
    for name in ["sh", "bash", "dash", "ksh", "ash", "zsh"] {
        let head = format!("#!/usr/bin/env {name}");
        assert_eq!(claims("bin/tool", &head), name != "zsh", "{name}");
    }
}

#[test]
fn a_bats_file_is_never_claimed() {
    // `languages/shells/shell:V137`: the bash grammar ACCEPTS bats and gets the
    // verdict wrong, so the refusal cannot wait for a parse error.
    assert!(!claims("tests/unit/x.bats", ""));
    assert!(!claims("tests/unit/x.bats", "#!/usr/bin/env bats"));
    assert!(!claims("tests/unit/x.bats", "#!/usr/bin/env bash"));
}

#[test]
fn other_files_are_not_claimed() {
    assert!(!claims("x.nix", ""));
    assert!(!claims("x.py", "#!/usr/bin/env python3"));
    assert!(!claims("bin/tool", "#!/usr/bin/env ruby"));
    assert!(!claims("bin/tool", ""));
    assert!(!claims("bin/tool", "echo hi"));
    // A shebang names the interpreter; one that merely mentions a shell
    // in its arguments does not make the file shell.
    assert!(!claims("bin/tool", "#!/usr/bin/env nix-shell"));
    // `.sh` as a directory name, or `sh` as a bare name, is not an
    // extension.
    assert!(!claims("x.sh/readme", ""));
    assert!(!claims("sh", ""));
    assert!(!claims("x.envrc", ""));
}

#[test]
fn the_host_checks_its_own_files_with_shellcheck_and_shfmt() {
    // `languages/shells/shell` §G: the default linters. The dialect is the
    // file's own shebang or extension, which both tools read themselves.
    let checks: Vec<String> = ShellHost
        .checks()
        .iter()
        .map(|cmd| cmd.argv.join(" "))
        .collect();
    assert_eq!(checks, ["shellcheck --format=json", "shfmt --diff"]);
    let fixers: Vec<String> = ShellHost
        .fixers()
        .iter()
        .map(|cmd| cmd.argv.join(" "))
        .collect();
    assert_eq!(fixers, ["shfmt --write"]);
}

#[test]
fn a_heredoc_site_names_its_tag_quoting_and_indent() {
    let src = "python3 <<'PY'\nprint(1)\nPY\n";
    let site = only(src);
    assert_eq!(site.guest, LangId::Python);
    assert_eq!(
        site.delim.kind,
        DelimKind::Heredoc {
            tag: "PY".to_owned(),
            quoted: true,
            strip_indent: false,
        }
    );
    assert_eq!(site.delim.open.of(src), Some("<<'PY'"));
    assert_eq!(site.delim.close.of(src), Some("PY"));
    assert_eq!(body(src, &site), "print(1)\n");
}

#[test]
fn an_unquoted_heredoc_decodes_the_escapes_the_shell_decodes() {
    // In an unquoted heredoc the shell removes a backslash before `$`,
    // a backtick, a backslash and a newline -- and ONLY those: `\"` and
    // `\n` reach the interpreter as written.
    let src = "ruby <<RB\nputs \"\\$HOME \\` \\\\ \\\" \\n\"\nx = 1 + \\\n  2\nRB\n";
    let site = only(src);
    assert_eq!(
        site.delim.kind,
        DelimKind::Heredoc {
            tag: "RB".to_owned(),
            quoted: false,
            strip_indent: false,
        }
    );
    assert_eq!(
        body(src, &site),
        "puts \"$HOME ` \\ \\\" \\n\"\nx = 1 +   2\n"
    );
}

#[test]
fn a_quoted_heredoc_is_read_verbatim() {
    let src = "perl <<\"PL\"\nprint \"\\$x\\n\";\nPL\n";
    let site = only(src);
    assert!(site.holes.is_empty(), "a quoted tag has no holes");
    assert_eq!(body(src, &site), "print \"\\$x\\n\";\n");
}

#[test]
fn a_dash_heredoc_strips_leading_tabs_from_body_and_terminator() {
    // `<<-`: the shell drops leading TABS (not spaces) from every body
    // line and from the terminator line. The body span starts at its
    // line, so the tabs are the host's to strip, not the guest's to see.
    let src = "if true; then\n\tpython3 <<-PY\n\tif True:\n\t    print(1)\n\tPY\nfi\n";
    let site = only(src);
    assert_eq!(
        site.delim.kind,
        DelimKind::Heredoc {
            tag: "PY".to_owned(),
            quoted: false,
            strip_indent: true,
        }
    );
    assert_eq!(
        site.delim.body.of(src),
        Some("\tif True:\n\t    print(1)\n")
    );
    assert_eq!(body(src, &site), "if True:\n    print(1)\n");
}

#[test]
fn an_argv_string_is_read_verbatim() {
    let src = "python3 -c 'print(\"a\\nb\")'\n";
    let site = only(src);
    assert_eq!(site.delim.kind, DelimKind::ArgvString);
    assert_eq!(site.delim.open.of(src), Some("'"));
    assert_eq!(site.delim.close.of(src), Some("'"));
    assert_eq!(body(src, &site), "print(\"a\\nb\")");
}

#[test]
fn a_double_quoted_argv_string_marks_its_expansions_as_holes() {
    let src = "sh -c \"cd $dir && make ${target} $(nproc)\"\n";
    let site = only(src);
    let holes: Vec<&str> = site.holes.iter().filter_map(|h| h.of(src)).collect();
    assert_eq!(holes, ["$dir", "${target}", "$(nproc)"]);
}

#[test]
fn sites_are_sorted_by_span() {
    let src = "bash -c 'a | b'\npython3 <<'PY'\nprint(1)\nPY\nruby -e 'p 1'\n";
    let opens: Vec<usize> = sites(src).iter().map(|s| s.delim.open.start).collect();
    let mut sorted = opens.clone();
    sorted.sort_unstable();
    assert_eq!(opens.len(), 3);
    assert_eq!(opens, sorted);
}

#[test]
fn a_file_that_does_not_parse_is_an_error_not_a_clean_file() {
    // `languages:V78`: spans inside and after an ERROR node are not
    // trustworthy enough to cut a file on, so the file is reported as
    // broken rather than scanned in part.
    let found = ShellHost.sites("python3 -c 'print(1)'\nif then fi (\n");
    assert!(
        matches!(
            found,
            Err(Error::Parse {
                lang: LangId::Shell,
                ..
            })
        ),
        "{found:?}"
    );
}

#[test]
fn a_child_shell_extract_runs_under_its_own_options_only() {
    // `languages/shells/shell:V82` end to end: the site's env is what the
    // guest's prelude reproduces. The enclosing `set -e` never reached
    // the `bash -c` child, so its extract gets no strict line; the child
    // that set `-e` itself keeps it.
    let src = "set -e\nbash -c 'ls | wc -l'\nbash -e -c 'ls | wc -l'\n";
    let preludes: Vec<(String, Option<String>)> = sites(src)
        .iter()
        .map(|site| {
            let prelude = ShellGuest.prelude(&site.env);
            let line = prelude.shebang.map(|s| s.line()).unwrap_or_default();
            (line, prelude.strict)
        })
        .collect();
    assert_eq!(
        preludes,
        [
            ("#!/usr/bin/env bash".to_owned(), None),
            ("#!/usr/bin/env bash".to_owned(), Some("set -e".to_owned())),
        ]
    );
}

#[test]
fn a_non_shell_site_leaves_its_env_to_the_guest() {
    // Dialect and options are shell facts (`languages/shells/shell:V82`); a
    // python site states none, so the python guest's defaults apply.
    let site = only("python3 -u -c 'print(1)'\n");
    assert_eq!(site.env, GuestEnv::default());
}

#[test]
fn unescape_refuses_a_delimiter_shell_never_writes() {
    let delim = Delim {
        kind: DelimKind::NixIndented,
        open: Span::new(0, 2),
        body: Span::new(2, 2),
        close: Span::new(2, 4),
    };
    assert_eq!(
        ShellHost.unescape(&delim, "x"),
        Err(Error::unsupported(LangId::Shell, "unescape"))
    );
}
