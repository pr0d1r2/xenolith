//! Running one command: the mirror of `src/lint/run.rs` (`src:C139`).
//!
//! Every tool here is a stub script on a `PATH` the test controls
//! (`tests:V150`), so no case depends on which linters the machine has.

use std::time::{Duration, Instant};

use super::{TAIL_LINES, Tools, absent, run, tail};
use crate::discover::Sandbox;
use crate::lint::report::Status;
use crate::lint::tests::stub;

fn argv(words: &[&str]) -> Vec<String> {
    words.iter().map(|w| (*w).to_owned()).collect()
}

#[test]
fn exit_zero_is_a_pass_with_no_tail() {
    let sandbox = Sandbox::new();
    let bin = sandbox.plain("bin");
    stub(&bin, "ok", "echo fine\nexit 0");
    let ran = run(
        sandbox.path(),
        &argv(&["ok", "f"]),
        None,
        &Tools::on_path(&bin),
    );
    assert_eq!(ran.status, Status::Pass);
    assert_eq!(ran.exit, Some(0));
    assert_eq!(ran.tail, None);
}

#[test]
fn a_non_zero_exit_is_a_fail_keeping_what_the_tool_printed() {
    let sandbox = Sandbox::new();
    let bin = sandbox.plain("bin");
    stub(&bin, "bad", "echo \"out $1\"\necho err >&2\nexit 3");
    let ran = run(
        sandbox.path(),
        &argv(&["bad", "f.sh"]),
        None,
        &Tools::on_path(&bin),
    );
    assert_eq!(ran.status, Status::Fail);
    assert_eq!(ran.exit, Some(3));
    assert_eq!(ran.tail.as_deref(), Some("out f.sh\nerr"));
}

#[test]
fn a_silent_failure_still_says_it_failed() {
    let sandbox = Sandbox::new();
    let bin = sandbox.plain("bin");
    stub(&bin, "quiet", "exit 1");
    let ran = run(
        sandbox.path(),
        &argv(&["quiet"]),
        None,
        &Tools::on_path(&bin),
    );
    assert_eq!(ran.status, Status::Fail);
    assert!(ran.tail.is_some_and(|t| t.contains("exited 1")));
}

#[test]
fn a_tool_not_on_path_is_an_error_naming_it_and_how_to_get_it() {
    // `src/lint:V8`: never a silent skip.
    let sandbox = Sandbox::new();
    let bin = sandbox.plain("bin");
    let ran = run(
        sandbox.path(),
        &argv(&["nosuchlinter", "f"]),
        None,
        &Tools::on_path(&bin),
    );
    assert_eq!(ran.status, Status::Error);
    assert_eq!(ran.exit, None);
    assert_eq!(ran.tail, Some(absent("nosuchlinter")));
    let why = absent("nosuchlinter");
    assert!(why.contains("`nosuchlinter`"), "{why}");
    assert!(why.contains("install"), "{why}");
    assert!(why.contains("nix"), "{why}");
}

#[test]
fn an_empty_command_is_an_error_not_a_pass() {
    let sandbox = Sandbox::new();
    let ran = run(sandbox.path(), &[], None, &Tools::inherit());
    assert_eq!(ran.status, Status::Error);
}

#[test]
fn the_tool_runs_in_the_root() {
    let sandbox = Sandbox::new();
    let bin = sandbox.plain("bin");
    let root = sandbox.plain("tree");
    crate::discover::write(&root, "here.txt", "x");
    stub(&bin, "look", "test -f here.txt");
    let ran = run(&root, &argv(&["look"]), None, &Tools::on_path(&bin));
    assert_eq!(ran.status, Status::Pass, "{:?}", ran.tail);
}

#[test]
fn the_tail_keeps_the_last_lines_only() {
    let text = (1..=TAIL_LINES + 5)
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let kept = tail(&text).unwrap_or_default();
    assert_eq!(kept.lines().count(), TAIL_LINES);
    assert!(kept.starts_with("6\n"), "{kept}");
    assert!(kept.ends_with(&format!("{}", TAIL_LINES + 5)), "{kept}");
    assert_eq!(tail("  \n\n"), None);
}

#[test]
fn a_tool_past_its_limit_is_killed_and_is_an_error_naming_it_and_the_limit() {
    // `src/lint:V126`: a gate that hangs is bypassed next commit.
    let sandbox = Sandbox::new();
    let bin = sandbox.plain("bin");
    stub(&bin, "sleeper", "exec /bin/sleep 30");
    let started = Instant::now();
    let ran = run(
        sandbox.path(),
        &argv(&["sleeper", "f"]),
        Some(Duration::from_secs(1)),
        &Tools::on_path(&bin),
    );
    assert!(
        started.elapsed() < Duration::from_secs(20),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(ran.status, Status::Error);
    assert_eq!(ran.exit, None);
    let why = ran.tail.unwrap_or_default();
    assert!(why.contains("`sleeper`") && why.contains("1s"), "{why}");
}

#[test]
fn a_tool_within_its_limit_is_judged_as_usual() {
    let sandbox = Sandbox::new();
    let bin = sandbox.plain("bin");
    stub(&bin, "quick", "exit 1");
    let ran = run(
        sandbox.path(),
        &argv(&["quick"]),
        Some(Duration::from_secs(30)),
        &Tools::on_path(&bin),
    );
    assert_eq!(ran.status, Status::Fail);
    assert_eq!(ran.exit, Some(1));
}
