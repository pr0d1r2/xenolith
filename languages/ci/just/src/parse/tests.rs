//! Unit tests for `parse.rs` (`src:C139`): just's items, settings and
//! recipe headers, as GRAMMAR.md and just 1.51 define them.

use xenolith_lang_api::{Error, LangId};

use super::parse;
use crate::recipe::{Justfile, Settings};

fn parsed(src: &str) -> Justfile {
    parse(src).unwrap_or_else(|e| panic!("{src:?}: {e}"))
}

fn settings_of(src: &str) -> Settings {
    parsed(src).settings
}

fn clean(src: &str) -> bool {
    parse(src).is_ok()
}

#[test]
fn every_top_level_item_reads() {
    let src = concat!(
        "# comment\n",
        "\n",
        "alias b := build\n",
        "alias t := mod::test\n",
        "x := `echo not a recipe`\n",
        "export Y := \"y\"\n",
        "eager stamp := \"now\"\n",
        "unexport SECRET\n",
        "import 'other.just'\n",
        "import? \"maybe.just\"\n",
        "mod tools\n",
        "mod? extra 'extra.just'\n",
        "double(n) := n + n\n",
        "set dotenv-load\n",
        "set tempdir := \"/tmp\"\n",
        "[private]\n",
        "hidden := \"h\"\n",
        "build:\n",
    );
    let got = parsed(src);
    let names: Vec<&str> = got.recipes.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, ["build"]);
    assert!(got.settings.imports);
}

#[test]
fn a_recipe_header_reads_its_parameters_dependencies_and_attributes() {
    let src = concat!(
        "[script, positional-arguments]\n",
        "[group: 'ci']\n",
        "[arg(\"count\", pattern='\\d+')]\n",
        "[working-directory('sub')]\n",
        "@build $target=\"a\" mode=(\"x\" + y) *rest: dep (arg 'v' x) && post # why\n",
        "    echo\n",
    );
    let got = parsed(src);
    let [build] = got.recipes.as_slice() else {
        panic!("{:#?}", got.recipes);
    };
    assert_eq!(build.name, "build");
    assert_eq!(
        build.header.of(src),
        Some("@build $target=\"a\" mode=(\"x\" + y) *rest: dep (arg 'v' x) && post")
    );
    assert!(build.params && build.script && build.positional);
}

#[test]
fn a_recipe_with_no_parameters_and_no_body() {
    let got = parsed("a:\nb: a\n");
    let [a, b] = got.recipes.as_slice() else {
        panic!("{:#?}", got.recipes);
    };
    assert!(!a.params && a.lines.is_empty() && a.shebang.is_none());
    assert_eq!(b.header.of("a:\nb: a\n"), Some("b: a"));
}

#[test]
fn keywords_are_names_where_no_item_follows() {
    let got = parsed("set:\n    echo set\nexport:\n    echo e\nmod := \"m\"\nalias := \"a\"\n");
    let names: Vec<&str> = got.recipes.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, ["set", "export"]);
}

#[test]
fn an_empty_file_has_no_recipe_and_default_settings() {
    let got = parsed("");
    assert!(got.recipes.is_empty());
    assert_eq!(got.settings, Settings::default());
}

// --- settings (`languages/ci/just:V179`) ------------------------------------

#[test]
fn set_shell_keeps_each_string_as_written() {
    let got = settings_of("set shell := [\"bash\", '-uc',]\n");
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
    assert!(settings_of("set positional-arguments := true\n").positional);
}

#[test]
fn a_setting_just_does_not_know_still_reads() {
    assert!(clean("set future-setting := \"x\"\nset other\n"));
}

// --- what just rejects (`languages:V78`) ------------------------------------

#[test]
fn what_just_rejects_is_a_parse_error_naming_where() {
    for src in [
        "set shell := bash\na:\n    echo\n",
        "set shell := []\n",
        "set positional-arguments := maybe\n",
        "set tempdir := [\"a\"]\n",
        "  x := \"indented\"\n",
        "x := \n",
        "x := \"a\" \"b\"\n",
        "build\n",
        "a: (b\n",
        "[private\na:\n",
        "alias b c\n",
        "f(a := 1\n",
    ] {
        match parse(src) {
            Err(Error::Parse { lang, message }) => {
                assert_eq!(lang, LangId::Just);
                assert!(message.contains(':'), "{src:?}: {message}");
            }
            other => panic!("{src:?}: expected a parse error, got {other:?}"),
        }
    }
}
