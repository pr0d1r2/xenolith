//! Unit tests for `host.rs` (`src:C139`): the claim, the word grouping,
//! the program word, the walk and the error containment, part by part.

use std::path::Path;

use xenolith_lang_api::{Delim, DelimKind, Error, Host, Invoke, LangId, LoadRef, Site, Span};

use super::{
    EXTENSIONS, INTERPRETERS, TclHost, Walk, dialect, errors_contained, plain, program_word,
    shebang_interpreter, words,
};
use crate::grammar::parse;

/// The walk over `src`: its sites and its opaque spans.
fn walked(src: &str) -> Walk {
    let tree = parse(src).unwrap_or_else(|e| panic!("{e}"));
    let mut walk = Walk::default();
    walk.node(tree.root_node(), src);
    walk
}

fn sinks(src: &str) -> Vec<String> {
    walked(src)
        .sites
        .into_iter()
        .map(|site| site.sink)
        .collect()
}

/// The words of the first command's argument list in `src`, as text.
fn word_texts(src: &str) -> Vec<String> {
    let tree = parse(src).unwrap_or_else(|e| panic!("{e}"));
    let command = tree
        .root_node()
        .named_child(0)
        .unwrap_or_else(|| unreachable!());
    let list = command
        .child_by_field_name("arguments")
        .unwrap_or_else(|| panic!("no arguments in {src:?}"));
    let mut cursor = list.walk();
    let nodes: Vec<_> = list.children(&mut cursor).collect();
    words(&nodes)
        .iter()
        .map(|word| {
            let start = word.first().map_or(0, tree_sitter::Node::start_byte);
            let end = word.last().map_or(0, tree_sitter::Node::end_byte);
            src.get(start..end).unwrap_or_default().to_owned()
        })
        .collect()
}

/// The program word of the LAST argument of the first command in `src`.
fn last_word(src: &str) -> Option<(Delim, Vec<Span>)> {
    let tree = parse(src).unwrap_or_else(|e| panic!("{e}"));
    let command = tree.root_node().named_child(0)?;
    let list = command.child_by_field_name("arguments")?;
    let mut cursor = list.walk();
    let nodes: Vec<_> = list.children(&mut cursor).collect();
    let grouped = words(&nodes);
    program_word(grouped.last()?, src)
}

#[test]
fn the_tables_are_sorted_and_name_the_dialect() {
    assert!(EXTENSIONS.is_sorted());
    assert!(INTERPRETERS.is_sorted());
    assert!(INTERPRETERS.contains(&super::EXPECT));
}

#[test]
fn a_shebang_names_its_interpreter_version_stripped() {
    assert_eq!(
        shebang_interpreter("#!/usr/bin/env tclsh8.6"),
        Some("tclsh")
    );
    assert_eq!(shebang_interpreter("#!/usr/bin/expect -f"), Some("expect"));
    assert_eq!(shebang_interpreter("#!/usr/local/bin/wish"), Some("wish"));
    assert_eq!(shebang_interpreter("#!/bin/sh"), None);
    assert_eq!(shebang_interpreter("puts hi"), None);
}

#[test]
fn the_dialect_is_expect_by_extension_or_shebang_only() {
    assert_eq!(dialect(Path::new("a.exp"), ""), Some("expect"));
    assert_eq!(
        dialect(Path::new("bin/login"), "#!/usr/bin/env expect"),
        Some("expect")
    );
    assert_eq!(dialect(Path::new("a.tcl"), ""), None);
    assert_eq!(dialect(Path::new("a.tcl"), "#!/usr/bin/env tclsh"), None);
    assert_eq!(dialect(Path::new("a.tk"), "#!/usr/bin/env wish"), None);
}

#[test]
fn adjacent_pieces_are_one_word() {
    assert_eq!(
        word_texts("puts foo$bar [x]y \"q\" {b} $z\n"),
        ["foo$bar", "[x]y", "\"q\"", "{b}", "$z"]
    );
}

#[test]
fn only_a_lone_simple_word_is_plain() {
    let src = "exec sh $x foo$y {z}\n";
    let tree = parse(src).unwrap_or_else(|e| panic!("{e}"));
    let command = tree
        .root_node()
        .named_child(0)
        .unwrap_or_else(|| unreachable!());
    let list = command
        .child_by_field_name("arguments")
        .unwrap_or_else(|| unreachable!());
    let mut cursor = list.walk();
    let nodes: Vec<_> = list.children(&mut cursor).collect();
    let texts: Vec<Option<&str>> = words(&nodes).iter().map(|w| plain(w, src)).collect();
    assert_eq!(texts, [Some("sh"), None, None, None]);
}

#[test]
fn a_braced_program_is_its_inside_verbatim() {
    let src = "exec sh -c {a | b}\n";
    let (delim, holes) = last_word(src).unwrap_or_else(|| panic!("no program"));
    assert_eq!(delim.kind, DelimKind::ArgvString);
    assert_eq!(delim.open.of(src), Some("{"));
    assert_eq!(delim.body.of(src), Some("a | b"));
    assert_eq!(delim.close.of(src), Some("}"));
    assert!(holes.is_empty());
}

#[test]
fn a_braced_program_with_backslash_newline_is_no_site() {
    // Tcl replaces `\` + newline with a space even inside braces, so the
    // bytes are not the argument.
    assert_eq!(last_word("exec sh -c {a \\\n b}\n"), None);
}

#[test]
fn a_quoted_program_holds_its_substitutions_as_holes() {
    let src = "exec sh -c \"echo $x [pwd]\"\n";
    let (delim, holes) = last_word(src).unwrap_or_else(|| panic!("no program"));
    assert_eq!(delim.open.of(src), Some("\""));
    assert_eq!(delim.body.of(src), Some("echo $x [pwd]"));
    let holes: Vec<_> = holes.iter().filter_map(|h| h.of(src)).collect();
    assert_eq!(holes, ["$x", "[pwd]"]);
}

#[test]
fn a_quoted_program_with_an_escape_is_no_site() {
    assert_eq!(last_word("exec sh -c \"echo \\$x\"\n"), None);
}

#[test]
fn a_bare_program_has_empty_delimiters_around_the_word() {
    let src = "exec python3 << $src\n";
    let (delim, holes) = last_word(src).unwrap_or_else(|| panic!("no program"));
    assert_eq!(delim.open, Span::new(16, 16));
    assert_eq!(delim.body.of(src), Some("$src"));
    assert_eq!(delim.close, Span::new(20, 20));
    assert_eq!(holes, [Span::new(16, 20)]);
    let src = "exec sh -c foo$bar\n";
    let (delim, holes) = last_word(src).unwrap_or_else(|| panic!("no program"));
    assert_eq!(delim.body.of(src), Some("foo$bar"));
    assert_eq!(holes.len(), 1);
}

#[test]
fn a_mixed_or_expanded_word_is_no_program() {
    assert_eq!(last_word("exec sh -c {*}$cmd\n"), None);
    assert_eq!(last_word("exec sh -c a\"b\"\n"), None);
    assert_eq!(last_word("exec sh -c a\\ b\n"), None);
}

#[test]
fn the_walk_reaches_script_positions_at_any_depth() {
    let src = "proc f {} {\n  if {1} {\n    set x [exec sh -c {a | b}]\n  }\n}\n";
    assert_eq!(sinks(src), ["exec sh -c"]);
    let src = "foreach f $l {\n  catch {spawn bash -c {x}}\n}\n";
    assert_eq!(sinks(src), ["spawn bash -c"]);
    let src = "puts \"[exec sh -c {a | b}]\"\n";
    assert_eq!(sinks(src), ["exec sh -c"]);
}

#[test]
fn braced_arguments_of_an_ordinary_command_are_data() {
    let walk = walked("puts {exec sh -c {a | b}}\n");
    assert!(walk.sites.is_empty());
    assert_eq!(walk.opaque, [Span::new(6, 24)]);
    assert!(sinks("set doc {exec sh -c {a | b}}\n").is_empty());
}

#[test]
fn a_site_body_is_opaque() {
    let walk = walked("exec sh -c {a | b}\n");
    assert_eq!(walk.opaque, [Span::new(12, 17)]);
}

#[test]
fn namespace_eval_runs_its_body_and_other_subcommands_do_not() {
    assert_eq!(
        sinks("namespace eval ns {\n  exec sh -c {a | b}\n}\n"),
        ["exec sh -c"]
    );
    assert!(sinks("namespace ensemble create -map {x {exec sh -c {a}}}\n").is_empty());
}

#[test]
fn a_comment_holds_no_site() {
    assert!(sinks("# exec sh -c {a | b}\n").is_empty());
}

#[test]
fn errors_inside_opaque_braces_are_contained() {
    let src = "exec sh -c {ls $(pwd) | wc -l}\n";
    let tree = parse(src).unwrap_or_else(|e| panic!("{e}"));
    assert!(tree.root_node().has_error());
    let walk = walked(src);
    assert!(errors_contained(tree.root_node(), &walk.opaque));
}

#[test]
fn errors_outside_opaque_braces_are_not() {
    let src = "expect \"$ \"\n";
    let tree = parse(src).unwrap_or_else(|e| panic!("{e}"));
    assert!(tree.root_node().has_error());
    let walk = walked(src);
    assert!(!errors_contained(tree.root_node(), &walk.opaque));
}

#[test]
fn a_clean_tree_is_contained_trivially() {
    let tree = parse("puts hi\n").unwrap_or_else(|e| panic!("{e}"));
    assert!(errors_contained(tree.root_node(), &[]));
}

#[test]
fn loads_rewrite_and_inline_are_refused_by_name() {
    let site = Site {
        sink: "exec sh -c".to_owned(),
        guest: LangId::Shell,
        env: xenolith_lang_api::GuestEnv::default(),
        delim: Delim {
            kind: DelimKind::ArgvString,
            open: Span::new(0, 1),
            body: Span::new(1, 2),
            close: Span::new(2, 3),
        },
        holes: Vec::new(),
    };
    let invoke = Invoke {
        argv: vec!["sh".to_owned(), "x.sh".to_owned()],
    };
    let load = LoadRef {
        span: Span::new(0, 1),
        path: "x.sh".into(),
        guest: LangId::Shell,
    };
    assert_eq!(
        TclHost.loads(""),
        Err(Error::unsupported(LangId::Tcl, "loads"))
    );
    assert_eq!(
        TclHost.rewrite("", &site, &invoke, Path::new("x.sh")),
        Err(Error::unsupported(LangId::Tcl, "rewrite"))
    );
    assert_eq!(
        TclHost.inline("", &load, ""),
        Err(Error::unsupported(LangId::Tcl, "inline"))
    );
}

#[test]
fn unescape_is_verbatim_for_argv_strings_only() {
    let argv = Delim {
        kind: DelimKind::ArgvString,
        open: Span::new(0, 0),
        body: Span::new(0, 0),
        close: Span::new(0, 0),
    };
    assert_eq!(
        TclHost.unescape(&argv, "a \\n $b"),
        Ok("a \\n $b".to_owned())
    );
    let other = Delim {
        kind: DelimKind::NixIndented,
        ..argv
    };
    assert_eq!(
        TclHost.unescape(&other, "x"),
        Err(Error::unsupported(LangId::Tcl, "unescape"))
    );
}

#[test]
fn no_host_check_is_chosen_before_it_is_measured() {
    assert!(TclHost.checks().is_empty());
    assert!(TclHost.fixers().is_empty());
}
