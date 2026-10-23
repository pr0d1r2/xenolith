//! Unit tests for `syntax.rs` (`src:C139`): what Pkl accepts (pklr, which
//! agrees with Pkl 0.31.1) and the object tree the host reads, every span
//! exact.

use xenolith_lang_api::{Error, LangId, Span};

use super::{Body, Member, Module, Str, Value, parse};

fn module(src: &str) -> Module {
    parse(src).unwrap_or_else(|e| panic!("{src:?}: {e}"))
}

fn of(span: Span, src: &str) -> &str {
    span.of(src).unwrap_or_else(|| panic!("{span:?} outside {src:?}"))
}

/// The value of the property `name` directly in `body`.
fn property<'b>(body: &'b Body, name: &str) -> &'b Value {
    body.members
        .iter()
        .find_map(|m| match m {
            Member::Property(p) if p.name == name => Some(&p.value),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no property {name} in {body:#?}"))
}

/// The body of the entry keyed `key` directly in `body`.
fn entry<'b>(body: &'b Body, key: &str) -> &'b Body {
    body.members
        .iter()
        .find_map(|m| match m {
            Member::Entry(e) if e.key.as_deref() == Some(key) => match &e.value {
                Value::Object(b) => Some(b),
                _ => None,
            },
            _ => None,
        })
        .unwrap_or_else(|| panic!("no entry {key} in {body:#?}"))
}

fn object(value: &Value) -> &Body {
    match value {
        Value::Object(b) => b,
        other => panic!("not an object: {other:#?}"),
    }
}

fn string(value: &Value) -> &Str {
    match value {
        Value::Str(s) => s,
        other => panic!("not a string: {other:#?}"),
    }
}

const HK: &str = concat!(
    "amends \"pkl/Config.pkl\"\n",
    "local shell = \"bash -c\"\n",
    "hooks {\n",
    "  [\"pre-commit\"] {\n",
    "    steps {\n",
    "      [\"lint\"] {\n",
    "        check = \"\"\"\n",
    "          a | b\n",
    "          \"\"\"\n",
    "        fix = \"sh scripts/hk/lint.sh {{files}}\"\n",
    "      }\n",
    "      [\"next\"] { check = \"ls\" }\n",
    "    }\n",
    "  }\n",
    "}\n",
);

#[test]
fn the_module_header_names_what_it_amends() {
    assert_eq!(module(HK).amends.as_deref(), Some("pkl/Config.pkl"));
    assert_eq!(module("x = 1\n").amends, None);
}

#[test]
fn properties_entries_and_objects_nest_as_written() {
    let m = module(HK);
    let hooks = object(property(&m.body, "hooks"));
    let steps = object(property(entry(hooks, "pre-commit"), "steps"));
    let lint = entry(steps, "lint");
    let check = string(property(lint, "check"));
    assert!(check.multiline);
    assert_eq!(check.pounds, 0);
    assert_eq!(of(check.open, HK), "\"\"\"");
    assert_eq!(of(check.close, HK), "\"\"\"");
    assert_eq!(of(check.body(), HK), "\n          a | b\n          ");
    let fix = string(property(lint, "fix"));
    assert!(!fix.multiline);
    assert_eq!(fix.plain.as_deref(), Some("sh scripts/hk/lint.sh {{files}}"));
    assert_eq!(of(fix.span, HK), "\"sh scripts/hk/lint.sh {{files}}\"");
    assert_eq!(string(property(entry(steps, "next"), "check")).plain.as_deref(), Some("ls"));
}

#[test]
fn a_local_property_is_marked() {
    let m = module(HK);
    let local = m.body.members.iter().find_map(|mb| match mb {
        Member::Property(p) if p.name == "shell" => Some(p.local),
        _ => None,
    });
    assert_eq!(local, Some(true));
}

#[test]
fn new_and_typed_objects_are_objects() {
    let src = concat!(
        "steps: Mapping<String, Step> = new Mapping<String, Step> {\n",
        "  [\"a\"] = new Step { check = \"x\" }\n",
        "}\n",
    );
    let m = module(src);
    let steps = object(property(&m.body, "steps"));
    let a = steps.members.iter().find_map(|mb| match mb {
        Member::Entry(e) if e.key.as_deref() == Some("a") => Some(object(&e.value)),
        _ => None,
    });
    assert!(a.is_some_and(|b| string(property(b, "check")).plain.as_deref() == Some("x")));
}

#[test]
fn a_custom_delimiter_counts_its_pounds_and_holes_span_their_backslash() {
    let src = "x = #\"\"\"\n  a \\#(y) \\(z) \\#(w.v)\n  \"\"\"#\ny = \"a\\(b)c\"\n";
    let m = module(src);
    let x = string(property(&m.body, "x"));
    assert_eq!(x.pounds, 1);
    assert_eq!(of(x.open, src), "#\"\"\"");
    assert_eq!(of(x.close, src), "\"\"\"#");
    let holes: Vec<&str> = x.holes.iter().map(|h| of(*h, src)).collect();
    assert_eq!(holes, ["\\#(y)", "\\#(w.v)"]);
    let y = string(property(&m.body, "y"));
    assert_eq!(y.plain, None);
    assert_eq!(y.holes.iter().map(|h| of(*h, src)).collect::<Vec<_>>(), ["\\(b)"]);
}

#[test]
fn an_escape_or_an_empty_string_is_not_plain() {
    let m = module("a = \"x\\ty\"\nb = \"\"\nc = #\"x\\y\"#\n");
    assert_eq!(string(property(&m.body, "a")).plain, None);
    assert_eq!(string(property(&m.body, "b")).plain, None);
    assert_eq!(string(property(&m.body, "c")).plain.as_deref(), Some("x\\y"));
}

#[test]
fn a_string_joined_to_more_is_no_string_value() {
    let m = module("a = \"x\" + \"y\"\nb = \"x\".length\nc = \"z\"\n[\"k\"] = 1\n");
    assert!(matches!(property(&m.body, "a"), Value::Other(_)));
    assert!(matches!(property(&m.body, "b"), Value::Other(_)));
    assert!(matches!(property(&m.body, "c"), Value::Str(_)));
}

#[test]
fn objects_inside_expressions_and_generators_are_still_reached() {
    let src = "local x = if (true) new { [\"a\"] { check = \"\"\"\n  q\n  \"\"\" } } else null\n\
               for (k in List(1)) { [\"b\"] { check = \"r\" } }\n";
    let m = module(src);
    let mut keys = Vec::new();
    m.body.visit(&mut |body| {
        for member in &body.members {
            if let Member::Entry(e) = member {
                keys.extend(e.key.clone());
            }
        }
    });
    keys.sort();
    assert_eq!(keys, ["a", "b"]);
}

#[test]
fn what_pkl_rejects_is_a_parse_error() {
    for src in [
        "hooks {\n  /// dangling\n  [\"a\"] {}\n}\n",
        "x = \"open\n",
        "x = {\n",
        "x = 1 +\n",
    ] {
        match parse(src) {
            Err(Error::Parse { lang, .. }) => assert_eq!(lang, LangId::Pkl),
            other => panic!("{src:?}: expected a parse error, got {other:?}"),
        }
    }
}
