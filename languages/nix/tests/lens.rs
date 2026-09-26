//! The round trip over nix (`languages/api/src/lens:V34`,
//! `languages/api/src/lens:V39`, `languages/nix:V170`).
//!
//! `escape` is `unescape` read backwards: every body a nix string can
//! hold must go back between the same delimiters and read the same. The
//! law is the api's own [`lens::escape_law`], run over every site of
//! every fixture under `tests/fixtures/`, and over hand vectors for the
//! corners a fixture does not reach -- quotes next to escapes, a body
//! whose every line is indented, a last line of spaces.

use std::fs;
use std::path::{Path, PathBuf};

use xenolith_lang_api::{Delim, DelimKind, Error, Host, LangId, Site, Span, lens};
use xenolith_lang_nix::NixHost;

const NIX: NixHost = NixHost;

/// Every fixture case directory, sorted.
fn cases() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut dirs: Vec<PathBuf> = fs::read_dir(&root)
        .unwrap_or_else(|e| panic!("{}: {e}", root.display()))
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();
    dirs
}

/// A case's `input.nix` and the sites the host reports in it.
fn fixture(case: &Path) -> (String, Vec<Site>) {
    let path = case.join("input.nix");
    let src = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let sites = NIX
        .sites(&src)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    (src, sites)
}

/// The raw text between a site's delimiters.
fn raw<'s>(src: &'s str, site: &Site) -> &'s str {
    site.delim
        .body
        .of(src)
        .unwrap_or_else(|| panic!("{}: body span does not fit", site.sink))
}

/// A delimiter of `kind`; `escape` and `unescape` read only the kind.
fn delim(kind: DelimKind) -> Delim {
    Delim {
        kind,
        open: Span::new(0, 0),
        body: Span::new(0, 0),
        close: Span::new(0, 0),
    }
}

fn escape(kind: DelimKind, body: &str) -> String {
    NIX.escape(&delim(kind), body)
        .unwrap_or_else(|e| panic!("escape {body:?}: {e}"))
}

// --- escape (`languages/api/src/lens:V39`) -----------------------------

#[test]
fn the_escape_law_holds_for_every_fixture_site() {
    let mut checked = 0;
    let mut failures = Vec::new();
    for case in cases() {
        let (src, sites) = fixture(&case);
        for site in &sites {
            checked += 1;
            if let Err(e) = lens::escape_law(&NIX, &site.delim, raw(&src, site)) {
                failures.push(format!("{} {}: {e}", case.display(), site.sink));
            }
        }
    }
    assert!(checked > 0, "no fixture site -- nothing was checked");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Bodies a nix string can hold that the fixtures do not show: quotes
/// beside escapes, `${` and `$${`, a trailing `'`, carriage returns, a
/// body whose every line is indented, a last line of spaces, nothing.
const BODIES: &[&str] = &[
    "",
    "\n",
    "echo hi",
    "echo hi\n",
    "a\nb",
    "a\n\nb\n",
    "  indented\n  all\n",
    "  a\n    b",
    "   ",
    "a\n  ",
    "\n\n  ",
    "\ttab\n  space\n",
    "it's",
    "ends with '",
    "''",
    "'''",
    "''''",
    "'${x}",
    "'''${x}",
    "echo ${HOME} $${x} $$ $",
    "a$",
    "printf '%s\\n' \"$1\" \\",
    "''\\n not an escape once written",
    "cr\r\nlf",
    "quote \" and backslash \\ and ${a}",
    "unicode: zażółć",
];

#[test]
fn escape_then_unescape_is_the_identity_for_both_delimiters() {
    for kind in [DelimKind::NixIndented, DelimKind::NixString] {
        for body in BODIES {
            let raw = escape(kind.clone(), body);
            let back = NIX
                .unescape(&delim(kind.clone()), &raw)
                .unwrap_or_else(|e| panic!("{kind:?} {raw:?}: {e}"));
            assert_eq!(&back, body, "{kind:?}: escape gave {raw:?}");
        }
    }
}

#[test]
fn an_escaped_body_parses_back_as_one_string_of_that_body() {
    // The unescape above is this crate's model of nix; the grammar is
    // the judge of whether the text is a nix string at all, and of
    // where it ends (`languages/api/src/site:V38`).
    for (kind, open, close) in [
        (DelimKind::NixIndented, "''", "''"),
        (DelimKind::NixString, "\"", "\""),
    ] {
        for body in BODIES {
            let raw = escape(kind.clone(), body);
            let src = format!("{{ shellHook = {open}{raw}{close}; }}");
            let sites = NIX
                .sites(&src)
                .unwrap_or_else(|e| panic!("{src:?} did not parse: {e}"));
            let [site] = sites.as_slice() else {
                panic!("{src:?}: expected one site, got {sites:#?}");
            };
            assert_eq!(site.delim.kind, kind, "{src:?}");
            assert!(site.holes.is_empty(), "{src:?}: escape left a hole");
            assert_eq!(site.delim.body.of(&src), Some(raw.as_str()), "{src:?}");
        }
    }
}

#[test]
fn the_escape_law_holds_for_raw_escapes_nix_writes() {
    let raws = [
        "\n    a ''${b}\n    ''\\tc\n  ",
        "'''q''' ''$x",
        "\n  ''\\ lead\n    more\n",
        "  a\n  b",
    ];
    for raw in raws {
        let found = lens::escape_law(&NIX, &delim(DelimKind::NixIndented), raw);
        assert_eq!(found, Ok(()), "{raw:?}");
    }
    let found = lens::escape_law(&NIX, &delim(DelimKind::NixString), r#"a\"b\${c}\\"#);
    assert_eq!(found, Ok(()));
}

#[test]
fn escape_refuses_a_delimiter_nix_does_not_write() {
    assert_eq!(
        NIX.escape(&delim(DelimKind::PklMultiline { pounds: 0 }), "x"),
        Err(Error::unsupported(LangId::Nix, "escape"))
    );
}
