//! The shell guest's own parts (`languages/shell:V51`,
//! `languages/shell:V82`, `src:C139`).
//!
//! `tests/guest.rs` checks the trait answers a host sees. These cases go
//! under them: the dialect-to-family mapping every answer branches on,
//! and the two renderers that turn an option set into the one line that
//! decides whether extraction preserved the program -- `set_line` and
//! `zsh_setopt` -- including the inputs no current host sends yet.

use std::path::Path;

use xenolith_lang_api::{FileArg, Format, Guest, GuestEnv, LintCmd};

use super::{
    DEFAULT_STRICT, Family, SET_FLAGS, ShellGuest, ZSH_OPTIONS, dialect_name, family, raw,
    set_line, shellcheck, strict_line, zsh_setopt,
};

fn env(dialect: Option<&str>, options: &[&str]) -> GuestEnv {
    GuestEnv {
        dialect: dialect.map(str::to_owned),
        options: owned(options),
    }
}

fn owned(options: &[&str]) -> Vec<String> {
    options.iter().map(|o| (*o).to_owned()).collect()
}

fn argvs(cmds: &[LintCmd]) -> Vec<Vec<&str>> {
    cmds.iter()
        .map(|cmd| cmd.argv.iter().map(String::as_str).collect())
        .collect()
}

// --- dialect_name / family ----------------------------------------------

#[test]
fn dialect_name_is_the_hosts_word_or_bash() {
    assert_eq!(dialect_name(&env(None, &[])), "bash");
    assert_eq!(dialect_name(&env(Some("sh"), &[])), "sh");
    assert_eq!(dialect_name(&env(Some("zsh"), &[])), "zsh");
    // Passed through verbatim, not normalised: the shebang and the
    // invocation name the shell the host saw.
    assert_eq!(dialect_name(&env(Some("dash"), &[])), "dash");
    assert_eq!(dialect_name(&env(Some("Bash"), &[])), "Bash");
}

#[test]
fn family_maps_known_names_and_treats_the_rest_as_posix() {
    assert_eq!(family(&env(None, &[])), Family::Bash);
    assert_eq!(family(&env(Some("bash"), &[])), Family::Bash);
    assert_eq!(family(&env(Some("zsh"), &[])), Family::Zsh);
    assert_eq!(family(&env(Some("sh"), &[])), Family::Posix);
    // `languages/shell:V82`: dash and ksh as sh-family, and anything
    // unrecognised takes the conservative side too.
    for other in ["dash", "ksh", "mksh", "fish", "", "Bash", "ZSH"] {
        assert_eq!(family(&env(Some(other), &[])), Family::Posix, "{other:?}");
    }
}

// --- set_line -----------------------------------------------------------

#[test]
fn set_line_orders_letters_by_the_table_not_by_input() {
    // `src:V11`: the same set in any order renders the same bytes.
    let forward = set_line(&owned(&["errexit", "nounset", "xtrace"]), true);
    let backward = set_line(&owned(&["xtrace", "nounset", "errexit"]), true);
    assert_eq!(forward, "set -eux");
    assert_eq!(forward, backward);
}

#[test]
fn set_line_renders_every_known_flag() {
    for (name, flag) in SET_FLAGS {
        assert_eq!(
            set_line(&owned(&[name]), true),
            format!("set -{flag}"),
            "{name}"
        );
    }
    let all: Vec<&str> = SET_FLAGS.iter().map(|(name, _)| *name).collect();
    assert_eq!(set_line(&owned(&all), true), "set -aeuvxC");
}

#[test]
fn set_line_merges_pipefail_into_the_letter_run() {
    assert_eq!(
        set_line(&owned(&["pipefail", "nounset", "errexit"]), true),
        "set -euo pipefail"
    );
    assert_eq!(set_line(&owned(&["pipefail"]), true), "set -o pipefail");
    assert_eq!(
        set_line(&owned(&["noclobber", "pipefail"]), true),
        "set -Co pipefail"
    );
}

#[test]
fn set_line_drops_pipefail_where_unsupported() {
    assert_eq!(set_line(&owned(&["errexit", "pipefail"]), false), "set -e");
    // Dropped, not demoted to an unknown `-o pipefail`.
    assert_eq!(
        set_line(&owned(&["pipefail", "noglob"]), false),
        "set -o noglob"
    );
}

#[test]
fn set_line_keeps_unknown_options_sorted_and_deduplicated() {
    assert_eq!(
        set_line(&owned(&["noglob", "errexit", "monitor", "noglob"]), true),
        "set -e -o monitor -o noglob"
    );
    assert_eq!(
        set_line(&owned(&["errexit", "pipefail", "noglob"]), true),
        "set -eo pipefail -o noglob"
    );
}

#[test]
fn set_line_with_only_unknown_options_has_no_bare_dash() {
    assert_eq!(set_line(&owned(&["noglob"]), true), "set -o noglob");
    assert_eq!(
        set_line(&owned(&["noglob", "monitor"]), true),
        "set -o monitor -o noglob"
    );
}

#[test]
fn set_line_duplicate_known_options_render_once() {
    assert_eq!(
        set_line(
            &owned(&["errexit", "errexit", "pipefail", "pipefail"]),
            true
        ),
        "set -eo pipefail"
    );
}

// --- zsh_setopt ---------------------------------------------------------

#[test]
fn zsh_setopt_translates_every_known_option() {
    for (posix, zsh) in ZSH_OPTIONS {
        assert_eq!(
            zsh_setopt(&owned(&[posix])),
            format!("setopt {zsh}"),
            "{posix}"
        );
    }
}

#[test]
fn zsh_setopt_sorts_dedups_and_keeps_unknown_names() {
    assert_eq!(
        zsh_setopt(&owned(&["pipefail", "errexit", "nounset", "errexit"])),
        "setopt err_exit no_unset pipe_fail"
    );
    // An unknown name reaches zsh as written, where a wrong one fails
    // loudly rather than being silently lost.
    assert_eq!(
        zsh_setopt(&owned(&["extended_glob", "errexit"])),
        "setopt err_exit extended_glob"
    );
    // A zsh-native name and its posix spelling collapse to one.
    assert_eq!(
        zsh_setopt(&owned(&["err_exit", "errexit"])),
        "setopt err_exit"
    );
}

// --- strict_line --------------------------------------------------------

#[test]
fn strict_line_is_none_without_options() {
    for dialect in [None, Some("bash"), Some("sh"), Some("zsh"), Some("dash")] {
        assert_eq!(strict_line(&env(dialect, &[])), None, "{dialect:?}");
    }
}

#[test]
fn strict_line_dispatches_on_family() {
    let options = &["errexit", "nounset", "pipefail"];
    assert_eq!(
        strict_line(&env(Some("bash"), options)).as_deref(),
        Some("set -euo pipefail")
    );
    assert_eq!(
        strict_line(&env(Some("sh"), options)).as_deref(),
        Some("set -eu")
    );
    assert_eq!(
        strict_line(&env(Some("dash"), options)).as_deref(),
        Some("set -eu")
    );
    assert_eq!(
        strict_line(&env(Some("zsh"), options)).as_deref(),
        Some("setopt err_exit no_unset pipe_fail")
    );
}

#[test]
fn an_sh_site_with_only_pipefail_gets_no_strict_line() {
    // `pipefail` is dropped for sh (not POSIX), and then nothing is
    // left to set. A bare `set -` is not "nothing": bash documents it
    // as turning `-v` and `-x` OFF -- a line that looks like a mistake
    // and changes state the site never asked to change.
    assert_eq!(strict_line(&env(Some("sh"), &["pipefail"])), None);
    let prelude = ShellGuest.prelude(&env(Some("sh"), &["pipefail"]));
    assert_eq!(prelude.strict, None);
}

// --- prelude / extension / invoke ---------------------------------------

#[test]
fn prelude_without_a_dialect_ignores_options_and_takes_the_default() {
    // No dialect means the host knew nothing about the site; options
    // without a dialect are not a statement to reproduce.
    let prelude = ShellGuest.prelude(&env(None, &["xtrace"]));
    assert_eq!(prelude.strict.as_deref(), Some(DEFAULT_STRICT));
    assert_eq!(
        prelude.shebang.map(|s| s.line()).as_deref(),
        Some("#!/usr/bin/env bash")
    );
}

#[test]
fn prelude_shebang_names_the_dialect_as_written() {
    for dialect in ["sh", "bash", "zsh", "dash", "ksh"] {
        let prelude = ShellGuest.prelude(&env(Some(dialect), &["errexit"]));
        assert_eq!(
            prelude.shebang.map(|s| s.line()),
            Some(format!("#!/usr/bin/env {dialect}")),
            "{dialect}"
        );
    }
}

#[test]
fn extension_is_zsh_only_for_zsh() {
    assert_eq!(ShellGuest.extension(&env(Some("zsh"), &[])), "zsh");
    for dialect in [None, Some("sh"), Some("bash"), Some("dash"), Some("ksh")] {
        assert_eq!(
            ShellGuest.extension(&env(dialect, &[])),
            "sh",
            "{dialect:?}"
        );
    }
}

#[test]
fn invoke_in_uses_the_dialect_and_the_path_verbatim() {
    let path = Path::new("dir with space/x.zsh");
    assert_eq!(
        ShellGuest.invoke_in(path, &env(Some("zsh"), &[])).argv,
        vec!["zsh", "dir with space/x.zsh"]
    );
    assert_eq!(
        ShellGuest.invoke_in(path, &env(Some("ksh"), &[])).argv,
        vec!["ksh", "dir with space/x.zsh"]
    );
    assert_eq!(
        ShellGuest.invoke(path).argv,
        ShellGuest.invoke_in(path, &GuestEnv::default()).argv
    );
}

#[test]
fn trivial_passes_the_classifier_error_through() {
    assert_eq!(ShellGuest.trivial(""), Ok(true));
    assert_eq!(ShellGuest.trivial("a; b"), Ok(false));
    assert!(ShellGuest.trivial("a |").is_err());
}

// --- checks / fixers / the LintCmd builders -----------------------------

#[test]
fn shellcheck_is_json_appended_per_dialect() {
    let cmd = shellcheck("sh");
    assert_eq!(cmd.argv, vec!["shellcheck", "--shell=sh", "--format=json"]);
    assert_eq!(cmd.file_arg, FileArg::Append);
    assert_eq!(cmd.format, Format::Json("shellcheck"));
}

#[test]
fn raw_copies_argv_and_appends_the_file() {
    let cmd = raw(&["zsh", "-n"]);
    assert_eq!(cmd.argv, vec!["zsh", "-n"]);
    assert_eq!(cmd.file_arg, FileArg::Append);
    assert_eq!(cmd.format, Format::Raw);
    assert_eq!(raw(&[]).argv, Vec::<String>::new());
}

#[test]
fn checks_per_family_in_order() {
    assert_eq!(
        argvs(&ShellGuest.checks(&env(Some("bash"), &[]))),
        vec![
            vec!["shellcheck", "--shell=bash", "--format=json"],
            vec!["shfmt", "--diff", "--language-dialect", "bash"],
        ]
    );
    // No dialect is bash.
    assert_eq!(
        ShellGuest.checks(&env(None, &[])),
        ShellGuest.checks(&env(Some("bash"), &[]))
    );
    assert_eq!(
        argvs(&ShellGuest.checks(&env(Some("dash"), &[]))),
        vec![
            vec!["shellcheck", "--shell=sh", "--format=json"],
            vec!["checkbashisms"],
            vec!["shfmt", "--diff", "--language-dialect", "posix"],
        ]
    );
    assert_eq!(
        argvs(&ShellGuest.checks(&env(Some("zsh"), &[]))),
        vec![vec!["zsh", "-n"]]
    );
}

#[test]
fn fixers_per_family() {
    assert_eq!(
        argvs(&ShellGuest.fixers(&env(Some("bash"), &[]))),
        vec![vec!["shfmt", "--write", "--language-dialect", "bash"]]
    );
    assert_eq!(
        argvs(&ShellGuest.fixers(&env(Some("sh"), &[]))),
        vec![vec!["shfmt", "--write", "--language-dialect", "posix"]]
    );
    assert!(ShellGuest.fixers(&env(Some("zsh"), &[])).is_empty());
    for cmd in ShellGuest.fixers(&env(None, &[])) {
        assert_eq!(cmd.format, Format::Raw);
        assert_eq!(cmd.file_arg, FileArg::Append);
    }
}
