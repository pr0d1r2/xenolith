//! The sink matrix, function by function (`languages/nix` §I,
//! `src:C139`).
//!
//! `tests/fixtures/` shows which whole files yield which sites; this file
//! holds what a fixture cannot isolate -- each helper's own branches: the
//! closed `ExecStart*` suffix set, the argument position a builder's body
//! sits at, the one context in which `text` is shell, and the dotted name
//! a report points back to (`languages/api/src/site:V43`).

use rnix::{Root, SyntaxKind, SyntaxNode};
use xenolith_lang_api::GuestEnv;

use super::{
    Sink, TEXT_BUILDER, apply_chain, attr_name, attr_segments, attr_sink, attr_value_sink,
    call_sink, callee_name, classify, is_builder, is_function_of_parent_apply, sink_path,
};

/// The tree for `src`, or a panic naming the parse errors. The workspace
/// denies `expect` (`src:C5`), tests included.
fn tree(src: &str) -> SyntaxNode {
    let parsed = Root::parse(src);
    let errors = parsed.errors();
    assert!(errors.is_empty(), "{src:?} did not parse: {errors:?}");
    parsed.syntax()
}

/// Every node of `kind` in `src`, in preorder.
fn nodes(src: &str, kind: SyntaxKind) -> Vec<SyntaxNode> {
    tree(src)
        .descendants()
        .filter(|n| n.kind() == kind)
        .collect()
}

/// The `i`th node of `kind` in `src` (preorder), or a panic.
fn nth(src: &str, kind: SyntaxKind, i: usize) -> SyntaxNode {
    nodes(src, kind)
        .into_iter()
        .nth(i)
        .unwrap_or_else(|| panic!("{src:?}: no {kind:?} node at {i}"))
}

/// The first node of `kind` in `src`, or a panic.
fn first(src: &str, kind: SyntaxKind) -> SyntaxNode {
    nth(src, kind, 0)
}

/// The string node whose source text is exactly `text`, quotes included.
fn string(src: &str, text: &str) -> SyntaxNode {
    nodes(src, SyntaxKind::NODE_STRING)
        .into_iter()
        .find(|n| n.text() == text)
        .unwrap_or_else(|| panic!("{src:?}: no string {text:?}"))
}

/// The sink of the string `text` in `src`.
fn sink_of(src: &str, text: &str) -> Option<Sink> {
    classify(&string(src, text))
}

/// The sink path of the string `text` in `src`.
fn path_of(src: &str, text: &str) -> String {
    sink_path(&string(src, text))
}

/// The sink of the first binding in `src`.
fn binding_sink(src: &str) -> Option<Sink> {
    attr_value_sink(&first(src, SyntaxKind::NODE_ATTRPATH_VALUE))
}

/// The callee name of the outermost application's function half.
fn callee(src: &str) -> Option<String> {
    callee_name(&first(src, SyntaxKind::NODE_APPLY).first_child()?)
}

fn bash(options: &[&str]) -> GuestEnv {
    GuestEnv {
        dialect: Some("bash".to_owned()),
        options: options.iter().map(|o| (*o).to_owned()).collect(),
    }
}

// --- Sink::env ---------------------------------------------------------

#[test]
fn env_is_what_nix_wraps_each_sink_in() {
    // `languages/shell:V82`: the host declares, the prelude reproduces.
    assert_eq!(Sink::ServiceScript.env(), bash(&["errexit"]));
    assert_eq!(Sink::ShellHook.env(), bash(&[]));
    assert_eq!(Sink::WriteShellScript.env(), bash(&[]));
    let strict = bash(&["errexit", "nounset", "pipefail"]);
    assert_eq!(Sink::WriteShellApplication.env(), strict);
    assert_eq!(Sink::Stdenv.env(), strict);
}

#[test]
fn an_exec_line_establishes_no_dialect() {
    // `languages/nix:V69`: systemd splits an exec line itself; it is not
    // shell grammar, so nothing is declared for it.
    assert_eq!(Sink::ExecStart.env(), GuestEnv::default());
}

// --- attr_sink ---------------------------------------------------------

#[test]
fn service_attributes_are_service_scripts() {
    for name in ["script", "preStart", "postStart"] {
        assert_eq!(attr_sink(name), Some(Sink::ServiceScript), "{name}");
    }
}

#[test]
fn shell_hook_is_its_own_sink() {
    assert_eq!(attr_sink("shellHook"), Some(Sink::ShellHook));
}

#[test]
fn exec_start_takes_a_closed_suffix_set() {
    for name in ["ExecStart", "ExecStartPre", "ExecStartPost"] {
        assert_eq!(attr_sink(name), Some(Sink::ExecStart), "{name}");
    }
    // Not exec lines systemd has. The `ExecStart` prefix decides alone:
    // even a `*Phase` suffix does not make `ExecStartPhase` a phase.
    for name in ["ExecStartFoo", "ExecStartpre", "ExecStartPhase", "ExecStop"] {
        assert_eq!(attr_sink(name), None, "{name}");
    }
}

#[test]
fn any_named_phase_is_stdenv_but_a_bare_phase_is_not() {
    for name in ["buildPhase", "installPhase", "checkPhase", "xPhase"] {
        assert_eq!(attr_sink(name), Some(Sink::Stdenv), "{name}");
    }
    for name in ["Phase", "phase", "buildphase", "PhaseBuild"] {
        assert_eq!(attr_sink(name), None, "{name}");
    }
}

#[test]
fn other_names_are_not_attribute_sinks() {
    // `text` is a sink only in one builder's argument set, decided by
    // `attr_value_sink`, never by the name alone.
    for name in ["text", "description", "Script", "postStop", "", "shellhook"] {
        assert_eq!(attr_sink(name), None, "{name:?}");
    }
}

// --- call_sink / is_builder --------------------------------------------

#[test]
fn write_shell_script_takes_its_body_second() {
    for callee in ["writeShellScript", "writeShellScriptBin"] {
        assert_eq!(
            call_sink(callee),
            Some((Sink::WriteShellScript, 2)),
            "{callee}"
        );
    }
}

#[test]
fn run_command_and_its_variants_take_their_body_third() {
    for callee in [
        "runCommand",
        "runCommandLocal",
        "runCommandCC",
        "runCommandNoCC",
    ] {
        assert_eq!(call_sink(callee), Some((Sink::Stdenv, 3)), "{callee}");
    }
}

#[test]
fn other_callees_have_no_positional_body() {
    // `writeShellApplication` holds its body in an argument SET, not a
    // position, and `writeScript` does not say what interprets it.
    for callee in [
        TEXT_BUILDER,
        "writeScript",
        "writeShellScriptBinX",
        "RunCommand",
        "",
    ] {
        assert_eq!(call_sink(callee), None, "{callee:?}");
    }
}

#[test]
fn builders_are_the_positional_ones_and_the_text_builder() {
    for callee in [
        "writeShellScript",
        "writeShellScriptBin",
        "runCommand",
        "runCommandLocal",
        TEXT_BUILDER,
    ] {
        assert!(is_builder(callee), "{callee}");
    }
    for callee in ["mkDerivation", "writeText", "", "writeShellApp"] {
        assert!(!is_builder(callee), "{callee:?}");
    }
}

// --- apply_chain / callee_name -----------------------------------------

#[test]
fn apply_chain_counts_curried_depth() {
    let outer = |src: &str| apply_chain(&first(src, SyntaxKind::NODE_APPLY));
    assert_eq!(outer("f a"), Some(("f".to_owned(), 1)));
    assert_eq!(outer("f a b"), Some(("f".to_owned(), 2)));
    assert_eq!(outer("f a b c"), Some(("f".to_owned(), 3)));
    assert_eq!(outer("p.f a b"), Some(("f".to_owned(), 2)));
}

#[test]
fn apply_chain_is_relative_to_the_node_asked() {
    // Preorder visits the outer application first, then its function half.
    let inner = nth("f a b", SyntaxKind::NODE_APPLY, 1);
    assert_eq!(apply_chain(&inner), Some(("f".to_owned(), 1)));
}

#[test]
fn apply_chain_has_no_name_for_an_anonymous_function() {
    assert_eq!(
        apply_chain(&first("(x: x) a", SyntaxKind::NODE_APPLY)),
        None
    );
}

#[test]
fn callee_name_reads_idents_selects_and_parens() {
    let named = |name: &str| Some(name.to_owned());
    assert_eq!(callee("writeShellScript a"), named("writeShellScript"));
    assert_eq!(callee("pkgs.writeShellScript a"), named("writeShellScript"));
    assert_eq!(callee("pkgs.lib.foo a"), named("foo"));
    assert_eq!(callee(r#"pkgs."runCommand" a"#), named("runCommand"));
    assert_eq!(
        callee("(pkgs.writeShellScript) a"),
        named("writeShellScript")
    );
    assert_eq!(callee("((f)) a"), named("f"));
}

#[test]
fn callee_name_has_nothing_for_other_expressions() {
    assert_eq!(callee("(x: x) a"), None);
    assert_eq!(callee("[ f ] a"), None);
    assert_eq!(callee(r#""f" a"#), None);
}

// --- attr_name / attr_segments -----------------------------------------

/// The attribute segments of the first binding in `src`.
fn segments(src: &str) -> Vec<String> {
    attr_segments(&first(src, SyntaxKind::NODE_ATTRPATH_VALUE))
}

#[test]
fn attr_segments_split_a_dotted_path() {
    assert_eq!(segments("{ a.b.c = 1; }"), ["a", "b", "c"]);
    assert_eq!(segments("{ a = 1; }"), ["a"]);
}

#[test]
fn a_quoted_attribute_is_named_by_its_content() {
    assert_eq!(segments(r#"{ "x y".z = 1; }"#), ["x y", "z"]);
    assert_eq!(segments(r#"{ "script" = 1; }"#), ["script"]);
}

#[test]
fn a_dynamic_attribute_is_named_by_its_source_text() {
    // No static name to give, so the reader gets what they wrote.
    assert_eq!(segments("{ ${d}.e = 1; }"), ["${d}", "e"]);
    assert_eq!(segments(r#"{ "a${b}" = 1; }"#), [r#""a${b}""#]);
}

#[test]
fn attr_name_of_an_ident_is_its_text() {
    let attr = first("{ foo = 1; }", SyntaxKind::NODE_ATTRPATH)
        .first_child()
        .unwrap_or_else(|| panic!("no attribute"));
    assert_eq!(attr_name(&attr), "foo");
}

// --- attr_value_sink ---------------------------------------------------

#[test]
fn a_binding_in_an_attribute_set_is_judged_by_its_last_segment() {
    assert_eq!(
        binding_sink("{ a.b.script = 1; }"),
        Some(Sink::ServiceScript)
    );
    assert_eq!(binding_sink("{ script.a = 1; }"), None);
    assert_eq!(
        binding_sink("rec { shellHook = 1; }"),
        Some(Sink::ShellHook)
    );
    assert_eq!(binding_sink(r#"{ "buildPhase" = 1; }"#), Some(Sink::Stdenv));
}

#[test]
fn a_let_binding_is_a_variable_not_a_sink() {
    assert_eq!(binding_sink("let script = 1; in script"), None);
}

#[test]
fn text_is_a_sink_only_as_the_text_builders_first_argument() {
    assert_eq!(
        binding_sink("writeShellApplication { text = 1; }"),
        Some(Sink::WriteShellApplication)
    );
    assert_eq!(
        binding_sink("pkgs.writeShellApplication { text = 1; }"),
        Some(Sink::WriteShellApplication)
    );
    // Another builder, a second argument, or the set in function position.
    assert_eq!(binding_sink("writeTextFile { text = 1; }"), None);
    assert_eq!(binding_sink("writeShellApplication x { text = 1; }"), None);
    assert_eq!(binding_sink("{ text = 1; } writeShellApplication"), None);
    // `environment.etc."x".text` is a config file (`languages:V2`).
    assert_eq!(binding_sink(r#"{ environment.etc."x".text = 1; }"#), None);
    assert_eq!(binding_sink("{ text = 1; }"), None);
}

// --- classify ----------------------------------------------------------

#[test]
fn a_string_bound_in_an_attribute_set_takes_the_attributes_sink() {
    assert_eq!(
        sink_of("{ systemd.services.foo.script = ''x''; }", "''x''"),
        Some(Sink::ServiceScript)
    );
    assert_eq!(
        sink_of(r#"{ ExecStart = "x"; }"#, r#""x""#),
        Some(Sink::ExecStart)
    );
    assert_eq!(sink_of(r#"{ description = "x"; }"#, r#""x""#), None);
}

#[test]
fn a_list_element_is_a_sink_only_under_an_exec_line() {
    let src = r#"{ ExecStartPre = [ "a" "b" ]; }"#;
    assert_eq!(sink_of(src, r#""a""#), Some(Sink::ExecStart));
    assert_eq!(sink_of(src, r#""b""#), Some(Sink::ExecStart));
    // A list under another sink attribute is not a list of bodies.
    assert_eq!(sink_of(r#"{ script = [ "a" ]; }"#, r#""a""#), None);
    // A list that is not a binding's value at all.
    assert_eq!(sink_of(r#"[ "a" ]"#, r#""a""#), None);
    assert_eq!(sink_of(r#"f [ "a" ]"#, r#""a""#), None);
    // A let-bound list named like an exec line is still a variable.
    assert_eq!(
        sink_of(r#"let ExecStart = [ "a" ]; in ExecStart"#, r#""a""#),
        None
    );
}

#[test]
fn a_positional_builder_argument_is_a_sink_only_at_its_position() {
    let src = r#"writeShellScript "n" "b""#;
    assert_eq!(sink_of(src, r#""n""#), None);
    assert_eq!(sink_of(src, r#""b""#), Some(Sink::WriteShellScript));

    let src = r#"pkgs.runCommand "n" "e" "b""#;
    assert_eq!(sink_of(src, r#""n""#), None);
    assert_eq!(sink_of(src, r#""e""#), None);
    assert_eq!(sink_of(src, r#""b""#), Some(Sink::Stdenv));
}

#[test]
fn an_unknown_callee_has_no_positional_sink() {
    assert_eq!(sink_of(r#"writeText "n" "b""#, r#""b""#), None);
    assert_eq!(sink_of(r#"(x: y: y) "n" "b""#, r#""b""#), None);
}

#[test]
fn a_string_in_function_position_is_not_an_argument() {
    assert_eq!(sink_of(r#""f" x"#, r#""f""#), None);
}

#[test]
fn a_string_with_no_sink_context_is_not_classified() {
    assert_eq!(sink_of(r#""x""#, r#""x""#), None);
    assert_eq!(sink_of(r#"let a = "x"; in a"#, r#""x""#), None);
    assert_eq!(sink_of(r#"{ script = ("x"); }"#, r#""x""#), None);
}

// --- is_function_of_parent_apply ---------------------------------------

#[test]
fn only_an_inner_application_is_its_parents_function() {
    let flags: Vec<bool> = nodes("f a b", SyntaxKind::NODE_APPLY)
        .iter()
        .map(is_function_of_parent_apply)
        .collect();
    assert_eq!(flags, [false, true]);
    // An application as the ARGUMENT of another is not its function half.
    assert!(
        nodes("f (g a)", SyntaxKind::NODE_APPLY)
            .iter()
            .all(|a| !is_function_of_parent_apply(a))
    );
    // The root has no parent at all.
    assert!(!is_function_of_parent_apply(&tree("f a")));
}

// --- sink_path ---------------------------------------------------------

#[test]
fn sink_path_joins_every_enclosing_binding() {
    assert_eq!(
        path_of("{ systemd.services.foo.script = ''x''; }", "''x''"),
        "systemd.services.foo.script"
    );
    assert_eq!(
        path_of("{ a = { b.c = { d = ''x''; }; }; }", "''x''"),
        "a.b.c.d"
    );
}

#[test]
fn sink_path_names_a_builder_once_however_curried() {
    assert_eq!(
        path_of(r#"{ p = pkgs.writeShellScript "n" "b"; }"#, r#""b""#),
        "p.writeShellScript"
    );
    assert_eq!(
        path_of(r#"{ p = runCommand "n" { } "b"; }"#, r#""b""#),
        "p.runCommand"
    );
}

#[test]
fn sink_path_names_the_text_builder_between_bindings() {
    assert_eq!(
        path_of(
            r#"{ packages.hello = writeShellApplication { name = "h"; text = "b"; }; }"#,
            r#""b""#
        ),
        "packages.hello.writeShellApplication.text"
    );
}

#[test]
fn sink_path_skips_applications_of_non_builders() {
    assert_eq!(
        path_of(r#"{ a = f { script = "x"; }; }"#, r#""x""#),
        "a.script"
    );
}

#[test]
fn sink_path_is_empty_with_nothing_around_it() {
    assert_eq!(path_of(r#""x""#, r#""x""#), "");
    assert_eq!(
        path_of(r#"writeShellScript "n" "b""#, r#""b""#),
        "writeShellScript"
    );
}

#[test]
fn sink_path_counts_a_let_binding_as_a_segment() {
    // The path is syntax only (`languages/api/src/site:V43`): a `let`
    // binding a string passes through is named like any other binding.
    assert_eq!(
        path_of(r#"let s = writeShellScript "n" "b"; in s"#, r#""b""#),
        "s.writeShellScript"
    );
}
