//! Unit tests for `sinks.rs` (`src:C139`): each interpreter's argv,
//! read the way that interpreter reads it.

use xenolith_lang_api::{GuestEnv, LangId};

use super::{
    INTERPRETERS, Interpreter, Kind, Letter, ShellArgv, env, interpreter, is_eval_flag,
    letter_option, option_name, program_arg, shell_argv, stdin_is_program, toggle,
};

/// `words` as the host hands them over: a plain word as `Some`, and a
/// `'…'` placeholder as `None` (quoted, so never a flag).
fn argv<'a>(words: &[&'a str]) -> Vec<Option<&'a str>> {
    words
        .iter()
        .map(|word| (!word.starts_with('\'')).then_some(*word))
        .collect()
}

fn named(word: &str) -> Interpreter<'_> {
    interpreter(word).unwrap_or_else(|| panic!("{word} is not an interpreter"))
}

fn program(word: &str, words: &[&str]) -> Option<usize> {
    program_arg(&named(word), &argv(words))
}

fn stdin(word: &str, words: &[&str]) -> bool {
    stdin_is_program(&named(word), &argv(words))
}

#[test]
fn the_interpreter_table_is_sorted_and_has_no_duplicates() {
    let names: Vec<&str> = INTERPRETERS.iter().map(|(name, _, _)| *name).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(names, sorted);
}

#[test]
fn an_interpreter_is_read_by_basename_with_its_version_stripped() {
    let python = named("/usr/bin/python3.12");
    assert_eq!(python.kind, Kind::Python);
    assert_eq!(python.guest, LangId::Python);
    assert_eq!(python.name, "python3.12");
    assert_eq!(named("bash").guest, LangId::Shell);
    assert_eq!(named("nodejs").guest, LangId::Js);
    assert_eq!(named("psql").guest, LangId::Sql);
}

#[test]
fn wrappers_and_unknown_commands_are_not_interpreters() {
    for word in [
        "env", "sudo", "exec", "command", "jq", "awk", "cat", "", "3",
    ] {
        assert_eq!(interpreter(word), None, "{word}");
    }
}

#[test]
fn a_shell_program_is_the_first_operand_after_c() {
    assert_eq!(program("bash", &["-c", "'x'"]), Some(1));
    assert_eq!(program("bash", &["-ec", "'x'", "name", "arg"]), Some(1));
    assert_eq!(program("bash", &["-c", "-e", "'x'"]), Some(2));
    assert_eq!(program("bash", &["-o", "pipefail", "-c", "'x'"]), Some(3));
    assert_eq!(program("bash", &["-eo", "pipefail", "-c", "'x'"]), Some(3));
    assert_eq!(program("sh", &["--norc", "-c", "'x'"]), Some(2));
    assert_eq!(program("zsh", &["-fc", "'x'"]), Some(1));
}

#[test]
fn a_shell_without_c_or_with_an_unknown_option_has_no_program_argument() {
    assert_eq!(program("bash", &["script.sh"]), None);
    assert_eq!(program("bash", &["-e", "script.sh"]), None);
    assert_eq!(program("bash", &["-c"]), None);
    // `--rcfile` takes a value; an unknown option ends the reading.
    assert_eq!(program("bash", &["--rcfile", "x", "-c", "'x'"]), None);
    assert_eq!(program("bash", &["-Q", "-c", "'x'"]), None);
    // `-f` is noglob in bash but not a zsh letter this reading maps,
    // and zsh's `-a` is not one either.
    assert_eq!(program("zsh", &["-ac", "'x'"]), None);
    // `-o` with nothing after it.
    assert_eq!(program("bash", &["-o"]), None);
}

#[test]
fn shell_argv_stops_at_the_end_of_options() {
    assert_eq!(
        shell_argv(false, &argv(&["-", "x"])),
        Some(ShellArgv {
            operand: Some(1),
            ..ShellArgv::default()
        })
    );
    assert_eq!(
        shell_argv(false, &argv(&["--"])),
        Some(ShellArgv::default())
    );
    assert_eq!(
        shell_argv(false, &argv(&["-s", "a"])),
        Some(ShellArgv {
            stdin: true,
            operand: Some(1),
            ..ShellArgv::default()
        })
    );
    // `+c` does not turn command mode on.
    assert_eq!(
        shell_argv(false, &argv(&["+c", "x"])),
        Some(ShellArgv {
            operand: Some(1),
            ..ShellArgv::default()
        })
    );
}

#[test]
fn shell_argv_collects_the_options_its_flags_leave_on() {
    let options =
        |zsh: bool, words: &[&str]| shell_argv(zsh, &argv(words)).map(|read| read.options);
    assert_eq!(
        options(false, &["-xeu", "-o", "pipefail", "-c", "'x'"]),
        Some(vec![
            "errexit".to_owned(),
            "nounset".to_owned(),
            "pipefail".to_owned(),
            "xtrace".to_owned(),
        ])
    );
    assert_eq!(options(false, &["-e", "+e", "-c", "'x'"]), Some(vec![]));
    assert_eq!(
        options(false, &["-o", "errexit", "+o", "errexit"]),
        Some(vec![])
    );
    assert_eq!(options(false, &["-ee"]), Some(vec!["errexit".to_owned()]));
    // Only up to the first operand: after it, flags are the program's.
    assert_eq!(options(false, &["script.sh", "-e"]), Some(vec![]));
    assert_eq!(
        options(true, &["-o", "NO_UNSET", "-e"]),
        Some(vec!["errexit".to_owned(), "nounset".to_owned()])
    );
}

#[test]
fn letters_follow_the_dialect() {
    assert_eq!(letter_option(false, 'f'), Some(Letter::Sets("noglob")));
    assert_eq!(
        letter_option(true, 'f'),
        Some(Letter::Invocation),
        "NO_RCS in zsh"
    );
    assert_eq!(letter_option(false, 'C'), Some(Letter::Sets("noclobber")));
    assert_eq!(letter_option(true, 'C'), None);
    assert_eq!(letter_option(false, 'c'), Some(Letter::Invocation));
    assert_eq!(letter_option(false, 'Q'), None);
}

#[test]
fn zsh_option_names_fold_to_the_portable_ones() {
    assert_eq!(option_name(true, "ERR_EXIT"), "errexit");
    assert_eq!(option_name(true, "pipe_fail"), "pipefail");
    assert_eq!(option_name(true, "no_unset"), "nounset");
    // Unknown: kept as written, for the guest to render.
    assert_eq!(option_name(true, "EXTENDED_GLOB"), "EXTENDED_GLOB");
    // bash names are bash's own, as written.
    assert_eq!(option_name(false, "pipefail"), "pipefail");
    assert_eq!(option_name(false, "ERR_EXIT"), "ERR_EXIT");
}

#[test]
fn toggle_keeps_each_name_once() {
    let mut options = vec!["errexit".to_owned()];
    toggle(&mut options, "errexit".to_owned(), true);
    assert_eq!(options, ["errexit"]);
    toggle(&mut options, "errexit".to_owned(), false);
    assert!(options.is_empty());
    toggle(&mut options, "xtrace".to_owned(), false);
    assert!(options.is_empty());
}

#[test]
fn a_shell_env_is_its_dialect_and_its_own_options() {
    let bash = env(&named("/bin/bash"), &argv(&["-e", "-c", "'x'"]));
    assert_eq!(bash.dialect.as_deref(), Some("bash"));
    assert_eq!(bash.options, ["errexit"]);
    // A version suffix is not part of the dialect.
    assert_eq!(env(&named("bash5"), &[]).dialect.as_deref(), Some("bash"));
    assert_eq!(env(&named("dash"), &[]).dialect.as_deref(), Some("dash"));
}

#[test]
fn a_non_shell_env_is_the_default() {
    assert_eq!(
        env(&named("python3"), &argv(&["-c", "'x'"])),
        GuestEnv::default()
    );
    assert_eq!(env(&named("psql"), &[]), GuestEnv::default());
}

#[test]
fn eval_flags_are_each_interpreters_own() {
    assert!(is_eval_flag(Kind::Python, "-c"));
    assert!(!is_eval_flag(Kind::Python, "-e"));
    assert!(!is_eval_flag(Kind::Python, "-uc"));
    assert!(is_eval_flag(Kind::Node, "--eval"));
    assert!(is_eval_flag(Kind::Node, "-e"));
    assert!(is_eval_flag(Kind::Ruby, "-ne"));
    assert!(!is_eval_flag(Kind::Ruby, "-E"));
    assert!(is_eval_flag(Kind::Perl, "-lne"));
    assert!(is_eval_flag(Kind::Perl, "-E"));
    // `-Ie` would be `-I` with the value `e`, not a bundle.
    assert!(!is_eval_flag(Kind::Perl, "-Ie"));
    assert!(!is_eval_flag(Kind::Perl, "e"));
    assert!(!is_eval_flag(Kind::Perl, "-"));
    assert!(!is_eval_flag(Kind::Shell, "-c"));
    assert!(!is_eval_flag(Kind::Psql, "-c"));
}

#[test]
fn a_flag_program_needs_only_flags_before_it_and_no_second_one() {
    assert_eq!(program("python3", &["-u", "-c", "'x'"]), Some(2));
    assert_eq!(program("perl", &["-ne", "'x'", "file"]), Some(1));
    assert_eq!(program("node", &["--eval", "'x'"]), Some(1));
    // The `-c` belongs to script.py.
    assert_eq!(program("python3", &["script.py", "-c", "'x'"]), None);
    // `-W` takes a value, which reads as an operand.
    assert_eq!(program("python3", &["-W", "ignore", "-c", "'x'"]), None);
    // A program in two pieces.
    assert_eq!(program("perl", &["-e", "'a'", "-e", "'b'"]), None);
    assert_eq!(program("python3", &["-u"]), None);
    assert_eq!(program("psql", &["-c", "'select 1'"]), None);
}

#[test]
fn stdin_is_a_shell_program_unless_an_operand_or_c_says_otherwise() {
    assert!(stdin("bash", &[]));
    assert!(stdin("bash", &["-e"]));
    assert!(stdin("bash", &["-s", "arg"]));
    assert!(stdin("sh", &["-"]));
    assert!(!stdin("bash", &["script.sh"]));
    assert!(!stdin("bash", &["-c", "'x'"]));
    assert!(!stdin("bash", &["--rcfile", "x"]));
}

#[test]
fn stdin_is_a_program_for_flag_only_or_dash_argv() {
    assert!(stdin("python3", &[]));
    assert!(stdin("python3", &["-u"]));
    assert!(stdin("python3", &["-", "arg"]));
    assert!(stdin("perl", &["-w"]));
    assert!(stdin("node", &["-"]));
    assert!(!stdin("python3", &["script.py"]));
    assert!(!stdin("python3", &["-m", "json.tool"]));
    assert!(!stdin("python3", &["-mjson.tool"]));
    assert!(!stdin("python3", &["-c", "'x'"]));
    assert!(!stdin("ruby", &["-e", "'x'"]));
    assert!(!stdin("python3", &["'quoted'"]));
}

#[test]
fn stdin_is_sql_unless_psql_was_given_a_command_file_or_listing() {
    assert!(stdin("psql", &[]));
    assert!(stdin("psql", &["'postgres://x'"]));
    assert!(stdin("psql", &["-U", "user", "db"]));
    assert!(!stdin("psql", &["-c", "'select 1'"]));
    assert!(!stdin("psql", &["-Atc", "'select 1'"]));
    assert!(!stdin("psql", &["-f", "x.sql"]));
    assert!(!stdin("psql", &["--file=x.sql"]));
    assert!(!stdin("psql", &["--command", "'x'"]));
    assert!(!stdin("psql", &["-l"]));
}

#[test]
fn tcl_interpreters_run_tcl_and_expect_is_its_dialect() {
    for word in ["tclsh", "tclsh8.6", "/usr/bin/wish", "expect"] {
        assert_eq!(named(word).guest, LangId::Tcl, "{word}");
    }
    assert_eq!(env(&named("tclsh8.6"), &[]), GuestEnv::default());
    assert_eq!(env(&named("wish"), &[]), GuestEnv::default());
    let expect = env(&named("expect"), &argv(&["-c", "'x'"]));
    assert_eq!(expect.dialect.as_deref(), Some("expect"));
    assert!(expect.options.is_empty());
}

#[test]
fn only_expect_takes_a_c_program() {
    assert_eq!(program("expect", &["-c", "'x'"]), Some(1));
    assert_eq!(program("expect", &["-c", "'x'", "login.exp"]), Some(1));
    assert_eq!(program("expect", &["-c", "'a'", "-c", "'b'"]), None);
    assert_eq!(program("expect", &["login.exp", "-c", "'x'"]), None);
    // tclsh has no `-c`: the argument is the script's own.
    assert_eq!(program("tclsh", &["-c", "'x'"]), None);
    assert_eq!(program("wish", &["-c", "'x'"]), None);
}

#[test]
fn stdin_is_a_tcl_program_with_no_script_or_one_naming_stdin() {
    for word in ["tclsh", "wish", "expect"] {
        assert!(stdin(word, &[]), "{word}");
        assert!(stdin(word, &["-", "'arg'"]), "{word}");
        assert!(stdin(word, &["/dev/stdin", "'arg'"]), "{word}");
        assert!(!stdin(word, &["script.tcl"]), "{word}");
        assert!(!stdin(word, &["'script.tcl'"]), "{word}");
    }
    assert!(!stdin("tclsh", &["-c", "'x'"]));
    assert!(!stdin("expect", &["-c", "'x'"]));
    assert!(!stdin("expect", &["-f", "login.exp"]));
    assert!(!stdin("tclsh", &["-encoding", "utf-8", "x.tcl"]));
}
