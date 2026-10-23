//! Unit tests for `guest.rs` (`src:C139`): the defaults of
//! `languages/data/python:V59`.

use std::path::Path;

use xenolith_lang_api::{Error, FileArg, Format, Guest, GuestEnv, LangId, Shebang};

use super::PythonGuest;

fn trivial(body: &str) -> bool {
    PythonGuest
        .trivial(body)
        .unwrap_or_else(|e| panic!("{body:?}: {e:?}"))
}

#[test]
fn the_guest_is_python() {
    assert_eq!(PythonGuest.id(), LangId::Python);
}

#[test]
fn an_extract_is_a_py_file_run_by_python3() {
    assert_eq!(PythonGuest.extension(&GuestEnv::default()), "py");
    assert_eq!(
        PythonGuest.invoke(Path::new("scripts/probe.py")).argv,
        ["python3", "scripts/probe.py"]
    );
}

#[test]
fn the_prelude_is_an_env_python3_shebang_and_no_strict_line() {
    let prelude = PythonGuest.prelude(&GuestEnv::default());
    assert_eq!(prelude.shebang, Some(Shebang::env("python3")));
    assert_eq!(prelude.strict, None);
    assert!(PythonGuest.executable());
}

#[test]
fn a_single_expression_statement_is_trivial() {
    for body in [
        "print(1)\n",
        "print(sys.argv[1].upper())",
        "  print(1)  \n",
        "# what it does\nprint(1)\n",
        "__import__('json').dumps({})\n",
    ] {
        assert!(trivial(body), "{body:?}");
    }
}

#[test]
fn more_than_one_statement_is_not_trivial() {
    for body in ["print(1)\nprint(2)\n", "print(1); print(2)\n"] {
        assert!(!trivial(body), "{body:?}");
    }
}

#[test]
fn import_def_class_and_assignment_are_not_trivial() {
    for body in [
        "import sys\n",
        "from os import path\n",
        "def f():\n    return 1\n",
        "class A:\n    pass\n",
        "@dataclass\nclass A:\n    pass\n",
        "x = 1\n",
        "x += 1\n",
        "if x:\n    print(x)\n",
        "for a in b:\n    print(a)\n",
        "pass\n",
    ] {
        assert!(!trivial(body), "{body:?}");
    }
}

#[test]
fn an_empty_body_is_not_trivial() {
    assert!(!trivial(""));
    assert!(!trivial("# only a comment\n"));
}

#[test]
fn a_body_the_grammar_rejects_is_a_parse_error() {
    match PythonGuest.trivial("print(1\n") {
        Err(Error::Parse { lang, .. }) => assert_eq!(lang, LangId::Python),
        other => panic!("expected a parse error, got {other:?}"),
    }
}

#[test]
fn the_guest_names_no_constructs_so_the_size_threshold_judges() {
    assert!(matches!(
        PythonGuest.constructs("print(1)\nprint(2)\n"),
        Err(Error::Unsupported {
            lang: LangId::Python,
            ..
        })
    ));
}

#[test]
fn checks_are_ruff_mypy_and_bandit() {
    let checks = PythonGuest.checks(&GuestEnv::default());
    let argv: Vec<Vec<&str>> = checks
        .iter()
        .map(|c| c.argv.iter().map(String::as_str).collect())
        .collect();
    assert_eq!(
        argv,
        [
            vec!["ruff", "check", "--output-format=json"],
            vec!["ruff", "format", "--check"],
            vec!["mypy"],
            vec!["bandit", "-q"],
        ]
    );
    assert_eq!(checks.first().map(|c| &c.format), Some(&Format::Json("ruff")));
    for check in checks.iter().skip(1) {
        assert_eq!(check.format, Format::Raw, "{:?}", check.argv);
    }
    assert!(checks.iter().all(|c| c.file_arg == FileArg::Append));
}

#[test]
fn fixers_are_ruff_check_fix_and_ruff_format() {
    let argv: Vec<Vec<String>> = PythonGuest
        .fixers(&GuestEnv::default())
        .into_iter()
        .map(|c| c.argv)
        .collect();
    assert_eq!(
        argv,
        [
            vec!["ruff".to_owned(), "check".to_owned(), "--fix".to_owned()],
            vec!["ruff".to_owned(), "format".to_owned()],
        ]
    );
}
