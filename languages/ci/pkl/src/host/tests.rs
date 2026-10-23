//! Unit tests for the pkl host's own logic (`src:C139`).
//!
//! `tests/host.rs` drives [`PklHost`] through the `Host` trait over
//! fixtures. These reach the private helpers behind it -- the three
//! layers of sink context, the site and load readers, the argv filter and
//! the splice -- one branch at a time, on sources small enough that the
//! case is the whole file.

use std::path::{Path, PathBuf};

use xenolith_lang_api::{
    Delim, DelimKind, Error, GuestEnv, Host, Invoke, LangId, LoadRef, Site, Span,
};

use super::{
    FILES, PklHost, SINKS, is_hk_config, line_break, load, plain_word, sinks, site, splice,
};
use crate::syntax::{self, Member, Module, Value};

/// A hk config header, the layer-1 context every positive case needs.
const HK: &str = "amends \"pkl/Config.pkl\"\n";

fn module(src: &str) -> Module {
    syntax::parse(src).unwrap_or_else(|e| panic!("parse failed: {e}"))
}

fn nth<T>(items: &[T], i: usize) -> &T {
    items
        .get(i)
        .unwrap_or_else(|| panic!("expected an item at {i}, found {}", items.len()))
}

/// The plain content of the top-level property `name = "..."`, if any.
fn plain(src: &str, name: &str) -> Option<String> {
    module(src)
        .body
        .members
        .into_iter()
        .find_map(|member| match member {
            Member::Property(property) if property.name == name => match property.value {
                Value::Str(literal) => literal.plain,
                _ => None,
            },
            _ => None,
        })
}

/// A hk config: the header, then `lines`, each ending in a newline.
fn hk(lines: &[&str]) -> String {
    let mut src = HK.to_owned();
    for line in lines {
        src.push_str(line);
        src.push('\n');
    }
    src
}

/// One hk step `["name"] { ... }` in a hk config, `body` as its lines.
fn step(name: &str, body: &str) -> String {
    format!("{HK}steps {{\n  [\"{name}\"] {{\n{body}  }}\n}}\n")
}

fn hk_sinks(src: &str) -> Vec<(String, String)> {
    let module = module(src);
    sinks(&module)
        .into_iter()
        .map(|sink| (sink.step, sink.property))
        .collect()
}

fn pairs(items: &[(&str, &str)]) -> Vec<(String, String)> {
    items
        .iter()
        .map(|&(a, b)| (a.to_owned(), b.to_owned()))
        .collect()
}

fn only_site(src: &str) -> Site {
    let found = PklHost
        .sites(src)
        .unwrap_or_else(|e| panic!("sites failed: {e}"));
    assert_eq!(found.len(), 1, "expected one site in {src:?}: {found:?}");
    nth(&found, 0).clone()
}

fn only_load(src: &str) -> Option<LoadRef> {
    let module = module(src);
    let found: Vec<LoadRef> = sinks(&module).iter().filter_map(load).collect();
    assert!(found.len() <= 1, "expected at most one load: {found:?}");
    found.into_iter().next()
}

fn invoke(argv: &[&str]) -> Invoke {
    Invoke {
        argv: argv.iter().map(|&word| word.to_owned()).collect(),
    }
}

fn check_script(src_body: &str) -> String {
    step(
        "lint",
        &format!("    check = \"\"\"\n{src_body}      \"\"\"\n"),
    )
}

// --- what Pkl rejects (`languages:V78`) --------------------------------------

#[test]
fn a_file_pkl_rejects_has_no_sites_or_loads_only_a_parse_error() {
    // Anything Pkl itself rejects fails the whole file: spans after an
    // error are a guess, and a site built on a guess rewrites the wrong
    // bytes. Pkl 0.31 rejects a `///` doc comment inside an object too.
    for src in [
        "x = = )\n".to_owned(),
        format!(
            "{HK}steps {{\n  [\"a\"] {{ check = \"x\" }}\n  [\"b\"] {{ ) check = \"y\" }}\n}}\n"
        ),
        format!(
            "{HK}steps {{\n  [\"a\"] {{ check = \"x\" }}\n  [\"b\"] {{ check = (\"y\" }}\n}}\n"
        ),
        check_script("      \\(\n"),
        format!("{HK}steps {{\n  /// a step\n  [\"a\"] {{ check = \"x\" }}\n}}\n"),
    ] {
        for got in [
            PklHost.sites(&src).map(|_| ()),
            PklHost.loads(&src).map(|_| ()),
        ] {
            assert!(
                matches!(
                    got,
                    Err(Error::Parse {
                        lang: LangId::Pkl,
                        ..
                    })
                ),
                "{src:?}: {got:?}"
            );
        }
    }
    assert!(syntax::parse("x = 1\n").is_ok());
}

// --- plain strings ---------------------------------------------------------

#[test]
fn a_plain_string_is_its_one_literal_part() {
    assert_eq!(
        plain("a = \"bash x.sh\"\n", "a").as_deref(),
        Some("bash x.sh")
    );
}

#[test]
fn the_amends_uri_is_a_plain_string_too() {
    assert_eq!(module(HK).amends.as_deref(), Some("pkl/Config.pkl"));
}

#[test]
fn escapes_interpolations_and_empties_are_not_plain() {
    let src = "a = \"x\\ny\"\nb = \"\\(y) z\"\nc = \"\"\ny = 1\n";
    for name in ["a", "b", "c"] {
        assert_eq!(plain(src, name), None, "{name}");
    }
}

#[test]
fn other_expressions_are_not_plain_strings() {
    let src = "a = \"\"\"\n  x\n  \"\"\"\nb = c\nc = 2\nd = 1\n";
    for name in ["a", "b", "d"] {
        assert_eq!(plain(src, name), None, "{name}");
    }
}

// --- is_hk_config (layer 1) ------------------------------------------------

fn hk_config(src: &str) -> bool {
    syntax::parse(src).is_ok_and(|module| is_hk_config(&module))
}

#[test]
fn amending_config_pkl_by_any_uri_is_a_hk_config() {
    assert!(hk_config("amends \"pkl/Config.pkl\"\n"));
    assert!(hk_config("amends \"Config.pkl\"\n"));
    assert!(hk_config(
        "amends \"package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl\"\n"
    ));
    assert!(hk_config("module x\namends \"../pkl/Config.pkl\"\n"));
}

#[test]
fn another_module_is_not_a_hk_config() {
    assert!(!hk_config("amends \"pkl/MyConfig.pkl\"\n"));
    assert!(!hk_config("amends \"pkl/Config.pkl.bak\"\n"));
    assert!(!hk_config("amends \"pkl:Project\"\n"));
}

#[test]
fn extending_config_pkl_is_not_amending_it() {
    assert!(!hk_config("extends \"pkl/Config.pkl\"\n"));
}

#[test]
fn a_module_without_a_header_or_an_amends_is_not_a_hk_config() {
    assert!(!hk_config("x = 1\n"));
    assert!(!hk_config("module x\n"));
    assert!(!hk_config(""));
}

#[test]
fn an_interpolated_amends_uri_is_not_read() {
    // No plain URI, so the module is not known to be hk.
    assert!(!hk_config("amends \"\\(x)/Config.pkl\"\n"));
}

// --- sinks (layers 2 and 3) ----------------------------------------------

#[test]
fn every_hk_command_property_in_a_step_is_a_sink() {
    let body: String = SINKS
        .iter()
        .flat_map(|name| ["    ", name, " = \"x\"\n"])
        .collect();
    let expected: Vec<(&str, &str)> = SINKS.iter().map(|&name| ("s", name)).collect();
    assert_eq!(hk_sinks(&step("s", &body)), pairs(&expected));
}

#[test]
fn a_sink_carries_its_value() {
    let src = step("s", "    check = \"bash x.sh\"\n");
    let module = module(&src);
    let found = sinks(&module);
    let Value::Str(literal) = nth(&found, 0).value else {
        panic!("not a string value");
    };
    assert_eq!(literal.span.of(&src), Some("\"bash x.sh\""));
}

#[test]
fn other_step_properties_are_not_sinks() {
    let src = step(
        "s",
        "    glob = List(\"**/*\")\n    message = \"\"\"\n      rm -rf /\n      \"\"\"\n",
    );
    assert_eq!(hk_sinks(&src), pairs(&[]));
}

#[test]
fn a_local_named_like_a_sink_is_not_one() {
    let src = step("s", "    local check = \"x\"\n    fix = \"y\"\n");
    assert_eq!(hk_sinks(&src), pairs(&[("s", "fix")]));
}

#[test]
fn a_sink_name_outside_a_step_entry_is_not_one() {
    // Directly in a top-level object, one object deeper than a step, and
    // in an object a step's key builds with `new`.
    let src = hk(&[
        "hooks { check = \"x\" }",
        "steps {",
        "  [\"s\"] {",
        "    nested { check = \"y\" }",
        "  }",
        "  [\"t\"] = new Dynamic { check = \"z\" }",
        "}",
    ]);
    assert_eq!(hk_sinks(&src), pairs(&[]));
}

#[test]
fn a_step_needs_a_plain_non_empty_string_key() {
    let src = hk(&[
        "steps {",
        "  [1] { check = \"a\" }",
        "  [\"\"] { check = \"b\" }",
        "  [\"\\(x)\"] { check = \"c\" }",
        "  [\"ok\"] { check = \"d\" }",
        "}",
    ]);
    assert_eq!(hk_sinks(&src), pairs(&[("ok", "check")]));
}

#[test]
fn a_step_inside_a_new_mapping_or_a_generator_is_still_read() {
    let src = hk(&[
        "steps = new Mapping<String, Step> {",
        "  [\"m\"] { fix = \"1\" }",
        "  when (true) { [\"w\"] { check = \"2\" } }",
        "}",
    ]);
    assert_eq!(hk_sinks(&src), pairs(&[("m", "fix"), ("w", "check")]));
}

#[test]
fn steps_inside_expressions_and_other_entry_values_are_still_read() {
    let src = hk(&[
        "local made = List(new Dynamic { [\"in-call\"] { check = \"1\" } })",
        "steps {",
        "  [\"listed\"] = List(new Dynamic { [\"nested\"] { fix = \"2\" } })",
        "  [\"plain\"] = \"not a step\"",
        "}",
    ]);
    assert_eq!(
        hk_sinks(&src),
        pairs(&[("in-call", "check"), ("nested", "fix")])
    );
}

#[test]
fn a_module_that_is_not_hk_has_no_sinks() {
    let src = "amends \"other.pkl\"\nsteps {\n  [\"s\"] { check = \"x\" }\n}\n";
    assert_eq!(hk_sinks(src), pairs(&[]));
}

#[test]
fn sinks_come_in_source_order_across_steps() {
    let src = hk(&[
        "steps {",
        "  [\"b\"] {",
        "    fix = \"1\"",
        "    check = \"2\"",
        "  }",
        "  [\"a\"] { shell = \"3\" }",
        "}",
    ]);
    assert_eq!(
        hk_sinks(&src),
        pairs(&[("b", "fix"), ("b", "check"), ("a", "shell")])
    );
}

// --- site ----------------------------------------------------------------------

#[test]
fn a_multi_line_sink_value_is_a_shell_site() {
    let src = check_script("      echo hi\n");
    let found = only_site(&src);
    assert_eq!(found.sink, "lint.check");
    assert_eq!(found.guest, LangId::Shell);
    // No `shell` anywhere: hk's own `sh -o errexit -c` (`languages/ci/pkl:V172`).
    let hk_default = GuestEnv {
        dialect: Some("sh".to_owned()),
        options: vec!["errexit".to_owned()],
    };
    assert_eq!(found.env, hk_default);
    assert_eq!(found.delim.kind, DelimKind::PklMultiline { pounds: 0 });
    assert_eq!(found.delim.open.of(&src), Some("\"\"\""));
    assert_eq!(found.delim.close.of(&src), Some("\"\"\""));
    assert_eq!(found.delim.body.of(&src), Some("\n      echo hi\n      "));
    assert_eq!(found.delim.open.end, found.delim.body.start);
    assert_eq!(found.delim.body.end, found.delim.close.start);
    assert!(found.holes.is_empty());
}

#[test]
fn the_pound_count_comes_from_the_opening_delimiter() {
    for pounds in [1_usize, 3] {
        let guard = "#".repeat(pounds);
        let src = step(
            "s",
            &format!("    fix = {guard}\"\"\"\n      sed 's/\\t/ /'\n      \"\"\"{guard}\n"),
        );
        let found = only_site(&src);
        assert_eq!(found.delim.kind, DelimKind::PklMultiline { pounds });
        assert_eq!(
            found.delim.open.of(&src),
            Some(format!("{guard}\"\"\"").as_str())
        );
        assert_eq!(
            found.delim.close.of(&src),
            Some(format!("\"\"\"{guard}").as_str())
        );
    }
}

#[test]
fn interpolations_are_holes_in_source_order() {
    let src = check_script("      echo \\(a) and \\(b.c)\n");
    let found = only_site(&src);
    let holes: Vec<Option<&str>> = found.holes.iter().map(|hole| hole.of(&src)).collect();
    assert_eq!(holes, [Some("\\(a)"), Some("\\(b.c)")]);
}

#[test]
fn a_pound_guarded_interpolation_is_a_hole_and_a_bare_one_is_text() {
    let src = step(
        "s",
        "    check = #\"\"\"\n      \\(literal) \\#(real)\n      \"\"\"#\n",
    );
    let found = only_site(&src);
    let holes: Vec<Option<&str>> = found.holes.iter().map(|hole| hole.of(&src)).collect();
    assert_eq!(holes, [Some("\\#(real)")]);
}

#[test]
fn single_line_and_non_string_values_are_not_sites() {
    let src = step("s", "    check = \"bash x.sh\"\n    fix = other\n");
    let module = module(&src);
    let found = sinks(&module);
    assert_eq!(found.len(), 2);
    assert!(found.iter().all(|sink| site(sink).is_none()));
}

// --- load -----------------------------------------------------------------------

fn load_of(command: &str) -> Option<LoadRef> {
    only_load(&step("s", &format!("    check = \"{command}\"\n")))
}

#[test]
fn a_load_is_an_interpreter_a_script_and_maybe_files() {
    for (command, script) in [
        ("bash scripts/hk/s.sh {{files}}", "scripts/hk/s.sh"),
        ("bash scripts/hk/s.sh", "scripts/hk/s.sh"),
        ("sh x.sh", "x.sh"),
        ("zsh x.zsh {{files}}", "x.zsh"),
        ("bash x.bash", "x.bash"),
        ("  bash   x.sh   {{files}} ", "x.sh"),
    ] {
        let found = load_of(command).unwrap_or_else(|| panic!("no load in {command:?}"));
        assert_eq!(found.path, PathBuf::from(script), "{command}");
        assert_eq!(found.guest, LangId::Shell, "{command}");
    }
}

#[test]
fn a_load_spans_the_whole_string_literal() {
    let src = step("s", "    check = \"sh x.sh\"\n");
    let found = only_load(&src).unwrap_or_else(|| panic!("no load in {src:?}"));
    assert_eq!(found.span.of(&src), Some("\"sh x.sh\""));
}

#[test]
fn other_commands_are_not_loads() {
    for command in [
        "",
        "bash",
        "python x.sh",
        "bash x.py",
        "bash x",
        "bash x.sh --fix",
        "bash x.sh {{files}} {{files}}",
        "bash x.sh {{staged_files}}",
        "cargo fmt --check",
    ] {
        assert_eq!(load_of(command), None, "{command:?}");
    }
}

#[test]
fn a_load_must_be_a_plain_single_line_string() {
    let escaped = step("s", "    check = \"bash\\tx.sh\"\n");
    assert_eq!(only_load(&escaped), None);
    let interpolated = step("s", "    check = \"bash \\(x).sh\"\n");
    assert_eq!(only_load(&interpolated), None);
    let multiline = check_script("      bash x.sh\n");
    assert_eq!(only_load(&multiline), None);
}

// --- plain_word ----------------------------------------------------------------

#[test]
fn plain_words_need_no_quoting_anywhere() {
    for word in ["bash", "scripts/hk/x.sh", "a-b_c.d", "+x,y:z@w=v%u", "0"] {
        assert!(plain_word(word), "{word:?}");
    }
}

#[test]
fn words_that_need_quoting_are_not_plain() {
    for word in [
        "",
        "a b",
        "$x",
        "a'b",
        "a\"b",
        "a\\b",
        "{{files}}",
        "a;b",
        "x*",
        "é",
        "a\tb",
    ] {
        assert!(!plain_word(word), "{word:?}");
    }
}

// --- splice ----------------------------------------------------------------------

#[test]
fn splice_replaces_exactly_the_span() {
    assert_eq!(
        splice("abcdef", Span::new(1, 3), "XY"),
        Ok("aXYdef".to_owned())
    );
    assert_eq!(splice("abc", Span::new(0, 3), ""), Ok(String::new()));
    assert_eq!(splice("abc", Span::new(3, 3), "d"), Ok("abcd".to_owned()));
    assert_eq!(splice("abc", Span::new(1, 1), "-"), Ok("a-bc".to_owned()));
}

#[test]
fn splice_refuses_a_span_it_cannot_cut() {
    let outside = Error::parse(LangId::Pkl, "span outside the source");
    assert_eq!(splice("abc", Span::new(2, 4), "x"), Err(outside.clone()));
    assert_eq!(splice("abc", Span::new(4, 5), "x"), Err(outside.clone()));
    // Inside a multi-byte character.
    assert_eq!(splice("é", Span::new(1, 2), "x"), Err(outside));
}

// --- line_break (`languages/ci/pkl:B2`) -----------------------------------------------

#[test]
fn line_break_is_the_one_ending_the_line_else_the_one_before() {
    let at = Span::new(4, 5);
    assert_eq!(line_break("abc\nX\r\n", at), "\r\n");
    assert_eq!(line_break("abc\r\nX\n", Span::new(5, 6)), "\n");
    assert_eq!(line_break("abc\r\nX", Span::new(5, 6)), "\r\n");
    assert_eq!(line_break("abc\nX", at), "\n");
    assert_eq!(line_break("X", Span::new(0, 1)), "\n");
    assert_eq!(line_break("\nX", Span::new(1, 2)), "\n");
}

// --- Host --------------------------------------------------------------------------

#[test]
fn the_host_is_pkl_with_no_checks_or_fixers_yet() {
    assert_eq!(PklHost.id(), LangId::Pkl);
    assert!(PklHost.checks().is_empty());
    assert!(PklHost.fixers().is_empty());
}

#[test]
fn claims_by_extension_or_the_project_file_name() {
    assert!(PklHost.claims(Path::new("a/b.pkl"), ""));
    assert!(PklHost.claims(Path::new("PklProject"), ""));
    assert!(PklHost.claims(Path::new("sub/PklProject"), ""));
    assert!(!PklHost.claims(Path::new("pkl"), ""));
    assert!(!PklHost.claims(Path::new("x.PKL"), ""));
    assert!(!PklHost.claims(Path::new("PklProject.deps.json"), ""));
    // No shebang reading: the head is ignored.
    assert!(!PklHost.claims(Path::new("run"), "#!/usr/bin/env pkl eval"));
}

#[test]
fn sites_keep_only_multi_line_sinks_and_loads_only_single_line_ones() {
    let src = step(
        "s",
        "    check = \"\"\"\n      echo 1\n      \"\"\"\n    fix = \"bash x.sh {{files}}\"\n",
    );
    let found = only_site(&src);
    assert_eq!(found.sink, "s.check");
    let loads = PklHost
        .loads(&src)
        .unwrap_or_else(|e| panic!("loads failed: {e}"));
    assert_eq!(loads.len(), 1);
    assert_eq!(nth(&loads, 0).path, PathBuf::from("x.sh"));
}

#[test]
fn rewrite_replaces_the_literal_with_the_argv_and_forwards_files() {
    let src = check_script("      echo hi\n");
    let found = only_site(&src);
    let out = PklHost
        .rewrite(
            &src,
            &found,
            &invoke(&["bash", "scripts/hk/lint.sh"]),
            Path::new("hk.pkl"),
        )
        .unwrap_or_else(|e| panic!("rewrite failed: {e}"));
    assert_eq!(
        out,
        step(
            "lint",
            &format!("    check = \"bash scripts/hk/lint.sh {FILES}\"\n")
        )
    );
    let loads = PklHost
        .loads(&out)
        .unwrap_or_else(|e| panic!("loads failed: {e}"));
    assert_eq!(loads.len(), 1);
    assert_eq!(nth(&loads, 0).path, PathBuf::from("scripts/hk/lint.sh"));
}

#[test]
fn rewrite_refuses_a_site_that_is_not_in_the_source() {
    let src = check_script("      echo hi\n");
    let mut moved = only_site(&src);
    moved.delim.open = Span::new(moved.delim.open.start + 1, moved.delim.open.end + 1);
    let err = PklHost.rewrite(
        &src,
        &moved,
        &invoke(&["bash", "x.sh"]),
        Path::new("hk.pkl"),
    );
    assert!(
        matches!(
            err,
            Err(Error::Parse {
                lang: LangId::Pkl,
                ..
            })
        ),
        "{err:?}"
    );

    let other = Site {
        sink: "nope.check".to_owned(),
        guest: LangId::Shell,
        env: GuestEnv::default(),
        delim: Delim {
            kind: DelimKind::NixIndented,
            open: Span::new(0, 2),
            body: Span::new(2, 3),
            close: Span::new(3, 5),
        },
        holes: Vec::new(),
    };
    let err = PklHost.rewrite(
        &src,
        &other,
        &invoke(&["bash", "x.sh"]),
        Path::new("hk.pkl"),
    );
    assert!(
        matches!(
            err,
            Err(Error::Parse {
                lang: LangId::Pkl,
                ..
            })
        ),
        "{err:?}"
    );
}

#[test]
fn rewrite_refuses_an_argv_it_could_not_read_back() {
    let src = check_script("      echo hi\n");
    let found = only_site(&src);
    let unsupported = Err(Error::unsupported(LangId::Pkl, "rewrite"));
    for argv in [
        &[][..],
        &["bash", "my script.sh"][..],
        &["bash", "x.sh", "$1"][..],
        &["bash", ""][..],
    ] {
        assert_eq!(
            PklHost.rewrite(&src, &found, &invoke(argv), Path::new("hk.pkl")),
            unsupported,
            "{argv:?}"
        );
    }
}

#[test]
fn inline_indents_the_body_one_step_past_the_property() {
    let src = step("s", "    check = \"bash x.sh {{files}}\"\n");
    let loads = PklHost
        .loads(&src)
        .unwrap_or_else(|e| panic!("loads failed: {e}"));
    let out = PklHost
        .inline(&src, nth(&loads, 0), "echo a\n\necho b")
        .unwrap_or_else(|e| panic!("inline failed: {e}"));
    assert_eq!(
        out,
        step(
            "s",
            "    check = \"\"\"\n      echo a\n\n      echo b\n      \"\"\"\n"
        )
    );
}

#[test]
fn inline_keeps_a_tab_indent() {
    let src = format!("{HK}steps {{\n\t[\"s\"] {{\n\t\tcheck = \"sh x.sh\"\n\t}}\n}}\n");
    let loads = PklHost
        .loads(&src)
        .unwrap_or_else(|e| panic!("loads failed: {e}"));
    let out = PklHost
        .inline(&src, nth(&loads, 0), "true")
        .unwrap_or_else(|e| panic!("inline failed: {e}"));
    assert_eq!(
        out,
        format!(
            "{HK}steps {{\n\t[\"s\"] {{\n\t\tcheck = \"\"\"\n\t\t  true\n\t\t  \"\"\"\n\t}}\n}}\n"
        )
    );
}

#[test]
fn inline_then_sites_round_trips_the_body() {
    let src = step("s", "    fix = \"bash x.sh\"\n");
    let loads = PklHost
        .loads(&src)
        .unwrap_or_else(|e| panic!("loads failed: {e}"));
    let body = "sed 's/\\t/ /' \"$f\"\n";
    let out = PklHost
        .inline(&src, nth(&loads, 0), body)
        .unwrap_or_else(|e| panic!("inline failed: {e}"));
    let found = only_site(&out);
    let raw = found
        .delim
        .body
        .of(&out)
        .unwrap_or_else(|| panic!("body span does not fit"));
    assert_eq!(
        crate::string::unescape(&found.delim, raw),
        Ok(body.to_owned())
    );
}

#[test]
fn inline_breaks_its_lines_as_the_host_does() {
    let src = step("s", "    check = \"bash x.sh\"\n").replace('\n', "\r\n");
    let loads = PklHost
        .loads(&src)
        .unwrap_or_else(|e| panic!("loads failed: {e}"));
    let out = PklHost
        .inline(&src, nth(&loads, 0), "a\n")
        .unwrap_or_else(|e| panic!("inline failed: {e}"));
    assert_eq!(
        out,
        step("s", "    check = \"\"\"\n      a\n\n      \"\"\"\n").replace('\n', "\r\n")
    );
}

#[test]
fn rewrite_refuses_a_site_with_holes() {
    let src = check_script("      echo \\(x)\n");
    let found = only_site(&src);
    assert_eq!(found.holes.len(), 1);
    assert_eq!(
        PklHost.rewrite(
            &src,
            &found,
            &invoke(&["bash", "x.sh"]),
            Path::new("hk.pkl")
        ),
        Err(Error::unsupported(
            LangId::Pkl,
            "rewrite of a string with holes"
        ))
    );
}

#[test]
fn escape_is_the_string_modules_under_the_sites_pounds() {
    let found = only_site(&check_script("      echo hi\n"));
    assert_eq!(
        PklHost.escape(&found.delim, "a\\b"),
        Ok("\na\\\\b\n".to_owned())
    );
}

#[test]
fn inline_refuses_a_load_that_is_not_in_the_source() {
    let src = step("s", "    check = \"bash x.sh\"\n");
    let stale = LoadRef {
        span: Span::new(0, 5),
        path: PathBuf::from("x.sh"),
        guest: LangId::Shell,
    };
    let err = PklHost.inline(&src, &stale, "true");
    assert!(
        matches!(
            err,
            Err(Error::Parse {
                lang: LangId::Pkl,
                ..
            })
        ),
        "{err:?}"
    );
}

// --- unescape (`languages/api/src/lens:V39`) ---------------------------

#[test]
fn unescape_is_the_string_modules_for_a_site_body() {
    // The trait method hands the site's delimiter and raw body to
    // `string::unescape`: the closing line's indent goes, escapes decode.
    let src = check_script("        a \\t b\n");
    let found = only_site(&src);
    let raw = found
        .delim
        .body
        .of(&src)
        .unwrap_or_else(|| panic!("no body"));
    assert_eq!(
        PklHost.unescape(&found.delim, raw),
        Ok("  a \t b".to_owned())
    );
}

#[test]
fn unescape_refuses_a_delimiter_pkl_does_not_write() {
    let nix = Delim {
        kind: DelimKind::NixIndented,
        open: Span::new(0, 0),
        body: Span::new(0, 0),
        close: Span::new(0, 0),
    };
    assert_eq!(
        PklHost.unescape(&nix, "x"),
        Err(Error::unsupported(LangId::Pkl, "unescape"))
    );
}
