//! Shell as a GUEST: what an extract of it looks like and how it runs
//! (`languages/shells/shell:V51`, `languages/shells/shell:V82`).
//!
//! The prelude is the load-bearing part. A body that ran under `set -e`
//! inside a nix string has to still run under it as a file, or extraction
//! changed the program while claiming to move it.

use std::path::Path;

use xenolith_lang_api::{Guest, GuestEnv, LangId};
use xenolith_lang_shell::ShellGuest;

const SHELL: ShellGuest = ShellGuest;

fn env(dialect: &str, options: &[&str]) -> GuestEnv {
    GuestEnv {
        dialect: Some(dialect.to_owned()),
        options: options.iter().map(|o| (*o).to_owned()).collect(),
    }
}

#[test]
fn shell_is_the_guest_it_says_it_is() {
    assert_eq!(SHELL.id(), LangId::Shell);
    assert!(SHELL.executable(), "a shell extract is run, not read");
}

#[test]
fn the_extension_follows_the_dialect() {
    // `languages/shells/shell:V51`: sh and bash extracts are `.sh`, zsh gets
    // `.zsh` -- a `.sh` file holding zsh syntax invites shellcheck to
    // report syntax errors that are not errors.
    assert_eq!(SHELL.extension(&env("bash", &[])), "sh");
    assert_eq!(SHELL.extension(&env("sh", &[])), "sh");
    assert_eq!(SHELL.extension(&env("zsh", &[])), "zsh");
    assert_eq!(SHELL.extension(&GuestEnv::default()), "sh");
}

#[test]
fn invoke_runs_the_file_with_its_dialect() {
    // `languages/api:V35`: the guest says how to run a file of itself,
    // and the host wraps that argv in its own syntax.
    assert_eq!(
        SHELL
            .invoke_in(Path::new("scripts/x.sh"), &env("bash", &[]))
            .argv,
        vec!["bash", "scripts/x.sh"]
    );
    assert_eq!(
        SHELL
            .invoke_in(Path::new("scripts/x.sh"), &env("sh", &[]))
            .argv,
        vec!["sh", "scripts/x.sh"]
    );
    assert_eq!(
        SHELL.invoke(Path::new("scripts/x.sh")).argv,
        vec!["bash", "scripts/x.sh"],
        "with no dialect known, bash is the default (V51)"
    );
}

#[test]
fn with_no_context_the_prelude_is_bash_and_strict() {
    // `languages/shells/shell:V51`. Nothing known about the site means the safe
    // default, not the permissive one: a script extracted without
    // `errexit` keeps going after a failed command, and the failure
    // surfaces somewhere else entirely.
    let prelude = SHELL.prelude(&GuestEnv::default());
    assert_eq!(
        prelude.shebang.map(|s| s.line()).as_deref(),
        Some("#!/usr/bin/env bash")
    );
    assert_eq!(prelude.strict.as_deref(), Some("set -euo pipefail"));
}

#[test]
fn the_prelude_reproduces_the_options_the_site_established() {
    // `languages/shells/shell:V82`: exactly the options in force, no more and no
    // fewer. Adding `nounset` to a body written without it turns an unset
    // variable from empty string into an exit.
    let prelude = SHELL.prelude(&env("bash", &["errexit", "nounset", "pipefail"]));
    assert_eq!(prelude.strict.as_deref(), Some("set -euo pipefail"));

    let prelude = SHELL.prelude(&env("bash", &["errexit"]));
    assert_eq!(prelude.strict.as_deref(), Some("set -e"));

    let prelude = SHELL.prelude(&env("bash", &["errexit", "nounset"]));
    assert_eq!(prelude.strict.as_deref(), Some("set -eu"));

    let prelude = SHELL.prelude(&env("bash", &["pipefail"]));
    assert_eq!(prelude.strict.as_deref(), Some("set -o pipefail"));

    let prelude = SHELL.prelude(&env("bash", &["errexit", "xtrace"]));
    assert_eq!(prelude.strict.as_deref(), Some("set -ex"));
}

#[test]
fn a_site_with_no_options_gets_no_strict_line() {
    // The host KNEW the dialect and reported no options, which is
    // different from knowing nothing: inventing a `set -e` here would
    // change the program.
    let prelude = SHELL.prelude(&env("bash", &[]));
    assert_eq!(
        prelude.shebang.map(|s| s.line()).as_deref(),
        Some("#!/usr/bin/env bash")
    );
    assert_eq!(prelude.strict, None);
}

#[test]
fn sh_never_gets_pipefail() {
    // `pipefail` is not POSIX. Writing it into a `#!/usr/bin/env sh`
    // file makes the file fail to run under dash, which is the shell
    // `sh` usually is on Debian.
    let prelude = SHELL.prelude(&env("sh", &["errexit", "nounset", "pipefail"]));
    assert_eq!(prelude.strict.as_deref(), Some("set -eu"));
}

#[test]
fn zsh_uses_setopt_with_zsh_option_names() {
    let prelude = SHELL.prelude(&env("zsh", &["errexit", "nounset", "pipefail"]));
    assert_eq!(
        prelude.shebang.map(|s| s.line()).as_deref(),
        Some("#!/usr/bin/env zsh")
    );
    assert_eq!(
        prelude.strict.as_deref(),
        Some("setopt err_exit no_unset pipe_fail")
    );
}

#[test]
fn an_unknown_option_is_kept_verbatim_rather_than_dropped() {
    // Dropping an option the host reported would silently change the
    // program. Keeping a name this crate does not recognise makes the
    // extract fail loudly instead, which a reader can act on.
    let prelude = SHELL.prelude(&env("bash", &["errexit", "noglob"]));
    assert_eq!(prelude.strict.as_deref(), Some("set -e -o noglob"));
}

#[test]
fn trivial_is_the_classifier() {
    // One rule, one implementation (`languages/shells/shell:V3`): the guest's
    // "may this stay inline?" and the host's "is this a script?" must
    // never be able to disagree.
    assert_eq!(SHELL.trivial("echo hello"), Ok(true));
    assert_eq!(SHELL.trivial("cat x | grep y"), Ok(false));
    assert!(SHELL.trivial("if then fi )").is_err());
}

#[test]
fn checks_follow_the_dialect() {
    // `src/lint` §I lists the defaults per dialect.
    let argv: Vec<Vec<String>> = SHELL
        .checks(&env("bash", &[]))
        .into_iter()
        .map(|cmd| cmd.argv)
        .collect();
    assert!(
        argv.contains(&vec![
            "shellcheck".to_owned(),
            "--shell=bash".to_owned(),
            "--format=json".to_owned()
        ]),
        "got {argv:?}"
    );

    let argv: Vec<Vec<String>> = SHELL
        .checks(&env("sh", &[]))
        .into_iter()
        .map(|cmd| cmd.argv)
        .collect();
    assert!(
        argv.contains(&vec![
            "shellcheck".to_owned(),
            "--shell=sh".to_owned(),
            "--format=json".to_owned()
        ]),
        "got {argv:?}"
    );
    assert!(
        argv.iter()
            .any(|a| a.first().is_some_and(|c| c == "checkbashisms")),
        "sh extracts are checked for bashisms too; got {argv:?}"
    );

    let argv: Vec<Vec<String>> = SHELL
        .checks(&env("zsh", &[]))
        .into_iter()
        .map(|cmd| cmd.argv)
        .collect();
    assert_eq!(
        argv,
        vec![vec!["zsh".to_owned(), "-n".to_owned()]],
        "shellcheck does not read zsh, so it is not offered for it"
    );
}

#[test]
fn every_check_says_where_the_file_goes_and_how_to_read_the_output() {
    for cmd in SHELL.checks(&env("bash", &[])) {
        assert!(!cmd.argv.is_empty(), "a check with no command");
    }
    let fixers: Vec<Vec<String>> = SHELL
        .fixers(&env("bash", &[]))
        .into_iter()
        .map(|cmd| cmd.argv)
        .collect();
    assert!(
        fixers
            .iter()
            .any(|a| a.first().is_some_and(|c| c == "shfmt")),
        "shfmt is the formatter, so it is the fixer; got {fixers:?}"
    );
}
