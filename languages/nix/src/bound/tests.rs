//! Unit tests for `bound` (`src:C139`): holes carried as text through
//! `builtins.replaceStrings` (`languages/nix:V174`).

use xenolith_lang_api::holes::{self, Hole};
use xenolith_lang_api::{Delim, DelimKind, Error, GuestEnv, LangId, Site, Span};

use super::{body, call, pattern, replace_strings};

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

/// The one string of `src`, which starts with its opening delimiter and
/// ends with its closing one, as a site, holes at every `${…}` the test
/// names by text.
fn site(src: &str, kind: DelimKind, holes: &[&str]) -> Site {
    let quote = if kind == DelimKind::NixIndented { 2 } else { 1 };
    let mut spans = Vec::new();
    for hole in holes {
        let mut from = 0;
        while let Some(at) = src.get(from..).and_then(|rest| rest.find(hole)) {
            spans.push(Span::new(from + at, from + at + hole.len()));
            from += at + hole.len();
        }
    }
    spans.sort();
    Site {
        sink: "script".to_owned(),
        guest: LangId::Shell,
        env: GuestEnv::default(),
        delim: Delim {
            kind,
            open: Span::new(0, quote),
            body: Span::new(quote, src.len() - quote),
            close: Span::new(src.len() - quote, src.len()),
        },
        holes: spans,
    }
}

/// `body` over the one string of `src`, each hole named by `names`.
fn body_of(src: &str, kind: DelimKind, holes: &[&str], names: &[&str]) -> Result<String, Error> {
    let site = site(src, kind, holes);
    let found: Vec<Hole> = holes::collect(src, &site);
    body(src, &site.delim, &found, &strings(names))
}

fn refused() -> Result<String, Error> {
    Err(Error::unsupported(
        LangId::Nix,
        "rewrite of holes `replaceStrings` would not put back",
    ))
}

// --- replace_strings ---------------------------------------------------

#[test]
fn replace_strings_scans_left_to_right_and_never_rescans_what_it_wrote() {
    // nix's own `builtins.replaceStrings`: at each position the first
    // pattern in list order that matches wins, and its replacement is
    // not looked at again.
    let from = strings(&["__A__", "__B__"]);
    let to = strings(&["1", "__A__"]);
    assert_eq!(
        replace_strings("__A__ x __B____A__", &from, &to),
        "1 x __A__1"
    );
    let from = strings(&["aa", "a"]);
    assert_eq!(replace_strings("aaa", &from, &strings(&["X", "Y"])), "XY");
    let from = strings(&["__A__"]);
    assert_eq!(replace_strings("", &from, &strings(&["1"])), "");
    assert_eq!(
        replace_strings("zażółć __A__", &from, &strings(&["1"])),
        "zażółć 1"
    );
    assert_eq!(replace_strings("@A", &from, &strings(&["1"])), "@A");
}

// --- pattern -----------------------------------------------------------

#[test]
fn a_pattern_is_the_name_between_at_signs_and_only_a_plain_name() {
    // A name goes inside a `"…"` nix string as it is, so it may hold
    // nothing that string would read as an escape or a hole.
    assert_eq!(pattern("FOO_BIN"), Some("__FOO_BIN__".to_owned()));
    assert_eq!(pattern("x9"), Some("__x9__".to_owned()));
    for bad in ["", "A-B", "A B", "A\"", "${x}", "É", "A@B"] {
        assert_eq!(pattern(bad), None, "{bad:?}");
    }
}

// --- call --------------------------------------------------------------

#[test]
fn the_call_lists_the_patterns_then_the_holes_then_the_load() {
    let pairs = [
        ("__A__".to_owned(), "${a}".to_owned()),
        ("__B_BIN__".to_owned(), "${pkgs.b}/bin/b".to_owned()),
    ];
    assert_eq!(
        call(&pairs, "(builtins.readFile ./x.sh)"),
        "builtins.replaceStrings [ \"__A__\" \"__B_BIN__\" ] [ \"${a}\" \"${pkgs.b}/bin/b\" ] \
         (builtins.readFile ./x.sh)"
    );
}

// --- body --------------------------------------------------------------

#[test]
fn the_body_holds_a_pattern_wherever_a_hole_and_its_tail_were() {
    let src = "''\n  ${a}/bin/a --x\n  echo ${a}/bin/a; ${b}\n''";
    assert_eq!(
        body_of(
            src,
            DelimKind::NixIndented,
            &["${a}", "${b}"],
            &["A_BIN", "B"]
        ),
        Ok("__A_BIN__ --x\necho __A_BIN__; __B__\n".to_owned())
    );
}

#[test]
fn a_double_quoted_body_holds_patterns_too() {
    let src = "\"mkdir -p ${cfg.dir} && echo \\\"${cfg.dir}\\\"\"";
    assert_eq!(
        body_of(src, DelimKind::NixString, &["${cfg.dir}"], &["DIR"]),
        Ok("mkdir -p __DIR__ && echo \"__DIR__\"".to_owned())
    );
}

#[test]
fn escapes_and_indent_around_a_hole_read_back_as_nix_reads_them() {
    // `''${` is a literal `${`, `'''` a literal `''`, and a line led by a
    // hole still counts toward the common indent.
    let src = "''\n    ${a} ''${HOME} '''\n      ${a}\n  ''";
    assert_eq!(
        body_of(src, DelimKind::NixIndented, &["${a}"], &["A"]),
        Ok("__A__ ${HOME} ''\n  __A__\n".to_owned())
    );
}

#[test]
fn a_body_already_spelling_a_pattern_is_refused() {
    // `replaceStrings` would fill the author's `__A__` too, and the script
    // would change.
    let src = "''\n  echo __A__ ${a}\n''";
    assert_eq!(
        body_of(src, DelimKind::NixIndented, &["${a}"], &["A"]),
        refused()
    );
    let glued = "''\n  echo __A${a}__\n''";
    assert_eq!(
        body_of(glued, DelimKind::NixIndented, &["${a}"], &["A"]),
        refused()
    );
}

#[test]
fn a_name_that_is_no_plain_word_is_refused() {
    let src = "''\n  echo ${a}\n  b\n''";
    assert_eq!(
        body_of(src, DelimKind::NixIndented, &["${a}"], &["A-B"]),
        Err(Error::unsupported(
            LangId::Nix,
            "rewrite of a hole whose param name is no plain word"
        ))
    );
}
