//! The linter map: the mirror of `src/lint/plan.rs` (`src:C139`).
//!
//! Pinned: where the file goes in a command, how a config command is
//! read (`src/lint` §I), and how defaults and config combine under
//! `extend` and trust (`src/lint:V8`, `src/lint:V91`).

use xenolith_lang_api::{FileArg, Format, LintCmd};

use super::{Cmd, Configured, plan};
use crate::config::LintGuest;
use crate::lint::report::Source;

fn builtin(argv: &[&str]) -> LintCmd {
    LintCmd {
        argv: argv.iter().map(|a| (*a).to_owned()).collect(),
        file_arg: FileArg::Append,
        format: Format::Raw,
    }
}

fn strings(words: &[&str]) -> Vec<String> {
    words.iter().map(|w| (*w).to_owned()).collect()
}

fn words(plan: &[Cmd]) -> Vec<String> {
    plan.iter().map(|c| c.words.join(" ")).collect()
}

#[test]
fn a_builtin_command_takes_the_file_last() {
    let cmd = Cmd::builtin(&builtin(&["shellcheck", "--shell=bash"]));
    assert_eq!(
        cmd.argv("a.sh"),
        strings(&["shellcheck", "--shell=bash", "a.sh"])
    );
    assert_eq!(cmd.check(), "shellcheck");
    assert_eq!(cmd.source, Source::Default);
}

#[test]
fn a_placeholder_puts_the_file_where_it_stands() {
    let cmd = Cmd::builtin(&LintCmd {
        argv: strings(&["jq", "-n", "-f", "{file}"]),
        file_arg: FileArg::Placeholder,
        format: Format::Raw,
    });
    assert_eq!(cmd.argv("x.jq"), strings(&["jq", "-n", "-f", "x.jq"]));
}

#[test]
fn a_config_command_is_whitespace_split_and_never_a_shell_line() {
    let cmd = Cmd::configured("  gawk   --lint -f {file} /dev/null ");
    assert_eq!(cmd.file_arg, FileArg::Placeholder);
    assert_eq!(cmd.source, Source::Config);
    assert_eq!(
        cmd.argv("a.awk"),
        strings(&["gawk", "--lint", "-f", "a.awk", "/dev/null"])
    );
    // Quotes and `;` are literal words: nothing here is a shell.
    let cmd = Cmd::configured("echo 'a b'; rm");
    assert_eq!(cmd.argv("f"), strings(&["echo", "'a", "b';", "rm", "f"]));
}

#[test]
fn a_config_command_without_a_placeholder_gets_the_file_appended() {
    let cmd = Cmd::configured("typos");
    assert_eq!(cmd.file_arg, FileArg::Append);
    assert_eq!(cmd.argv("a.sh"), strings(&["typos", "a.sh"]));
}

#[test]
fn an_empty_config_command_never_runs_the_file() {
    let cmd = Cmd::configured("   ");
    assert!(cmd.argv("a.sh").is_empty());
    assert_eq!(cmd.check(), "");
}

#[test]
fn trusted_config_extends_the_defaults_then_all_follows() {
    let entry = LintGuest {
        checks: strings(&["own"]),
        ..LintGuest::default()
    };
    let all = strings(&["everywhere"]);
    let got = plan(
        &[builtin(&["a"]), builtin(&["b"])],
        Configured::checks(Some(&entry), true, &all),
        true,
    );
    assert_eq!(words(&got.run), ["a", "b", "own", "everywhere"]);
    assert!(got.untrusted.is_empty());
}

#[test]
fn trusted_config_with_extend_false_replaces_the_defaults() {
    let entry = LintGuest {
        checks: strings(&["own"]),
        extend: Some(false),
        ..LintGuest::default()
    };
    let got = plan(
        &[builtin(&["a"])],
        Configured::checks(Some(&entry), false, &[]),
        true,
    );
    assert_eq!(words(&got.run), ["own"]);
}

#[test]
fn untrusted_config_is_held_back_and_the_defaults_run_whatever_extend_says() {
    let entry = LintGuest {
        checks: strings(&["own"]),
        extend: Some(false),
        ..LintGuest::default()
    };
    let all = strings(&["everywhere"]);
    let got = plan(
        &[builtin(&["a"])],
        Configured::checks(Some(&entry), false, &all),
        false,
    );
    assert_eq!(words(&got.run), ["a"]);
    assert_eq!(words(&got.untrusted), ["own", "everywhere"]);
}

#[test]
fn fixers_take_the_guest_list_and_never_all() {
    let entry = LintGuest {
        fixers: strings(&["fixit"]),
        ..LintGuest::default()
    };
    let got = plan(&[], Configured::fixers(Some(&entry), true), true);
    assert_eq!(words(&got.run), ["fixit"]);
}

#[test]
fn no_config_is_the_defaults_alone() {
    let got = plan(
        &[builtin(&["a"])],
        Configured::checks(None, true, &[]),
        false,
    );
    assert_eq!(words(&got.run), ["a"]);
    assert!(got.untrusted.is_empty());
}
