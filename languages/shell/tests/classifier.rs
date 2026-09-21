//! The single-command classifier (`languages/shell:V3`).
//!
//! This is the rule the whole tool turns on: a sink holding ONE simple
//! command is fine where it is, and a sink holding control flow is a
//! script that has escaped its file. Every shell sink in every host --
//! nix `script`, a hk step, a `run:` block, a just recipe -- asks this
//! one question, which is why the classifier is shared rather than
//! reimplemented per host.
//!
//! From the AST, never from substrings. `echo "a | b"` holds a pipe
//! character and is a single command; `a|b` is two. No amount of
//! substring matching separates those, and getting it wrong means either
//! a false positive on every quoted pipe or a miss on every real one.

use xenolith_lang_shell::{Construct, classify};

fn constructs(body: &str) -> Vec<&'static str> {
    let found = classify(body).unwrap_or_else(|e| panic!("{body:?} did not parse: {e}"));
    found.constructs.iter().map(|c| c.as_str()).collect()
}

fn assert_simple(body: &str) {
    let found = classify(body).unwrap_or_else(|e| panic!("{body:?} did not parse: {e}"));
    assert!(
        found.simple,
        "{body:?} should be one simple command, found {:?}",
        found
            .constructs
            .iter()
            .map(|c| c.as_str())
            .collect::<Vec<_>>()
    );
    assert!(found.constructs.is_empty());
}

fn assert_construct(body: &str, expected: &str) {
    let found = classify(body).unwrap_or_else(|e| panic!("{body:?} did not parse: {e}"));
    assert!(!found.simple, "{body:?} should not be simple");
    assert!(
        found.constructs.iter().any(|c| c.as_str() == expected),
        "{body:?} should report {expected}, found {:?}",
        found
            .constructs
            .iter()
            .map(|c| c.as_str())
            .collect::<Vec<_>>()
    );
}

#[test]
fn a_bare_command_is_simple() {
    assert_simple("echo hello");
    assert_simple("shellcheck --shell=bash file.sh");
    assert_simple("echo hello\n");
}

#[test]
fn leading_assignments_are_simple() {
    // `languages/shell:V3` allows them explicitly: `FOO=1 cmd` is how a
    // load passes a parameter (`languages/api/src/holes:V40`), so a rule
    // that rejected it would reject the tool's own output.
    assert_simple("FOO=1 echo hello");
    assert_simple("FOO=1 BAR=2 my-tool --flag arg");
}

#[test]
fn an_assignment_alone_is_simple() {
    assert_simple("FOO=1");
}

#[test]
fn nothing_at_all_is_simple() {
    // An empty sink holds no script, so there is nothing to extract and
    // nothing to report.
    assert_simple("");
    assert_simple("\n");
    assert_simple("# just a comment\n");
}

#[test]
fn a_quoted_pipe_is_not_a_pipeline() {
    assert_simple("echo 'a | b'");
    assert_simple("grep -E 'foo|bar' file");
}

#[test]
fn the_bash_source_dirname_prefix_is_whitelisted() {
    // The one command substitution `V3` permits, because it is what the
    // tool's own generated load looks like. Whitelisting it is not a
    // convenience: without it, `xnl extract` would emit a load that
    // `xnl check` then flags.
    assert_simple("bash \"$(dirname \"${BASH_SOURCE[0]}\")/helper.sh\"");
    assert_simple("FOO=1 python3 \"$(dirname \"${BASH_SOURCE[0]}\")/x.py\"");
}

#[test]
fn any_other_command_substitution_is_a_construct() {
    assert_construct("echo $(date)", "command-substitution");
    assert_construct("echo `date`", "command-substitution");
    // Same shape as the whitelisted one but a different command inside:
    // the exemption is for that exact idiom, not for `$(` in general.
    assert_construct(
        "bash \"$(find-the-script)/helper.sh\"",
        "command-substitution",
    );
}

#[test]
fn a_pipeline_is_a_construct() {
    assert_construct("cat file | grep foo", "pipeline");
}

#[test]
fn and_or_lists_are_constructs() {
    assert_construct("make && make test", "and-or");
    assert_construct("test -f x || touch x", "and-or");
}

#[test]
fn two_commands_are_a_sequence() {
    assert_construct("echo one; echo two", "sequence");
    assert_construct("echo one\necho two", "sequence");
}

#[test]
fn redirects_are_constructs() {
    assert_construct("echo hi > out.txt", "redirect");
    assert_construct("sort < in.txt", "redirect");
    assert_construct("cmd 2>&1", "redirect");
}

#[test]
fn control_flow_is_reported_by_name() {
    assert_construct("if true; then echo yes; fi", "if");
    assert_construct("for f in a b; do echo $f; done", "for");
    assert_construct("while read -r l; do echo $l; done", "while");
    assert_construct("case $x in a) echo a ;; esac", "case");
}

#[test]
fn a_heredoc_is_a_construct() {
    assert_construct("cat <<EOF\nbody\nEOF\n", "heredoc");
}

#[test]
fn a_subshell_is_a_construct() {
    assert_construct("(cd /tmp && ls)", "subshell");
}

#[test]
fn a_function_definition_is_a_construct() {
    assert_construct("f() { echo hi; }", "function-definition");
}

#[test]
fn constructs_are_sorted_and_deduplicated() {
    // Deterministic output (`src:V11`): the same body must report the
    // same list in the same order, and a body with two pipes reports
    // `pipeline` once.
    let found = constructs("a | b | c && d");
    assert_eq!(found, vec!["and-or", "pipeline"]);
}

#[test]
fn a_body_that_does_not_parse_is_an_error_not_a_verdict() {
    // `languages:V77`: an unparseable body is its own finding. Reporting
    // it as "simple" would leave broken shell inline; reporting it as
    // "has control flow" would name a construct nobody wrote.
    let Err(err) = classify("if then fi done )") else {
        panic!("garbage must not classify as anything");
    };
    assert!(err.to_string().contains("shell"), "got {err}");
}

#[test]
fn an_allowed_construct_stops_being_a_violation() {
    // `[threshold.shell] allow` (`src/config` §I) is applied by the
    // engine, so the classifier's job is to NAME constructs with the same
    // strings the config uses -- otherwise an allow entry silently
    // matches nothing.
    for name in [
        "and-or",
        "case",
        "command-substitution",
        "for",
        "function-definition",
        "heredoc",
        "if",
        "pipeline",
        "redirect",
        "sequence",
        "subshell",
        "while",
    ] {
        assert!(
            Construct::from_name(name).is_some(),
            "{name} is not a construct name the config could use"
        );
    }
}
