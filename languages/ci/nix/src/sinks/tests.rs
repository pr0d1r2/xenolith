//! The sink matrix, function by function (`languages/ci/nix` §I,
//! `src:C139`).
//!
//! `tests/fixtures/` shows which whole files yield which sites; this file
//! holds what a fixture cannot isolate -- each helper's own branches: the
//! closed `ExecStart*` suffix set, the argument position a builder's body
//! sits at, the one context in which `text` is shell, and the dotted name
//! a report points back to (`languages/api/src/site:V43`).

use rnix::{Root, SyntaxKind, SyntaxNode, TextRange};
use xenolith_lang_api::{GuestEnv, LangId};

use super::{
    Sink, TEXT_BUILDER, apply_chain, attr_name, attr_segments, attr_sink, attr_value_sink,
    call_sink, callee_name, classify, is_builder, is_function_of_parent_apply, is_order_wrap,
    program_of, shebang_sink, shell_dialect, shell_init_sink, sink_path, sink_value, string_text,
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
    // `languages/shells/shell:V82`: the host declares, the prelude reproduces.
    assert_eq!(Sink::ServiceScript.env(), bash(&["errexit"]));
    assert_eq!(Sink::ShellHook.env(), bash(&[]));
    assert_eq!(Sink::WriteShellScript.env(), bash(&[]));
    let strict = bash(&["errexit", "nounset", "pipefail"]);
    assert_eq!(Sink::WriteShellApplication.env(), strict);
    assert_eq!(Sink::Stdenv.env(), strict);
}

#[test]
fn a_shebang_sink_declares_its_dialect_and_nothing_more() {
    // `languages/ci/nix:T157`: the interpreter line is all the host knows;
    // options a `#!/bin/sh -e` would set are not read off it.
    let sh = Sink::Shebang {
        guest: LangId::Shell,
        dialect: Some("sh"),
    };
    assert_eq!(
        sh.env(),
        GuestEnv {
            dialect: Some("sh".to_owned()),
            options: Vec::new(),
        }
    );
    let python = Sink::Shebang {
        guest: LangId::Python,
        dialect: None,
    };
    assert_eq!(python.env(), GuestEnv::default());
}

#[test]
fn a_shebang_sink_names_its_guest_and_every_other_sink_shell() {
    let python = Sink::Shebang {
        guest: LangId::Python,
        dialect: None,
    };
    assert_eq!(python.guest(), LangId::Python);
    for sink in [
        Sink::ServiceScript,
        Sink::ExecStart,
        Sink::ShellHook,
        Sink::WriteShellScript,
        Sink::WriteShellApplication,
        Sink::Stdenv,
    ] {
        assert_eq!(sink.guest(), LangId::Shell, "{sink:?}");
    }
    assert_eq!(Sink::ShellInit { dialect: "zsh" }.guest(), LangId::Shell);
}

#[test]
fn a_shell_init_sink_declares_its_program_and_no_options() {
    // `languages/ci/nix:T159`: the program's init file is sourced into its
    // interactive shell, which imposes no options on it.
    assert_eq!(
        Sink::ShellInit { dialect: "zsh" }.env(),
        GuestEnv {
            dialect: Some("zsh".to_owned()),
            options: Vec::new(),
        }
    );
    assert_eq!(Sink::ShellInit { dialect: "bash" }.env(), bash(&[]));
}

#[test]
fn an_exec_line_establishes_no_dialect() {
    // `languages/ci/nix:V69`: systemd splits an exec line itself; it is not
    // shell grammar, so nothing is declared for it.
    assert_eq!(Sink::ExecStart.env(), GuestEnv::default());
}

// --- attr_sink ---------------------------------------------------------

#[test]
fn service_attributes_are_service_scripts() {
    // `preStop` / `postStop` go through the same NixOS job script as
    // `preStart` (`languages/ci/nix:T156`).
    for name in ["script", "preStart", "postStart", "preStop", "postStop"] {
        assert_eq!(attr_sink(name), Some(Sink::ServiceScript), "{name}");
    }
}

#[test]
fn phase_hooks_are_stdenv() {
    // `languages/ci/nix:T156`: stdenv runs every `pre<Phase>` / `post<Phase>`
    // hook string through `runHook`, under the same options as a phase.
    for name in [
        "preCheck",
        "postInstall",
        "preBuild",
        "postPatch",
        "preConfigure",
        "postFixup",
        "preInstallCheck",
        "postUnpack",
    ] {
        assert_eq!(attr_sink(name), Some(Sink::Stdenv), "{name}");
    }
}

#[test]
fn a_pre_or_post_word_is_not_a_hook() {
    // A hook name continues with a capital: `preferLocalBuild` is a flag
    // and `prefix` a path. `*Phases` names LIST phases, it does not hold
    // one. The bare prefixes are nobody's hook.
    for name in [
        "pre",
        "post",
        "prefix",
        "preferLocalBuild",
        "postgresql",
        "pre_check",
        "pre1",
        "prePhases",
        "postPhases",
        "preInstallPhases",
        "PreCheck",
    ] {
        assert_eq!(attr_sink(name), None, "{name}");
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
    for name in ["text", "description", "Script", "stop", "", "shellhook"] {
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
        // Positional text a shebang may claim (`languages/ci/nix:T160`).
        "writeScript",
        "writeText",
    ] {
        assert!(is_builder(callee), "{callee}");
    }
    // `writeTextFile { text }` is an attribute value, named by its binding
    // as T157 already names it.
    for callee in ["mkDerivation", "writeTextFile", "", "writeShellApp"] {
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

// --- sink_value (`languages/ci/nix:T155`) ---------------------------------

/// A node by kind and range: each helper here parses its own tree, and
/// nodes of two trees never compare equal.
fn at(node: &SyntaxNode) -> (SyntaxKind, TextRange) {
    (node.kind(), node.text_range())
}

/// The node standing in sink position for the string `text`.
fn value_of(src: &str, text: &str) -> (SyntaxKind, TextRange) {
    at(&sink_value(&string(src, text)))
}

/// The kind of node standing in sink position for the string `text`.
fn value_kind(src: &str, text: &str) -> SyntaxKind {
    value_of(src, text).0
}

#[test]
fn a_lone_string_stands_in_its_own_sink_position() {
    let src = "{ a = ''x''; }";
    assert_eq!(value_of(src, "''x''"), at(&string(src, "''x''")));
    // Parentheses around a lone string are not climbed: `script = ("x")`
    // stays what it was before concatenation was understood.
    assert_eq!(value_kind("f (''x'')", "''x''"), SyntaxKind::NODE_STRING);
}

#[test]
fn every_operand_of_a_concatenation_stands_where_the_whole_does() {
    let src = r#"{ a = ''x'' + b + "y"; }"#;
    let whole = at(&first(src, SyntaxKind::NODE_BIN_OP));
    assert_eq!(value_of(src, "''x''"), whole);
    assert_eq!(value_of(src, r#""y""#), whole);
}

#[test]
fn parentheses_inside_and_around_a_concatenation_are_climbed() {
    let src = "f (''x'' + b)";
    assert_eq!(
        value_of(src, "''x''"),
        at(&first(src, SyntaxKind::NODE_PAREN))
    );
    // `a + (b + ''x'')`: the inner chain is itself an operand.
    let src = "{ v = a + (b + ''x''); }";
    assert_eq!(
        value_of(src, "''x''"),
        at(&first(src, SyntaxKind::NODE_BIN_OP))
    );
}

#[test]
fn only_plus_concatenates() {
    for src in [
        "{ a = ''x'' - b; }",
        "{ a = ''x'' == b; }",
        "{ a = ''x'' // b; }",
        "{ a = ''x'' ++ b; }",
    ] {
        assert_eq!(value_kind(src, "''x''"), SyntaxKind::NODE_STRING, "{src}");
    }
}

#[test]
fn a_concatenation_inside_a_hole_stops_at_the_hole() {
    let src = r#"{ a = "${''x'' + b}"; }"#;
    assert_eq!(value_kind(src, "''x''"), SyntaxKind::NODE_BIN_OP);
    assert_eq!(
        sink_value(&string(src, "''x''")).parent().map(|p| p.kind()),
        Some(SyntaxKind::NODE_INTERPOL)
    );
}

#[test]
fn every_string_operand_of_a_concatenated_sink_value_is_in_the_sink() {
    let src = r#"{ shellHook = ''a'' + x + "b"; }"#;
    assert_eq!(sink_of(src, "''a''"), Some(Sink::ShellHook));
    assert_eq!(sink_of(src, r#""b""#), Some(Sink::ShellHook));
    let src = r#"writeShellScript "n" ("a" + "b")"#;
    assert_eq!(sink_of(src, r#""n""#), None);
    assert_eq!(sink_of(src, r#""a""#), Some(Sink::WriteShellScript));
    assert_eq!(sink_of(src, r#""b""#), Some(Sink::WriteShellScript));
    let src = r#"{ ExecStartPre = [ ("a" + "b") ]; }"#;
    assert_eq!(sink_of(src, r#""b""#), Some(Sink::ExecStart));
}

#[test]
fn a_concatenation_outside_sink_position_is_data() {
    assert_eq!(sink_of("{ description = ''a'' + ''b''; }", "''a''"), None);
    assert_eq!(
        sink_of("let shellHook = ''a'' + ''b''; in shellHook", "''b''"),
        None
    );
    assert_eq!(sink_of(r#"{ shellHook = f ("a" + "b"); }"#, r#""a""#), None);
    assert_eq!(
        sink_of(r#"{ shellHook = "${"a" + "b"}"; }"#, r#""a""#),
        None
    );
    assert_eq!(sink_of(r#"{ shellHook = "a" - "b"; }"#, r#""a""#), None);
}

// --- order / priority wraps (`languages/ci/nix:T161`) ----------------------

/// Whether the string `text` in `src` is the value of an order wrap: its
/// parent asked about it.
fn wrapped(src: &str, text: &str) -> bool {
    let value = string(src, text);
    value.parent().is_some_and(|p| is_order_wrap(&p, &value))
}

#[test]
fn an_order_wrap_is_a_listed_function_at_its_arity() {
    for wrap in [
        "mkBefore",
        "lib.mkAfter",
        "mkOrder 1",
        "mkForce",
        "mkDefault",
    ] {
        assert!(wrapped(&format!("{wrap} ''x''"), "''x''"), "{wrap}");
    }
    for wrap in [
        "mkIf c",
        "mkOrder",
        "mkForce 1",
        "mkOverride 50",
        "f",
        "(x: x)",
    ] {
        assert!(!wrapped(&format!("{wrap} ''x''"), "''x''"), "{wrap}");
    }
    // Applied past its arity, or the string in function position.
    assert!(!wrapped("mkBefore ''x'' y", "''x''"));
    assert!(!wrapped("''x'' mkBefore", "''x''"));
    // Not an application at all.
    assert!(!wrapped("[ ''x'' ]", "''x''"));
}

#[test]
fn a_wrapped_value_stands_where_the_wrap_does() {
    for src in ["{ a = lib.mkBefore ''x''; }", "{ a = mkOrder 500 ''x''; }"] {
        assert_eq!(
            value_of(src, "''x''"),
            at(&first(src, SyntaxKind::NODE_APPLY)),
            "{src}"
        );
    }
    // Around a concatenation, through the parentheses it needs.
    let src = "{ a = mkAfter (''x'' + b); }";
    assert_eq!(
        value_of(src, "''x''"),
        at(&first(src, SyntaxKind::NODE_APPLY))
    );
    // Parentheses around the wrap itself are not climbed.
    let src = "f (mkForce ''x'')";
    assert_eq!(
        value_of(src, "''x''"),
        at(&nth(src, SyntaxKind::NODE_APPLY, 1))
    );
}

#[test]
fn a_named_sink_value_wrapped_in_an_order_or_priority_is_that_sinks_site() {
    for wrap in [
        "lib.mkBefore",
        "mkAfter",
        "lib.mkOrder 500",
        "lib.mkForce",
        "mkDefault",
    ] {
        let src = format!("{{ shellHook = {wrap} ''a''; }}");
        assert_eq!(sink_of(&src, "''a''"), Some(Sink::ShellHook), "{wrap}");
    }
    let src = "{ preCheck = lib.mkAfter (''a'' + x + \"b\"); }";
    assert_eq!(sink_of(src, "''a''"), Some(Sink::Stdenv));
    assert_eq!(sink_of(src, r#""b""#), Some(Sink::Stdenv));
    assert_eq!(
        sink_of(
            "{ programs.zsh.initContent = lib.mkOrder 550 ''a''; }",
            "''a''"
        ),
        Some(Sink::ShellInit { dialect: "zsh" })
    );
    assert_eq!(
        path_of("{ shellHook = lib.mkBefore ''a''; }", "''a''"),
        "shellHook"
    );
}

#[test]
fn a_wrap_counts_once_fully_applied_in_a_named_sink() {
    for src in [
        // `mkOrder`'s priority is not the body.
        "{ shellHook = lib.mkOrder ''a'' x; }",
        // Applied too far, applied to the wrapper, or wrapped twice.
        "{ shellHook = mkBefore ''a'' x; }",
        "{ shellHook = mkForce mkBefore ''a''; }",
        "{ shellHook = mkForce (mkBefore ''a''); }",
        // Not an order or priority, not a sink, not an attribute set.
        "{ shellHook = lib.mkIf c ''a''; }",
        "{ description = lib.mkForce ''a''; }",
        "let shellHook = mkBefore ''a''; in shellHook",
    ] {
        assert_eq!(sink_of(src, "''a''"), None, "{src}");
    }
    // T157 reads a whole value only, and a wrap is not one.
    let text = "\"#!/bin/sh\\na\"";
    let src = format!("{{ environment.etc.x.text = lib.mkForce {text}; }}");
    assert_eq!(sink_of(&src, text), None);
}

// --- string_text / shell_dialect / shebang_sink (`languages/ci/nix:T157`) --

#[test]
fn string_text_is_the_unescaped_body_with_each_hole_a_word() {
    let text = |src: &str| string_text(&first(src, SyntaxKind::NODE_STRING));
    assert_eq!(
        text("''\n  #!${pkgs.bash}/bin/bash\n  a ''${b}\n''"),
        "#!HOLE/bin/bash\na ${b}\n"
    );
    assert_eq!(text(r##""#!/bin/sh\n\${x}""##), "#!/bin/sh\n${x}");
    assert_eq!(text(r#""""#), "");
}

#[test]
fn shell_dialect_is_the_interpreter_when_the_api_has_it() {
    for name in ["sh", "bash", "zsh"] {
        assert_eq!(shell_dialect(name), Some(name), "{name}");
    }
    // `languages/shells/shell:V82` leaves the rest of the sh family undecided.
    for name in ["dash", "ksh", "ash", "fish", ""] {
        assert_eq!(shell_dialect(name), None, "{name:?}");
    }
}

/// The shebang sink of the first string in `src`.
fn shebang_of(src: &str) -> Option<Sink> {
    shebang_sink(&first(src, SyntaxKind::NODE_STRING))
}

/// The shebang sink of a shell interpreter of `dialect`.
fn shell_in(dialect: Option<&'static str>) -> Sink {
    Sink::Shebang {
        guest: LangId::Shell,
        dialect,
    }
}

#[test]
fn a_shebang_first_line_names_the_guest() {
    assert_eq!(
        shebang_of("''\n  #!/bin/sh\n  a\n''"),
        Some(shell_in(Some("sh")))
    );
    assert_eq!(
        shebang_of(r##""#!/usr/bin/env bash\nset -e""##),
        Some(shell_in(Some("bash")))
    );
    assert_eq!(
        shebang_of("''\n  #!${pkgs.zsh}/bin/zsh\n''"),
        Some(shell_in(Some("zsh")))
    );
    assert_eq!(shebang_of(r##""#!/bin/dash\na""##), Some(shell_in(None)));
    assert_eq!(
        shebang_of(r##""#!/usr/bin/env python3\nprint(1)""##),
        Some(Sink::Shebang {
            guest: LangId::Python,
            dialect: None,
        })
    );
}

#[test]
fn no_shebang_or_an_unknown_interpreter_is_no_sink() {
    // Not guessing (`languages/api` shebang `guest_of`): an interpreter
    // the api does not know, or one hidden in a hole, is nobody's guest.
    assert_eq!(shebang_of(r##""#!/usr/bin/env tclsh\nputs 1""##), None);
    assert_eq!(shebang_of("''\n  #!${pkgs.runtimeShell}\n  a\n''"), None);
    assert_eq!(shebang_of("''\n  a\n  #!/bin/sh\n''"), None);
    assert_eq!(shebang_of(r##""# !/bin/sh""##), None);
    assert_eq!(shebang_of(r#""""#), None);
}

#[test]
fn a_shebang_led_attribute_value_is_a_site_wherever_it_is() {
    let src = "{ environment.etc.\"xinitrc\".text = ''\n  #!/bin/sh\n  a\n''; }";
    assert_eq!(
        sink_of(src, "''\n  #!/bin/sh\n  a\n''"),
        Some(shell_in(Some("sh")))
    );
    assert_eq!(
        path_of(src, "''\n  #!/bin/sh\n  a\n''"),
        "environment.etc.xinitrc.text"
    );
    assert_eq!(
        sink_of(r##"{ a.b = "#!/bin/bash\nx"; }"##, r##""#!/bin/bash\nx""##),
        Some(shell_in(Some("bash")))
    );
}

#[test]
fn a_named_sink_outranks_the_shebang() {
    // NixOS writes its own interpreter line above a `script`: the body
    // is shell under the job script's options whatever it starts with.
    assert_eq!(
        sink_of(
            r##"{ script = "#!/usr/bin/env python\nx"; }"##,
            r##""#!/usr/bin/env python\nx""##
        ),
        Some(Sink::ServiceScript)
    );
}

#[test]
fn a_shebang_counts_only_as_a_whole_attribute_value() {
    // A variable, an operand, a list element and an argument are not an
    // attribute's value (`languages:V2`).
    let body = r##""#!/bin/sh\na""##;
    assert_eq!(sink_of(&format!("let t = {body}; in t"), body), None);
    assert_eq!(sink_of(&format!("{{ t = {body} + x; }}"), body), None);
    assert_eq!(sink_of(&format!("{{ t = [ {body} ]; }}"), body), None);
    assert_eq!(sink_of(&format!("{{ t = f {body}; }}"), body), None);
}

// --- shebang-led builder text (`languages/ci/nix:T160`) --------------------

#[test]
fn a_shebang_led_text_argument_is_a_site() {
    // T157's first-line read, for the whole text `writeScript` and
    // `writeText` write out.
    let body = r##""#!/bin/sh\na""##;
    for callee in [
        "writeScript",
        "pkgs.writeScript",
        "writeText",
        "pkgs.writeText",
    ] {
        let src = format!("{callee} \"n\" {body}");
        assert_eq!(sink_of(&src, body), Some(shell_in(Some("sh"))), "{callee}");
    }
    let python = r##""#!/usr/bin/env python3\nprint(1)""##;
    assert_eq!(
        sink_of(&format!("writeText \"n\" {python}"), python),
        Some(Sink::Shebang {
            guest: LangId::Python,
            dialect: None,
        })
    );
    assert_eq!(
        path_of(&format!("{{ p = pkgs.writeScript \"n\" {body}; }}"), body),
        "p.writeScript"
    );
}

#[test]
fn builder_text_without_a_shebang_or_out_of_place_is_no_site() {
    let body = r##""#!/bin/sh\na""##;
    // Plain text is the file it writes, a config file (`languages:V2`);
    // an interpreter the api does not know names no guest.
    assert_eq!(sink_of(r#"writeText "n" "a && b""#, r#""a && b""#), None);
    let tcl = r##""#!/usr/bin/env tclsh\nputs 1""##;
    assert_eq!(sink_of(&format!("writeScript \"n\" {tcl}"), tcl), None);
    // The NAME argument, a third argument, an operand, another builder.
    let src = format!("writeScript {body} \"b\"");
    assert_eq!(sink_of(&src, body), None);
    assert_eq!(sink_of(&format!("writeText \"n\" x {body}"), body), None);
    assert_eq!(
        sink_of(&format!("writeText \"n\" ({body} + x)"), body),
        None
    );
    assert_eq!(sink_of(&format!("writeTextDir \"n\" {body}"), body), None);
}

#[test]
fn a_named_builder_outranks_its_texts_shebang() {
    let python = r##""#!/usr/bin/env python3\nprint(1)""##;
    assert_eq!(
        sink_of(&format!("writeShellScript \"n\" {python}"), python),
        Some(Sink::WriteShellScript)
    );
}

// --- program_of / shell_init_sink (`languages/ci/nix:T159`) ---------------

/// The program the `binding`th binding of `src` (preorder) configures.
fn program(src: &str, binding: usize) -> Option<String> {
    program_of(&nth(src, SyntaxKind::NODE_ATTRPATH_VALUE, binding))
}

#[test]
fn program_of_is_the_segment_before_the_option() {
    assert_eq!(
        program("{ programs.zsh.initContent = 1; }", 0),
        Some("zsh".to_owned())
    );
    // Nested sets: the enclosing binding's last segment.
    let src = "{ programs.bash = { enable = true; initExtra = 1; }; }";
    assert_eq!(program(src, 2), Some("bash".to_owned()));
    // Through a function the set is handed to (`mkIf`).
    let src = "{ programs.zsh = lib.mkIf c { initExtra = 1; }; }";
    assert_eq!(program(src, 1), Some("zsh".to_owned()));
}

#[test]
fn program_of_a_top_level_option_is_none() {
    assert_eq!(program("{ initExtra = 1; }", 0), None);
}

#[test]
fn shell_init_options_take_their_programs_dialect() {
    let zsh = Some(Sink::ShellInit { dialect: "zsh" });
    for name in [
        "initContent",
        "initExtra",
        "initExtraFirst",
        "initExtraBeforeCompInit",
        "envExtra",
        "profileExtra",
        "loginExtra",
        "logoutExtra",
        "shellInit",
        "loginShellInit",
        "interactiveShellInit",
        "promptInit",
    ] {
        assert_eq!(shell_init_sink("zsh", name), zsh, "zsh {name}");
    }
    let bash = Some(Sink::ShellInit { dialect: "bash" });
    for name in [
        "initExtra",
        "bashrcExtra",
        "profileExtra",
        "logoutExtra",
        "shellInit",
        "loginShellInit",
        "interactiveShellInit",
        "promptInit",
    ] {
        assert_eq!(shell_init_sink("bash", name), bash, "bash {name}");
    }
}

#[test]
fn other_programs_and_options_are_not_shell_init() {
    // fish is not the shell guest; an alias table, a history setting or
    // the other program's option name is not an init file.
    assert_eq!(shell_init_sink("fish", "interactiveShellInit"), None);
    assert_eq!(shell_init_sink("zsh", "shellAliases"), None);
    assert_eq!(shell_init_sink("zsh", "bashrcExtra"), None);
    assert_eq!(shell_init_sink("bash", "initContent"), None);
    assert_eq!(shell_init_sink("bash", "historyFile"), None);
    assert_eq!(shell_init_sink("", "initExtra"), None);
}

#[test]
fn a_shell_init_option_is_a_site_under_its_program() {
    assert_eq!(
        sink_of("{ programs.zsh.initContent = ''a''; }", "''a''"),
        Some(Sink::ShellInit { dialect: "zsh" })
    );
    assert_eq!(
        sink_of(r#"{ programs.bash = { bashrcExtra = "a"; }; }"#, r#""a""#),
        Some(Sink::ShellInit { dialect: "bash" })
    );
    assert_eq!(
        sink_of(r#"{ programs.fish.initExtra = "a"; }"#, r#""a""#),
        None
    );
    assert_eq!(sink_of(r#"{ initExtra = "a"; }"#, r#""a""#), None);
    assert_eq!(
        sink_of(r#"let zsh.initExtra = "a"; in zsh"#, r#""a""#),
        None
    );
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
