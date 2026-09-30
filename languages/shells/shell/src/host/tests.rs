//! Unit tests for `host.rs` (`src:C139`): the claim, the walk and the
//! unescaping, part by part.

use std::path::Path;

use xenolith_lang_api::{
    Delim, DelimKind, Error, Format, GuestEnv, Host, Invoke, LangId, LoadRef, Site, Span, shebang,
};

use super::{
    BATS, EXTENSIONS, FILENAMES, ShellHost, ZSH, hole, is_zsh, line_start, parse, raw, strip_tabs,
    unbackslash, walk,
};

fn found(src: &str) -> Vec<Site> {
    let tree = parse(src).unwrap_or_else(|e| panic!("{e}"));
    let mut out = Vec::new();
    walk(tree.root_node(), src, &mut out);
    out
}

#[test]
fn the_walk_reaches_commands_at_any_depth() {
    let src = "f() { for x in 1; do (python3 -c 'print(1)'); done; }\n";
    let sinks: Vec<String> = found(src).into_iter().map(|site| site.sink).collect();
    assert_eq!(sinks, ["python3 -c"]);
}

#[test]
fn bash_valid_parameter_and_redirect_forms_are_parseable() {
    for src in [
        "#!/usr/bin/env bash\nexec 3<>/dev/null\n",
        "#!/usr/bin/env bash\necho \"${b:+ ($b)}\"\n",
        "#!/usr/bin/env bash\nx=\"${c:+$c; }y\"\n",
        "#!/usr/bin/env bash\nprintf %s \"${s:$i:1}\"\n",
    ] {
        assert!(ShellHost.sites(src).is_ok(), "{src:?}");
    }
    assert!(ShellHost.sites("if; then\n").is_err());
}

#[test]
fn bash_valid_forms_do_not_hide_embedded_sites() {
    let src = "echo \"${b:+ ($b)}\"\npython3 -c 'print(1)'\n";
    let sites = ShellHost.sites(src).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(sites.len(), 1);
    assert_eq!(sites.first().map(|site| site.sink.as_str()), Some("python3 -c"));
}

#[test]
fn a_heredoc_with_a_second_stdin_is_not_a_site() {
    assert!(found("python3 <<'PY' <input.txt\nprint(1)\nPY\n").is_empty());
    assert!(found("python3 <<'PY'\nprint(1)\nPY\n").len() == 1);
}

#[test]
fn a_heredoc_beside_another_statement_redirect_is_not_a_site() {
    // Two redirects on the statement itself: which stdin the interpreter
    // reads as its program is not the host's to guess.
    assert!(found("python3 <in.txt <<'PY'\nprint(1)\nPY\n").is_empty());
    assert!(found("python3 2>err.log <<'PY'\nprint(1)\nPY\n").is_empty());
}

#[test]
fn a_file_redirect_alone_is_not_a_site() {
    // The program comes from a file the host does not hold.
    assert!(found("python3 <prog.py\n").is_empty());
    assert!(found("python3 >out.txt\n").is_empty());
}

#[test]
fn an_empty_heredoc_is_a_site_with_an_empty_body() {
    let src = "python3 <<'PY'\nPY\n";
    let sites = found(src);
    let [site] = sites.as_slice() else {
        panic!("{sites:#?}");
    };
    assert!(site.delim.body.is_empty());
    assert_eq!(site.delim.close.of(src), Some("PY"));
}

#[test]
fn a_heredoc_tag_loses_its_quotes_and_backslash() {
    for (src, tag) in [
        ("python3 <<\\PY\nx\nPY\n", "PY"),
        ("python3 <<\"PY\"\nx\nPY\n", "PY"),
        ("python3 <<'P Y'\nx\nP Y\n", "P Y"),
    ] {
        let sites = found(src);
        let kind = sites.first().map(|site| site.delim.kind.clone());
        assert_eq!(
            kind,
            Some(DelimKind::Heredoc {
                tag: tag.to_owned(),
                quoted: true,
                strip_indent: false,
            }),
            "{src:?}"
        );
    }
}

#[test]
fn line_start_is_the_byte_after_the_previous_newline() {
    assert_eq!(line_start("ab\n\tcd", 4), 3);
    assert_eq!(line_start("ab\ncd", 3), 3);
    assert_eq!(line_start("abcd", 2), 0);
    assert_eq!(line_start("", 0), 0);
}

#[test]
fn strip_tabs_removes_only_leading_tabs() {
    assert_eq!(strip_tabs("\t\ta\tb\n  c\n\td"), "a\tb\n  c\nd");
    assert_eq!(strip_tabs(""), "");
}

#[test]
fn unbackslash_decodes_exactly_the_heredoc_escapes() {
    assert_eq!(
        unbackslash("\\$a \\` \\\\ \\\" \\n \\x"),
        "$a ` \\ \\\" \\n \\x"
    );
    assert_eq!(unbackslash("a \\\nb"), "a b");
    assert_eq!(unbackslash("trailing \\"), "trailing \\");
}

#[test]
fn a_hole_starts_at_its_first_non_blank_byte() {
    let src = "echo \"${a} $(b)\"\n";
    let tree = parse(src).unwrap_or_else(|e| panic!("{e}"));
    let string = tree
        .root_node()
        .named_child(0)
        .and_then(|command| command.child_by_field_name("argument"))
        .unwrap_or_else(|| panic!("no argument"));
    let mut cursor = string.walk();
    let holes: Vec<&str> = string
        .named_children(&mut cursor)
        .filter(|part| part.kind() != "string_content")
        .filter_map(|part| hole(part, src).of(src))
        .collect();
    assert_eq!(holes, ["${a}", "$(b)"]);
}

fn claims(path: &str, head: &str) -> bool {
    ShellHost.claims(Path::new(path), head)
}

#[test]
fn every_listed_extension_and_filename_is_claimed() {
    for ext in EXTENSIONS {
        assert!(claims(&format!("dir/x.{ext}"), ""), "{ext}");
    }
    for name in FILENAMES {
        assert!(claims(&format!("dir/{name}"), ""), "{name}");
    }
}

#[test]
fn bats_is_refused_before_any_shebang_is_read() {
    assert!(!claims(&format!("x.{BATS}"), "#!/bin/bash"));
    assert!(!claims(&format!("x.{BATS}"), "#!/usr/bin/env sh"));
    // bats is not in the extension list either, so the refusal is not
    // an accident of ordering.
    assert!(!EXTENSIONS.contains(&BATS));
}

#[test]
fn every_shell_dialect_shebang_but_zsh_is_claimed() {
    for dialect in ["sh", "bash", "dash", "ksh", "ash"] {
        assert!(
            claims("tool", &format!("#!/usr/bin/env {dialect}")),
            "{dialect}"
        );
        assert!(claims("tool", &format!("#!/bin/{dialect}")), "{dialect}");
    }
    // `languages/shells/shell:V310`, before the extension is looked at.
    assert!(!claims("tool", "#!/usr/bin/env zsh"));
    assert!(!claims("tool.sh", "#!/bin/zsh"));
    assert!(!claims("tool.bash", "#!/usr/local/bin/zsh -e"));
}

#[test]
fn is_zsh_reads_the_resolved_interpreter_basename() {
    let zsh = |head: &str| {
        let line = shebang::parse(head).unwrap_or_else(|| panic!("{head:?}"));
        is_zsh(&line)
    };
    assert!(zsh(&format!("#!/usr/bin/env {ZSH}")));
    assert!(zsh("#!/bin/zsh -f"));
    assert!(zsh("#!/usr/bin/env -S zsh -eu"));
    // The basename, whole: a path segment or a longer name is not zsh.
    assert!(!zsh("#!/opt/zsh/bin/bash"));
    assert!(!zsh("#!/usr/bin/env zshx"));
    assert!(!zsh("#!/usr/bin/env bash"));
    // An argument naming zsh does not make the interpreter zsh.
    assert!(!zsh("#!/usr/bin/env bash zsh"));
}

#[test]
fn a_foreign_or_missing_shebang_claims_nothing() {
    assert!(!claims("tool", "#!/usr/bin/env python3"));
    assert!(!claims("tool", "#!/usr/bin/env awk -f"));
    assert!(!claims("tool", "#!"));
    assert!(!claims("tool", "# !/bin/sh"));
    assert!(!claims("tool", ""));
}

#[test]
fn a_dotted_name_that_is_not_the_extension_is_not_claimed() {
    assert!(!claims("x.sh.txt", ""));
    assert!(!claims("x.envrc", ""));
    assert!(!claims(".envrc.local", ""));
}

#[test]
fn unoffered_operations_are_refused_by_name() {
    let refused = |operation: &'static str| Some(Error::unsupported(LangId::Shell, operation));
    let site = Site {
        sink: String::new(),
        guest: LangId::Python,
        env: GuestEnv::default(),
        delim: Delim {
            kind: DelimKind::ArgvString,
            open: Span::new(0, 1),
            body: Span::new(1, 1),
            close: Span::new(1, 2),
        },
        holes: Vec::new(),
    };
    let load = LoadRef {
        span: Span::new(0, 0),
        path: "x.py".into(),
        guest: LangId::Python,
    };
    let invoke = Invoke { argv: Vec::new() };
    assert_eq!(ShellHost.loads("").err(), refused("loads"));
    assert_eq!(ShellHost.inline("", &load, "").err(), refused("inline"));
    let rewritten = ShellHost.rewrite("", &site, &invoke, Path::new("x"));
    assert_eq!(rewritten.err(), refused("rewrite"));
}

#[test]
fn raw_appends_the_file_and_reads_no_output() {
    let cmd = raw(&["shfmt", "--diff"]);
    assert_eq!(cmd.argv, ["shfmt", "--diff"]);
    assert_eq!(cmd.format, Format::Raw);
}
