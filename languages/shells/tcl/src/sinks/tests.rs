//! Unit tests for `sinks.rs` (`src:C139`): the argv readings, word by
//! word, with `None` standing for a braced, quoted or substituted word.

use xenolith_lang_api::{GuestEnv, LangId};

use super::{Interpreter, Program, Runner, interpreter, option_name, program, shell_argv};

fn read(runner: Runner, name: &str, args: &[Option<&str>]) -> Option<Program> {
    program(runner, interpreter(name), args)
}

fn env(dialect: &str, options: &[&str]) -> GuestEnv {
    GuestEnv {
        dialect: Some(dialect.to_owned()),
        options: options.iter().map(|o| (*o).to_owned()).collect(),
    }
}

#[test]
fn runners_are_exec_and_spawn_only() {
    assert_eq!(Runner::of("exec"), Some(Runner::Exec));
    assert_eq!(Runner::of("spawn"), Some(Runner::Spawn));
    for other in ["open", "eval", "send", "Exec"] {
        assert_eq!(Runner::of(other), None, "{other}");
    }
    assert_eq!(Runner::Exec.as_str(), "exec");
    assert_eq!(Runner::Spawn.as_str(), "spawn");
}

#[test]
fn an_interpreter_is_read_by_its_basename_version_stripped() {
    assert_eq!(
        interpreter("/usr/bin/python3"),
        Interpreter {
            name: "python3",
            bare: "python"
        }
    );
    assert_eq!(interpreter("tclsh8.6").bare, "tclsh");
    assert_eq!(interpreter("sh").name, "sh");
}

#[test]
fn a_shell_c_word_is_the_program_with_the_dialect_named() {
    let found = read(Runner::Exec, "sh", &[Some("-c"), None]);
    assert_eq!(
        found,
        Some(Program {
            at: 1,
            guest: LangId::Shell,
            env: env("sh", &[]),
            how: "-c",
        })
    );
    let spawned = read(Runner::Spawn, "bash", &[Some("-c"), None]);
    assert_eq!(spawned.map(|p| p.env), Some(env("bash", &[])));
}

#[test]
fn every_listed_shell_reads_its_c_word() {
    for shell in ["sh", "bash", "zsh", "dash", "/bin/sh"] {
        let found = read(Runner::Exec, shell, &[Some("-c"), None]);
        assert_eq!(found.map(|p| p.guest), Some(LangId::Shell), "{shell}");
    }
}

#[test]
fn a_plain_word_after_c_is_the_program_too() {
    let found = read(Runner::Exec, "sh", &[Some("-c"), Some("ls")]);
    assert_eq!(found.map(|p| p.at), Some(1));
}

#[test]
fn flags_set_options_from_the_argv_itself() {
    let found = read(Runner::Exec, "bash", &[Some("-euc"), None]);
    assert_eq!(
        found.map(|p| p.env),
        Some(env("bash", &["errexit", "nounset"]))
    );
    let found = read(
        Runner::Exec,
        "bash",
        &[Some("-o"), Some("pipefail"), Some("-c"), None],
    );
    assert_eq!(found.map(|p| p.env), Some(env("bash", &["pipefail"])));
}

#[test]
fn zsh_option_names_fold_to_the_portable_ones() {
    let found = read(
        Runner::Exec,
        "zsh",
        &[Some("-o"), Some("ERR_EXIT"), Some("-c"), None],
    );
    assert_eq!(found.map(|p| p.env), Some(env("zsh", &["errexit"])));
    assert_eq!(option_name(true, "extended_glob"), "extended_glob");
    assert_eq!(option_name(false, "ERR_EXIT"), "ERR_EXIT");
}

#[test]
fn zsh_f_is_an_invocation_letter_and_sets_nothing() {
    let found = read(Runner::Exec, "zsh", &[Some("-fc"), None]);
    assert_eq!(found.map(|p| p.env), Some(env("zsh", &[])));
    let found = read(Runner::Exec, "sh", &[Some("-fc"), None]);
    assert_eq!(found.map(|p| p.env), Some(env("sh", &["noglob"])));
}

#[test]
fn a_shell_without_c_has_no_program_word() {
    assert_eq!(read(Runner::Exec, "sh", &[Some("script.sh")]), None);
    assert_eq!(read(Runner::Exec, "bash", &[None]), None);
    assert_eq!(read(Runner::Exec, "sh", &[]), None);
    // `-c` with nothing after it.
    assert_eq!(read(Runner::Exec, "sh", &[Some("-c")]), None);
}

#[test]
fn an_unknown_flag_ends_the_reading() {
    assert_eq!(
        read(
            Runner::Exec,
            "bash",
            &[Some("--rcfile"), Some("x"), Some("-c"), None]
        ),
        None
    );
    assert_eq!(
        read(
            Runner::Exec,
            "bash",
            &[Some("-O"), Some("globstar"), Some("-c"), None]
        ),
        None
    );
}

#[test]
fn a_known_long_option_is_skipped() {
    let found = read(Runner::Exec, "bash", &[Some("--norc"), Some("-c"), None]);
    assert_eq!(found.map(|p| p.at), Some(2));
}

#[test]
fn double_dash_makes_the_next_word_the_operand() {
    let found = read(Runner::Exec, "sh", &[Some("-c"), Some("--"), None]);
    assert_eq!(found.map(|p| p.at), Some(2));
    assert_eq!(read(Runner::Exec, "sh", &[Some("-c"), Some("--")]), None);
}

#[test]
fn a_non_shell_interpreter_has_no_c_program() {
    for name in ["ssh", "python3", "ls", "tclsh", "-ignorestderr"] {
        assert_eq!(
            read(Runner::Exec, name, &[Some("-c"), None]),
            None,
            "{name}"
        );
    }
}

#[test]
fn a_feed_after_the_interpreter_is_its_program() {
    for (name, guest) in [
        ("python3", LangId::Python),
        ("ruby", LangId::Ruby),
        ("perl", LangId::Perl),
        ("node", LangId::Js),
        ("psql", LangId::Sql),
        ("tclsh", LangId::Tcl),
        ("wish", LangId::Tcl),
        ("sh", LangId::Shell),
    ] {
        let found = read(Runner::Exec, name, &[Some("<<"), None]);
        assert_eq!(found.as_ref().map(|p| p.guest), Some(guest), "{name}");
        assert_eq!(
            found.as_ref().map(|p| (p.at, p.how)),
            Some((1, "<<")),
            "{name}"
        );
    }
}

#[test]
fn a_fed_program_names_only_the_dialects_it_knows() {
    let expect = read(Runner::Exec, "expect", &[Some("<<"), None]);
    assert_eq!(expect.map(|p| p.env), Some(env("expect", &[])));
    let tclsh = read(Runner::Exec, "tclsh", &[Some("<<"), None]);
    assert_eq!(tclsh.map(|p| p.env), Some(GuestEnv::default()));
    let bash = read(Runner::Exec, "bash", &[Some("<<"), None]);
    assert_eq!(bash.map(|p| p.env), Some(env("bash", &[])));
    let python = read(Runner::Exec, "python3", &[Some("<<"), None]);
    assert_eq!(python.map(|p| p.env), Some(GuestEnv::default()));
}

#[test]
fn a_feed_to_a_data_reader_is_no_program() {
    // `awk` and `jq` read DATA on stdin (`languages/shells/shell:V139`).
    for name in ["awk", "jq", "cat", "ssh"] {
        assert_eq!(
            read(Runner::Exec, name, &[Some("<<"), None]),
            None,
            "{name}"
        );
    }
}

#[test]
fn a_feed_not_straight_after_the_interpreter_is_no_program() {
    assert_eq!(
        read(Runner::Exec, "python3", &[Some("-u"), Some("<<"), None]),
        None
    );
    assert_eq!(read(Runner::Exec, "python3", &[Some("<<")]), None);
}

#[test]
fn a_second_stdin_after_the_feed_is_no_program() {
    for again in ["<", "<<", "<@", "<file"] {
        let args = [Some("<<"), None, Some(again), Some("x")];
        assert_eq!(read(Runner::Exec, "python3", &args), None, "{again}");
    }
    // Output redirections and a pipeline leave stdin alone.
    let args = [Some("<<"), None, Some("2>@1"), Some("|"), Some("wc")];
    assert!(read(Runner::Exec, "python3", &args).is_some());
}

#[test]
fn spawn_has_no_feed() {
    assert_eq!(read(Runner::Spawn, "python3", &[Some("<<"), None]), None);
    assert_eq!(read(Runner::Spawn, "sh", &[Some("<<"), None]), None);
}

#[test]
fn shell_argv_stops_at_the_first_operand() {
    let read = shell_argv(false, &[Some("-e"), Some("x.sh"), Some("-c")]);
    assert_eq!(read.map(|r| (r.command, r.operand)), Some((false, Some(1))));
    let plus = shell_argv(false, &[Some("-e"), Some("+e"), Some("-c"), None]);
    assert_eq!(plus.map(|r| r.options), Some(Vec::new()));
}
