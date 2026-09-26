//! `xnl check` in the CLI: the mirror of `src/cli/check.rs`
//! (`src:C139`).
//!
//! What is pinned: which stream each part of a report lands on, the exit
//! code it makes (`src/cli:V24`), and that a config which does not load
//! is a refusal. The engine's own rules are `src/check/tests.rs`'s; the
//! end-to-end cases here are gated on the languages they need.

use std::path::PathBuf;

use xenolith_lang_api::{DelimKind, LangId};

use super::{load, render};
use crate::cli::args::OutputFormat;
use crate::discover::{Sandbox, write};
use crate::model::{Direction, Fix, Report, Rule, Violation, Warning};

fn report(violations: usize) -> Report {
    let mut report = Report::new();
    for n in 0..violations {
        report.push(Violation {
            rule: Rule::Xenolith,
            file: PathBuf::from("a.nix"),
            line: n + 1,
            col: 3,
            host: LangId::Nix,
            guest: LangId::Shell,
            sink: "s".to_owned(),
            site: DelimKind::NixIndented,
            why: "non-trivial shell: and-or".to_owned(),
            directions: vec![Direction {
                kind: Fix::Mechanical,
                action: "extract it".to_owned(),
            }],
        });
    }
    report.warn(Warning {
        code: "symlink-skipped".to_owned(),
        file: Some(PathBuf::from("l")),
        message: "l is a symlink".to_owned(),
    });
    report
}

fn rendered(report: &Report, format: OutputFormat, verbose: bool) -> (u8, String, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = render(report, format, verbose, &mut out, &mut err);
    (
        code,
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    )
}

#[test]
fn human_puts_violations_on_stdout_and_warnings_on_stderr() {
    let (code, out, err) = rendered(&report(2), OutputFormat::Human, false);
    assert_eq!(code, 1);
    assert_eq!(
        out,
        "a.nix:1:3 xenolith: shell in nix s (non-trivial shell: and-or)\n\
         a.nix:2:3 xenolith: shell in nix s (non-trivial shell: and-or)\n"
    );
    assert_eq!(err, "l: warning: symlink-skipped: l is a symlink\n");
}

#[test]
fn a_clean_report_is_exit_zero_and_silent_on_stdout() {
    let (code, out, _) = rendered(&report(0), OutputFormat::Human, false);
    assert_eq!(code, 0);
    assert!(out.is_empty(), "{out:?}");
}

#[test]
fn verbose_adds_a_count_line_last_on_stderr() {
    let (_, _, err) = rendered(&report(1), OutputFormat::Human, true);
    assert!(err.ends_with("1 violations, 1 warnings\n"), "{err:?}");
}

#[test]
fn json_is_the_envelope_on_stdout_with_warnings_inside() {
    let (code, out, err) = rendered(&report(1), OutputFormat::Json, false);
    assert_eq!(code, 1);
    assert!(err.is_empty(), "{err:?}");
    assert_eq!(out, report(1).to_json());
}

#[test]
fn no_config_file_is_the_defaults() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    assert_eq!(load(&root), Ok(crate::config::Config::default()));
}

#[test]
fn a_config_that_does_not_parse_is_refused_naming_the_key() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    write(&root, "xenolith.toml", "[langs]\nunclaimed = \"warn\"\n");
    let Err(message) = load(&root) else {
        panic!("a config without `version` is refused (src/config:V70)");
    };
    assert!(
        message.starts_with("xnl: xenolith.toml: version"),
        "{message}"
    );
}

// ---------------------------------------------------------------------
// end to end, through `run_in`
// ---------------------------------------------------------------------

fn xnl(root: &std::path::Path, args: &[&str]) -> (u8, String, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = crate::cli::run_in(root, args, &mut out, &mut err);
    (
        code,
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    )
}

#[test]
fn a_config_error_is_exit_two_on_stderr() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    write(&root, "xenolith.toml", "version = 7\n");
    let (code, out, err) = xnl(&root, &["check", "a.nix"]);
    assert_eq!(code, 2);
    assert!(out.is_empty(), "{out:?}");
    assert!(err.contains("unknown version 7"), "{err:?}");
}

#[test]
fn a_named_path_that_does_not_exist_is_exit_two() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    let (code, out, err) = xnl(&root, &["check", "missing.nix"]);
    assert_eq!(code, 2);
    assert!(out.is_empty(), "{out:?}");
    assert!(err.contains("missing.nix: no such file"), "{err:?}");
}

#[cfg(all(feature = "lang-nix", feature = "lang-shell"))]
#[test]
fn a_nix_script_is_exit_one_and_a_single_command_exit_zero() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    write(
        &root,
        "bad.nix",
        "{ systemd.services.a.script = ''\n  make && make install\n''; }\n",
    );
    write(
        &root,
        "good.nix",
        "{ systemd.services.a.script = \"make\"; }\n",
    );
    let (code, out, err) = xnl(&root, &["check", "bad.nix", "good.nix"]);
    assert_eq!(code, 1, "{err}");
    assert!(
        out.starts_with("bad.nix:1:31 xenolith: shell in nix"),
        "{out:?}"
    );
    assert_eq!(out.lines().count(), 1, "{out:?}");
    let (code, out, err) = xnl(&root, &["check", "good.nix"]);
    assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""));
    let (code, out, _) = xnl(&root, &["check", "--format", "json", "bad.nix"]);
    assert_eq!(code, 1);
    assert!(out.contains("\"rule\": \"xenolith\""), "{out}");
}
