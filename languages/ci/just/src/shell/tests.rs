//! Unit tests for the shell a recipe line runs under (`src:C139`,
//! `languages/ci/just:V179`).

use xenolith_lang_api::GuestEnv;

use super::{JUST_DEFAULT, LineShell, line_shell, literal, read};
use crate::recipe::Settings;

fn with(dialect: &str, options: &[&str]) -> GuestEnv {
    GuestEnv {
        dialect: Some(dialect.to_owned()),
        options: options.iter().map(|&o| o.to_owned()).collect(),
    }
}

fn shell(strings: &[&str]) -> Settings {
    Settings {
        shell: Some(strings.iter().map(|&s| s.to_owned()).collect()),
        ..Settings::default()
    }
}

fn argv(words: &[&str]) -> Vec<String> {
    words.iter().map(|&w| w.to_owned()).collect()
}

#[test]
fn no_set_shell_is_just_s_sh_cu_plus_errexit() {
    // V180: `set -eu` -- `-u` from `sh -cu`, `-e` from stopping at the
    // first failing line.
    assert_eq!(JUST_DEFAULT, ["sh", "-cu"]);
    assert_eq!(
        line_shell(&Settings::default()),
        Some(LineShell {
            env: with("sh", &["errexit", "nounset"]),
            errexit: false,
        })
    );
}

#[test]
fn a_readable_set_shell_names_the_dialect_and_its_flags() {
    let bash = line_shell(&shell(&["\"bash\"", "\"-uc\""]));
    assert_eq!(
        bash.map(|s| s.env),
        Some(with("bash", &["errexit", "nounset"]))
    );

    let strict = line_shell(&shell(&["'bash'", "'-euo'", "'pipefail'", "'-c'"]));
    assert_eq!(
        strict,
        Some(LineShell {
            env: with("bash", &["errexit", "nounset", "pipefail"]),
            errexit: true,
        })
    );

    let zsh = line_shell(&shell(&[
        "\"/bin/zsh\"",
        "\"-o\"",
        "\"ERR_EXIT\"",
        "\"-c\"",
    ]));
    assert_eq!(zsh.map(|s| s.env), Some(with("zsh", &["errexit"])));
}

#[test]
fn an_unreadable_shell_is_none_never_a_guess() {
    for strings in [
        &["\"python3\"", "\"-c\""][..],
        &["\"bash\""],
        &["\"bash\"", "\"--norc\"", "\"-c\""],
        &["\"bash\"", "\"-lc\""],
        &["\"bash\"", "\"-c\"", "\"-c\""],
        &["\"b\\ash\"", "\"-c\""],
        &["\"\"\"bash\"\"\"", "\"-c\""],
        &[],
    ] {
        assert_eq!(line_shell(&shell(strings)), None, "{strings:?}");
    }
}

#[test]
fn a_windows_shell_or_an_import_without_set_shell_is_unreadable() {
    let windows = Settings {
        windows_shell: true,
        ..Settings::default()
    };
    assert_eq!(line_shell(&windows), None);
    let imports = Settings {
        imports: true,
        ..Settings::default()
    };
    assert_eq!(line_shell(&imports), None);
    // The file's own `set shell` is still read.
    let both = Settings {
        imports: true,
        windows_shell: true,
        ..shell(&["\"sh\"", "\"-c\""])
    };
    assert_eq!(
        line_shell(&both).map(|s| s.env),
        Some(with("sh", &["errexit"]))
    );
}

#[test]
fn literals_are_plain_quoted_strings_only() {
    assert_eq!(literal("\"bash\"").as_deref(), Some("bash"));
    assert_eq!(literal("'-c'").as_deref(), Some("-c"));
    assert_eq!(literal("'a\\b'").as_deref(), Some("a\\b"));
    assert_eq!(literal("\"a\\tb\""), None);
    assert_eq!(literal("\"\"\"x\"\"\""), None);
    assert_eq!(literal("x"), None);
}

#[test]
fn read_needs_one_c_and_known_letters() {
    assert_eq!(read(&argv(&["sh", "-cu"])), Some(("sh", vec!["nounset"])));
    assert_eq!(
        read(&argv(&["bash", "-e", "-o", "pipefail", "-c"])),
        Some(("bash", vec!["errexit", "pipefail"]))
    );
    assert_eq!(read(&argv(&["zsh", "-fc"])), None, "zsh -f is NO_RCS");
    assert_eq!(read(&argv(&["sh", "-"])), None);
    assert_eq!(read(&argv(&["sh", "-o"])), None);
    assert_eq!(read(&argv(&[])), None);
}
