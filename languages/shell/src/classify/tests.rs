//! The classifier's own parts (`languages/shell:V3`, `src:C139`).
//!
//! `tests/classifier.rs` asks the public question -- is this body one
//! simple command? -- over whole scripts. These cases go one level down:
//! the name table the config matches against, the node-kind table the
//! walk consults, the top-level statement count that alone decides
//! `sequence`, and the walk that must see constructs however deep they
//! sit. A wrong answer in any of them surfaces upstream only as "this
//! script was called simple", which says nothing about which part lied.

use tree_sitter::{Node, Parser, Tree};

use xenolith_lang_api::GuestEnv;

use super::{
    ALLOWED_SUBSTITUTION, Classification, Construct, classify, classify_in, collect, construct_of,
    top_level_statements,
};

fn tree(body: &str) -> Tree {
    let mut parser = Parser::new();
    if let Err(e) = parser.set_language(&tree_sitter_bash::LANGUAGE.into()) {
        panic!("the bash grammar did not load: {e}");
    }
    match parser.parse(body, None) {
        Some(tree) => tree,
        None => panic!("the bash parser returned no tree for {body:?}"),
    }
}

/// The first node of `kind` in document order.
fn find<'t>(node: Node<'t>, kind: &str) -> Option<Node<'t>> {
    if node.kind() == kind {
        return Some(node);
    }
    let mut cursor = node.walk();
    let children: Vec<Node<'t>> = node.named_children(&mut cursor).collect();
    children.into_iter().find_map(|child| find(child, kind))
}

/// `construct_of` applied to the first `kind` node of `body`.
fn construct_of_first(body: &str, kind: &str) -> Option<Construct> {
    let tree = tree(body);
    match find(tree.root_node(), kind) {
        Some(node) => construct_of(node, body),
        None => panic!("no {kind} node in {body:?}: {}", tree.root_node().to_sexp()),
    }
}

fn ok(body: &str) -> Classification {
    match classify(body) {
        Ok(found) => found,
        Err(err) => panic!("expected a classification of {body:?}, got: {err:?}"),
    }
}

fn collected(body: &str) -> Vec<Construct> {
    let tree = tree(body);
    let mut found = Vec::new();
    collect(tree.root_node(), body, &mut found);
    found
}

// --- Construct: the names `[threshold.shell] allow` matches -------------

#[test]
fn all_lists_every_construct_once_in_name_order() {
    // Declaration order is name order (the doc comment's promise), so
    // `ALL` sorted by the derived `Ord` and sorted by `as_str` agree.
    let mut by_ord = Construct::ALL.to_vec();
    by_ord.sort_unstable();
    by_ord.dedup();
    assert_eq!(by_ord, Construct::ALL.to_vec());

    let names: Vec<&str> = Construct::ALL.iter().map(|c| c.as_str()).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted, "ALL is not in name order");
    assert_eq!(Construct::ALL.len(), 12);
}

#[test]
fn every_name_round_trips_through_from_name() {
    for construct in Construct::ALL {
        assert_eq!(
            Construct::from_name(construct.as_str()),
            Some(*construct),
            "{construct:?}"
        );
    }
}

#[test]
fn the_config_facing_names_are_a_contract() {
    // Renaming one turns every allow entry naming it into a silent no-op
    // (`src/config` §I), so the strings are pinned, not derived.
    let pinned = [
        (Construct::AndOr, "and-or"),
        (Construct::Case, "case"),
        (Construct::CommandSubstitution, "command-substitution"),
        (Construct::For, "for"),
        (Construct::FunctionDefinition, "function-definition"),
        (Construct::Heredoc, "heredoc"),
        (Construct::If, "if"),
        (Construct::Pipeline, "pipeline"),
        (Construct::Redirect, "redirect"),
        (Construct::Sequence, "sequence"),
        (Construct::Subshell, "subshell"),
        (Construct::While, "while"),
    ];
    for (construct, name) in pinned {
        assert_eq!(construct.as_str(), name);
    }
}

#[test]
fn from_name_is_exact_match_only() {
    // A misspelled allow entry must be an error upstream, not a rule
    // that happens to match something close.
    for near_miss in [
        "",
        "Pipeline",
        "PIPELINE",
        " pipeline",
        "pipeline ",
        "pipe",
        "pipelines",
        "and_or",
        "andor",
        "function",
        "command_substitution",
        "until",
    ] {
        assert_eq!(Construct::from_name(near_miss), None, "{near_miss:?}");
    }
}

// --- construct_of: the node-kind table ----------------------------------

#[test]
fn each_node_kind_maps_to_its_construct() {
    let cases = [
        ("a | b", "pipeline", Construct::Pipeline),
        ("a && b", "list", Construct::AndOr),
        ("a || b", "list", Construct::AndOr),
        ("(a)", "subshell", Construct::Subshell),
        ("if a; then b; fi", "if_statement", Construct::If),
        ("for x in a; do b; done", "for_statement", Construct::For),
        (
            "for ((i=0;i<3;i++)); do b; done",
            "c_style_for_statement",
            Construct::For,
        ),
        ("while a; do b; done", "while_statement", Construct::While),
        ("until a; do b; done", "while_statement", Construct::While),
        ("case x in a) b;; esac", "case_statement", Construct::Case),
        (
            "f() { a; }",
            "function_definition",
            Construct::FunctionDefinition,
        ),
        (
            "function f { a; }",
            "function_definition",
            Construct::FunctionDefinition,
        ),
        (
            "cat <<EOF\nx\nEOF\n",
            "heredoc_redirect",
            Construct::Heredoc,
        ),
        (
            "cat <<-EOF\n\tx\n\tEOF\n",
            "heredoc_redirect",
            Construct::Heredoc,
        ),
        ("a > b", "file_redirect", Construct::Redirect),
        ("a 2>&1", "file_redirect", Construct::Redirect),
        ("a < b", "file_redirect", Construct::Redirect),
        (
            "echo $(date)",
            "command_substitution",
            Construct::CommandSubstitution,
        ),
        (
            "echo `date`",
            "command_substitution",
            Construct::CommandSubstitution,
        ),
        (
            "diff <(a) <(b)",
            "process_substitution",
            Construct::CommandSubstitution,
        ),
    ];
    for (body, kind, expected) in cases {
        assert_eq!(construct_of_first(body, kind), Some(expected), "{body:?}");
    }
}

#[test]
fn a_herestring_is_a_redirect() {
    // `languages/shell:V3`: ⊥ redirect. `<<<` is a redirection like `<`
    // is, and the grammar gives it a node of its own.
    assert_eq!(
        construct_of_first("cat <<< hi", "herestring_redirect"),
        Some(Construct::Redirect)
    );
    assert_eq!(ok("cat <<< hi").constructs, vec![Construct::Redirect]);
    assert!(!ok("cat <<< hi").simple);
}

#[test]
fn plain_command_nodes_are_not_constructs() {
    let body = "FOO=1 echo \"a | b\" 'c; d'";
    for kind in [
        "program",
        "command",
        "command_name",
        "word",
        "string",
        "raw_string",
        "variable_assignment",
    ] {
        assert_eq!(construct_of_first(body, kind), None, "{kind}");
    }
}

#[test]
fn only_the_exact_whitelisted_substitution_is_exempt() {
    let whitelisted = format!("{ALLOWED_SUBSTITUTION}/x.sh");
    assert_eq!(
        construct_of_first(&whitelisted, "command_substitution"),
        None
    );

    // Near misses are ordinary substitutions: the match is on the
    // node's exact text, so spacing, quoting and index all count.
    for near in [
        "$(dirname \"${BASH_SOURCE[1]}\")",
        "$(dirname ${BASH_SOURCE[0]})",
        "$(dirname  \"${BASH_SOURCE[0]}\")",
        "$(dirname \"$0\")",
        "`dirname \"${BASH_SOURCE[0]}\"`",
    ] {
        assert_eq!(
            construct_of_first(near, "command_substitution"),
            Some(Construct::CommandSubstitution),
            "{near:?}"
        );
    }

    // A process substitution wrapping the same command is not the
    // whitelisted text either.
    assert_eq!(
        construct_of_first(
            "cat <(dirname \"${BASH_SOURCE[0]}\")",
            "process_substitution"
        ),
        Some(Construct::CommandSubstitution)
    );
}

// --- top_level_statements: what alone decides `sequence` ----------------

#[test]
fn top_level_statements_counts_program_children_but_not_comments() {
    let cases = [
        ("", 0),
        ("# only a comment", 0),
        ("# one\n# two\n", 0),
        ("a", 1),
        ("# c\na", 1),
        ("a # trailing", 1),
        ("a; b", 2),
        ("a\nb\n", 2),
        ("a; b; c", 3),
        // Nested statements belong to their construct, not the program.
        ("if a; then b; c; d; fi", 1),
        ("f() { a; b; }", 1),
        ("(a; b)", 1),
        // One pipeline and one and-or list are one statement each.
        ("a | b | c", 1),
        ("a && b || c", 1),
    ];
    for (body, expected) in cases {
        let tree = tree(body);
        assert_eq!(top_level_statements(tree.root_node()), expected, "{body:?}");
    }
}

// --- collect: the walk --------------------------------------------------

#[test]
fn collect_finds_nothing_in_a_simple_command() {
    assert_eq!(collected(""), Vec::<Construct>::new());
    assert_eq!(collected("echo \"a | b\""), Vec::<Construct>::new());
    assert_eq!(collected("FOO=1 BAR=2 run --flag"), Vec::<Construct>::new());
}

#[test]
fn collect_reaches_nested_constructs_and_keeps_duplicates() {
    // The walk records every occurrence; sorting and dedup are
    // `classify`'s job, so the raw count here is the number of nodes.
    let found = collected("if a; then b | c; fi");
    assert_eq!(found, vec![Construct::If, Construct::Pipeline]);

    let found = collected("a | b | c");
    assert_eq!(found, vec![Construct::Pipeline]);

    let found = collected("a | b; c | d");
    assert_eq!(found, vec![Construct::Pipeline, Construct::Pipeline]);

    let found = collected("f() { while a; do echo $(b) > c; done; }");
    assert!(found.contains(&Construct::FunctionDefinition), "{found:?}");
    assert!(found.contains(&Construct::While), "{found:?}");
    assert!(found.contains(&Construct::CommandSubstitution), "{found:?}");
    assert!(found.contains(&Construct::Redirect), "{found:?}");
}

#[test]
fn collect_does_not_report_sequence() {
    // `sequence` is counted at the top level only (see `classify`), so
    // the walk never emits it -- nested lists are their construct's.
    for body in ["a; b", "if a; then b; c; fi", "(a; b)"] {
        assert!(!collected(body).contains(&Construct::Sequence), "{body:?}");
    }
}

// --- classify: the assembly ---------------------------------------------

#[test]
fn simple_is_exactly_no_constructs() {
    for body in ["", "echo hi", "# c\necho hi", "A=1 b", "a # trailing"] {
        let found = ok(body);
        assert!(found.simple, "{body:?}: {found:?}");
        assert!(found.constructs.is_empty(), "{body:?}");
    }
    let found = ok("a | b");
    assert!(!found.simple);
    assert!(!found.constructs.is_empty());
}

#[test]
fn nested_statements_are_not_also_a_sequence() {
    assert_eq!(ok("if a; then b; c; fi").constructs, vec![Construct::If]);
    assert_eq!(ok("(a; b)").constructs, vec![Construct::Subshell]);
    assert_eq!(
        ok("f() { a; b; }").constructs,
        vec![Construct::FunctionDefinition]
    );
}

#[test]
fn a_comment_beside_one_command_is_not_a_sequence() {
    assert_eq!(ok("# why\nrun").constructs, Vec::<Construct>::new());
}

#[test]
fn constructs_are_sorted_by_name_and_deduplicated() {
    let found = ok("a | b > c; d | e; while f; do g && h; done");
    assert_eq!(
        found.constructs,
        vec![
            Construct::AndOr,
            Construct::Pipeline,
            Construct::Redirect,
            Construct::Sequence,
            Construct::While,
        ]
    );
}

#[test]
fn the_same_body_always_classifies_the_same() {
    // `src:V11`: a fresh parser per call, no state carried between.
    let body = "a | b && c > d";
    assert_eq!(ok(body), ok(body));
}

#[test]
fn an_unparseable_body_is_a_parse_error_for_shell() {
    for body in ["if then fi )", "echo \"unterminated", "a |", "case x in"] {
        match classify(body) {
            Err(xenolith_lang_api::Error::Parse { lang, message }) => {
                assert_eq!(lang, xenolith_lang_api::LangId::Shell, "{body:?}");
                assert!(!message.is_empty(), "{body:?}");
            }
            other => panic!("expected a parse error for {body:?}, got {other:?}"),
        }
    }
}

// --- classify_in: the dialect (`languages/shell:V138`) -------------------

fn dialect(name: Option<&str>) -> GuestEnv {
    GuestEnv {
        dialect: name.map(str::to_owned),
        options: Vec::new(),
    }
}

fn ok_in(body: &str, env: &GuestEnv) -> Classification {
    match classify_in(body, env) {
        Ok(found) => found,
        Err(err) => panic!("expected a classification of {body:?}, got: {err:?}"),
    }
}

/// zsh the bash grammar rejects: flags, qualifiers, an anon function,
/// the short `for`.
const ZSH_ONLY: &[&str] = &[
    "print -rl -- ${(f)\"$(ls)\"}",
    "source ~/.zsh/*.zsh(N)",
    "print -rl -- *(.)",
    "() { print hi }",
    "for x (a b) print $x",
];

#[test]
fn zsh_only_syntax_under_zsh_is_unsupported_not_an_error() {
    for body in ZSH_ONLY {
        let found = ok_in(body, &dialect(Some("zsh")));
        assert!(found.unsupported, "{body:?}: {found:?}");
        assert!(!found.simple, "{body:?}");
        // Spans in an ERROR tree are unreliable (`languages:V78`), so a
        // construct named from one would be something nobody wrote.
        assert!(found.constructs.is_empty(), "{body:?}");
    }
}

#[test]
fn the_same_text_outside_zsh_is_still_a_parse_error() {
    // `languages:V77`: for sh and bash the grammar IS the language.
    for name in [None, Some("bash"), Some("sh"), Some("dash")] {
        for body in ZSH_ONLY {
            assert!(
                matches!(
                    classify_in(body, &dialect(name)),
                    Err(xenolith_lang_api::Error::Parse { .. })
                ),
                "{name:?} {body:?}"
            );
        }
    }
}

#[test]
fn zsh_the_bash_grammar_reads_is_classified_as_usual() {
    let zsh = dialect(Some("zsh"));
    for body in ["setopt err_exit", "print hi", "autoload -Uz compinit"] {
        let found = ok_in(body, &zsh);
        assert!(found.simple && !found.unsupported, "{body:?}: {found:?}");
    }
    let found = ok_in("autoload -Uz compinit && compinit", &zsh);
    assert!(!found.unsupported);
    assert_eq!(found.constructs, vec![Construct::AndOr]);
}

#[test]
fn classify_is_classify_in_with_no_dialect() {
    for body in ["echo hi", "a | b", "if a; then b; fi"] {
        assert_eq!(ok(body), ok_in(body, &GuestEnv::default()), "{body:?}");
        assert!(!ok(body).unsupported, "{body:?}");
    }
}
