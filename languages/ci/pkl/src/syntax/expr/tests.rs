//! Unit tests for `expr.rs` (`src:C139`): where an expression ends, and
//! that the objects inside expressions, types and other members are kept.

use super::super::{Body, Member, Value, parse};

fn body(src: &str) -> Body {
    parse(src).unwrap_or_else(|e| panic!("{src:?}: {e}")).body
}

/// Each top-level property's name and value kind, in order.
fn kinds(src: &str) -> Vec<(String, &'static str)> {
    body(src)
        .members
        .into_iter()
        .filter_map(|m| match m {
            Member::Property(p) => Some((
                p.name,
                match p.value {
                    Value::Str(_) => "str",
                    Value::Object(_) => "object",
                    Value::New(_) => "new",
                    Value::Other(_) => "other",
                },
            )),
            _ => None,
        })
        .collect()
}

/// Every entry key anywhere in `src`.
fn keys(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    body(src).visit(&mut |b| {
        for member in &b.members {
            if let Member::Entry(e) = member {
                out.extend(e.key.clone());
            }
        }
    });
    out
}

fn named(items: &[(&str, &'static str)]) -> Vec<(String, &'static str)> {
    items.iter().map(|&(n, k)| (n.to_owned(), k)).collect()
}

#[test]
fn operators_and_postfixes_continue_an_expression() {
    assert_eq!(
        kinds(concat!(
            "a = \"x\" + \"y\"\n",
            "b = \"x\".length\n",
            "c = \"x\" == \"y\" && true || false\n",
            "d = if (true) \"x\" else \"y\"\n",
            "e = (n) -> n + 1\n",
            "f = \"x\" as String\n",
            "g = null ?? \"y\"\n",
            "h = List(1, 2)[0]\n",
            "i = \"plain\"\n",
        )),
        named(&[
            ("a", "other"),
            ("b", "other"),
            ("c", "other"),
            ("d", "other"),
            ("e", "other"),
            ("f", "other"),
            ("g", "other"),
            ("h", "other"),
            ("i", "str"),
        ])
    );
}

#[test]
fn a_bracket_on_the_next_line_starts_a_new_member() {
    let src = "o {\n  a = \"x\"\n  [\"k\"] { check = \"y\" }\n  b = \"z\"\n}\n";
    assert_eq!(keys(src), ["k"]);
}

#[test]
fn objects_inside_calls_lets_and_amends_are_kept() {
    let src = concat!(
        "a = List(new Dynamic { [\"in-call\"] {} })\n",
        "b = let (x = new Dynamic { [\"in-let\"] {} }) x\n",
        "c = (new Dynamic {}) { [\"amended\"] {} }\n",
        "d = new Mapping<String, Listing<Int>> { [\"typed\"] {} }\n",
    );
    let mut got = keys(src);
    got.sort();
    assert_eq!(got, ["amended", "in-call", "in-let", "typed"]);
}

#[test]
fn types_are_skipped_whatever_their_shape() {
    assert_eq!(
        kinds(concat!(
            "a: String? = \"x\"\n",
            "b: List<Int>|Null = null\n",
            "c: (Int) -> Int = (n) -> n\n",
            "d: \"a\"|\"b\" = \"a\"\n",
        )),
        named(&[("a", "str"), ("b", "other"), ("c", "other"), ("d", "str")])
    );
}

#[test]
fn other_members_are_read_past_with_their_objects() {
    let src = concat!(
        "import \"pkl:math\" as m\n",
        "typealias Name = String\n",
        "function f(x: Int): Int = x + 1\n",
        "class C { o = new Dynamic { [\"in-class\"] {} } }\n",
        "@Deprecated\n",
        "x = \"after\"\n",
        "o {\n",
        "  ...List(1)\n",
        "  [[true]] { [\"predicate\"] {} }\n",
        "  when (true) { [\"when\"] {} } else { [\"else\"] {} }\n",
        "  for (v in List(1)) { [\"for\"] {} }\n",
        "  \"element\"\n",
        "}\n",
    );
    let mut got = keys(src);
    got.sort();
    assert_eq!(got, ["else", "for", "in-class", "predicate", "when"]);
    assert!(kinds(src).contains(&("x".to_owned(), "str")));
}

#[test]
fn prefixes_postfixes_and_keyword_calls_are_read_past() {
    assert_eq!(
        kinds(concat!(
            "a = !true\n",
            "b = -1\n",
            "c = y!!\n",
            "d = read(\"env:HOME\")\n",
            "e = throw(\"no\")\n",
            "f = new Dynamic { [\"n\"] {} }.toMap()\n",
            "g = new Listing<String?> {}\n",
            "h = \"last\"\n",
        )),
        named(&[
            ("a", "other"),
            ("b", "other"),
            ("c", "other"),
            ("d", "other"),
            ("e", "other"),
            ("f", "other"),
            ("g", "new"),
            ("h", "str"),
        ])
    );
    assert_eq!(keys("f = new Dynamic { [\"n\"] {} }.toMap()\n"), ["n"]);
}

#[test]
fn keys_with_brackets_or_objects_inside_are_skipped_whole() {
    let src = concat!(
        "o {\n",
        "  [List(1)[0]] = 1\n",
        "  [new Dynamic { [\"in-key\"] {} }] = 2\n",
        "  [\"after\"] {}\n",
        "}\n",
    );
    let mut got = keys(src);
    got.sort();
    assert_eq!(got, ["after", "in-key"]);
}

#[test]
fn a_predicate_takes_any_value_and_an_annotation_may_be_dotted_with_a_body() {
    let src = concat!(
        "o {\n",
        "  [[true]] = 1\n",
        "  [[false]] = \"s\"\n",
        "  [\"after\"] {}\n",
        "}\n",
        "@a.b.Deprecated { message = \"x\" }\n",
        "x = \"after\"\n",
    );
    assert_eq!(keys(src), ["after"]);
    assert!(kinds(src).contains(&("x".to_owned(), "str")));
}
