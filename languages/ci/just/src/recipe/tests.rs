//! Unit tests for reading recipes and settings off the tree (`src:C139`).

use xenolith_lang_api::{Error, LangId};

use super::{Settings, interpolations, parse, prefix, recipes, settings, text};

fn settings_of(src: &str) -> Settings {
    let tree = parse(src).unwrap_or_else(|e| panic!("{src:?}: {e}"));
    settings(&tree, src)
}

// --- parse ------------------------------------------------------------------

#[test]
fn a_file_with_a_parse_error_is_refused_whole() {
    // `languages:V78`: spans inside an error region are guesses.
    let Err(Error::Parse { lang, .. }) = parse("set shell := bash\na:\n    echo\n") else {
        panic!("a shell that is no list must not parse");
    };
    assert_eq!(lang, LangId::Just);
}

#[test]
fn an_empty_file_parses_to_no_recipe() {
    let tree = parse("").unwrap_or_else(|e| panic!("{e}"));
    assert!(recipes(&tree, "").is_empty());
}

// --- recipes ----------------------------------------------------------------

#[test]
fn every_recipe_is_read_with_its_name_lines_and_attributes() {
    let src = concat!(
        "# comment\n",
        "x := `echo not a recipe`\n",
        "build target=\"a\": dep\n",
        "    @echo {{target}}\n",
        "    -rm -f out\n",
        "\n",
        "dep:\n",
        "\n",
        "[script]\n",
        "[positional-arguments]\n",
        "py:\n",
        "    print(1)\n",
        "\n",
        "sb:\n",
        "    #!/usr/bin/env bash\n",
        "    echo hi\n",
    );
    let tree = parse(src).unwrap_or_else(|e| panic!("{e}"));
    let found = recipes(&tree, src);
    let names: Vec<&str> = found.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, ["build", "dep", "py", "sb"]);

    let [build, dep, py, sb] = found.as_slice() else {
        panic!("four recipes");
    };
    assert!(build.params && !build.script && !build.positional);
    assert_eq!(build.lines.len(), 2);
    assert_eq!(text(build.header, src), "build target=\"a\": dep");
    assert!(build.shebang.is_none());

    assert!(dep.lines.is_empty() && dep.body(src).is_none());

    assert!(py.script && py.positional && !py.params);

    assert_eq!(
        sb.shebang.map(|n| text(n, src)),
        Some("#!/usr/bin/env bash")
    );
    assert_eq!(sb.lines.len(), 1);
}

#[test]
fn a_body_span_starts_at_its_first_line_s_indent_and_ends_after_its_last() {
    let src = "a:\n    echo one\n\n    echo two\n\nb:\n  echo b\n";
    let tree = parse(src).unwrap_or_else(|e| panic!("{e}"));
    let found = recipes(&tree, src);
    let bodies: Vec<&str> = found
        .iter()
        .filter_map(|r| r.body(src).and_then(|s| s.of(src)))
        .collect();
    assert_eq!(bodies, ["    echo one\n\n    echo two", "  echo b"]);
}

#[test]
fn a_shebang_body_starts_at_the_shebang_line() {
    let src = "s:\n  #!/bin/sh\n  echo x\n";
    let tree = parse(src).unwrap_or_else(|e| panic!("{e}"));
    let found = recipes(&tree, src);
    let body = found
        .first()
        .and_then(|r| r.body(src))
        .and_then(|s| s.of(src));
    assert_eq!(body, Some("  #!/bin/sh\n  echo x"));
}

#[test]
fn prefixes_and_interpolations_come_from_the_line_nodes() {
    let src = "a x:\n    @-echo {{x}} and {{ x + \"y\" }}\n    -@true\n    plain\n";
    let tree = parse(src).unwrap_or_else(|e| panic!("{e}"));
    let found = recipes(&tree, src);
    let lines = &found.first().unwrap_or_else(|| panic!("one recipe")).lines;
    let prefixes: Vec<&str> = lines.iter().map(|l| prefix(*l, src)).collect();
    assert_eq!(prefixes, ["@-", "-@", ""]);
    let holes: Vec<&str> = lines
        .first()
        .map(|l| interpolations(*l, src))
        .unwrap_or_default()
        .iter()
        .filter_map(|s| s.of(src))
        .collect();
    assert_eq!(holes, ["{{x}}", "{{ x + \"y\" }}"]);
}

#[test]
fn four_braces_open_no_hole_whatever_the_grammar_read() {
    // `languages/ci/just:B2`: the grammar reads `{{y}}` in `x{{{{y}}` as
    // an interpolation; just prints `x{{y}}`.
    let src = "a:\n    echo x{{{{y}} {{ z }}\n    echo {{{{ w }}\n    echo {{{{{{v}}\n";
    let tree = parse(src).unwrap_or_else(|e| panic!("{e}"));
    let found = recipes(&tree, src);
    let holes: Vec<Vec<&str>> = found
        .first()
        .map(|r| r.lines.clone())
        .unwrap_or_default()
        .iter()
        .map(|l| {
            interpolations(*l, src)
                .iter()
                .filter_map(|s| s.of(src))
                .collect()
        })
        .collect();
    assert_eq!(holes.first().map(Vec::as_slice), Some(&["{{ z }}"][..]));
    assert_eq!(holes.get(1).map(Vec::len), Some(0));
    // `{{{{` then `{{v}}`: a hole, whether or not the grammar saw one.
    let third = holes.get(2).cloned().unwrap_or_default();
    assert!(third.iter().any(|h| h.starts_with("{{")), "{third:?}");
}

// --- settings (`languages/ci/just:V179`) ------------------------------------

#[test]
fn no_setting_reads_as_the_default() {
    assert_eq!(settings_of("a:\n  echo\n"), Settings::default());
}

#[test]
fn set_shell_keeps_each_string_as_written() {
    let got = settings_of("set shell := [\"bash\", '-uc']\n");
    assert_eq!(
        got.shell,
        Some(vec!["\"bash\"".to_owned(), "'-uc'".to_owned()])
    );
    assert!(!got.windows_shell && !got.positional && !got.imports);
}

#[test]
fn windows_shells_positional_arguments_and_imports_are_noted() {
    let got = settings_of(concat!(
        "set windows-shell := [\"pwsh\", \"-c\"]\n",
        "set positional-arguments\n",
        "import 'other.just'\n",
    ));
    assert!(got.shell.is_none());
    assert!(got.windows_shell && got.positional && got.imports);

    let off = settings_of("set windows-powershell := false\nset positional-arguments := false\n");
    assert!(!off.windows_shell && !off.positional);
    assert!(settings_of("set windows-powershell\n").windows_shell);
}
