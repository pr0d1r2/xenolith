//! The round trip over pkl (`languages/api/src/lens:V34`,
//! `languages/api/src/lens:V39`, `languages/pkl:V171`).
//!
//! `escape` is `unescape` read backwards under the delimiter's own `#`
//! count, checked by the api's [`lens::escape_law`] over every site of
//! every fixture -- LF and CRLF alike (`languages/pkl:B1`) -- and over
//! hand vectors. `inline` writes the host's own line break
//! (`languages/pkl:B2`), and `rewrite` refuses a site with holes.

use std::path::{Path, PathBuf};

use xenolith_lang_api::{Delim, DelimKind, Error, Host, Invoke, LangId, Site, Span, lens};
use xenolith_lang_pkl::PklHost;

const PKL: PklHost = PklHost;

/// Every fixture case, as `(name, source)`, each also saved with CRLF.
fn sources() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&root)
        .unwrap_or_else(|e| panic!("{}: {e}", root.display()))
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();
    let mut out = Vec::new();
    for dir in dirs {
        let path = dir.join("input.pkl");
        let src =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        out.push((
            format!("{} (crlf)", dir.display()),
            src.replace('\n', "\r\n"),
        ));
        out.push((dir.display().to_string(), src));
    }
    out
}

fn sites(src: &str) -> Vec<Site> {
    PKL.sites(src)
        .unwrap_or_else(|e| panic!("sites failed: {e}"))
}

fn raw<'s>(src: &'s str, site: &Site) -> &'s str {
    site.delim
        .body
        .of(src)
        .unwrap_or_else(|| panic!("{}: body span does not fit", site.sink))
}

fn pkl(pounds: usize) -> Delim {
    Delim {
        kind: DelimKind::PklMultiline { pounds },
        open: Span::new(0, 0),
        body: Span::new(0, 0),
        close: Span::new(0, 0),
    }
}

/// A hk config whose one step's `check` is `literal`.
fn hk(literal: &str) -> String {
    format!(
        "amends \"pkl/Config.pkl\"\n\nhooks {{\n  [\"pre-commit\"] {{\n    steps {{\n      \
         [\"x\"] {{\n        check = {literal}\n      }}\n    }}\n  }}\n}}\n"
    )
}

// --- escape (`languages/api/src/lens:V39`) -----------------------------

#[test]
fn the_escape_law_holds_for_every_fixture_site() {
    let mut checked = 0;
    let mut failures = Vec::new();
    for (name, src) in sources() {
        for site in sites(&src) {
            checked += 1;
            if let Err(e) = lens::escape_law(&PKL, &site.delim, raw(&src, &site)) {
                failures.push(format!("{name} {}: {e}", site.sink));
            }
        }
    }
    assert!(checked > 0, "no fixture site -- nothing was checked");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Bodies the fixtures do not show: backslashes and quote runs that
/// clash with each `#` count, interpolation look-alikes, carriage
/// returns, whitespace-only lines, nothing.
const BODIES: &[&str] = &[
    "",
    "\n",
    "echo hi",
    "echo hi\n",
    "a\n\n  b\n\t\n",
    "sed 's/\\t/ /g' \"$f\"",
    "a \\# b \\## c \\(x) \\#(y)",
    "\"\" \"\"\" \"\"\"\" \"\"\"\"\" \"\"\"# \"\"\"##",
    "ends with a quote \"",
    "\\",
    "\\u{41}",
    "cr\r\nlf\rlone",
    "unicode: zażółć",
];

#[test]
fn escape_then_unescape_is_the_identity_under_every_pound_count() {
    for pounds in 0..3 {
        for body in BODIES {
            let written = PKL
                .escape(&pkl(pounds), body)
                .unwrap_or_else(|e| panic!("escape {body:?}: {e}"));
            let back = PKL.unescape(&pkl(pounds), &written);
            assert_eq!(back.as_deref(), Ok(*body), "{pounds} #: {written:?}");
        }
    }
}

#[test]
fn an_escaped_body_parses_back_as_one_site_of_that_body() {
    // The grammar is the judge of where the literal ends
    // (`languages/api/src/site:V38`).
    for pounds in 0..3 {
        let guard = "#".repeat(pounds);
        for body in BODIES {
            let written = PKL
                .escape(&pkl(pounds), body)
                .unwrap_or_else(|e| panic!("escape {body:?}: {e}"));
            let src = hk(&format!("{guard}\"\"\"{written}\"\"\"{guard}"));
            let found = sites(&src);
            let [site] = found.as_slice() else {
                panic!("{src:?}: expected one site, got {found:#?}");
            };
            assert_eq!(
                site.delim.kind,
                DelimKind::PklMultiline { pounds },
                "{src:?}"
            );
            assert!(site.holes.is_empty(), "{src:?}: escape left a hole");
            assert_eq!(raw(&src, site), written, "{src:?}");
        }
    }
}

#[test]
fn escape_refuses_a_delimiter_that_is_not_pkl() {
    let mut delim = pkl(0);
    delim.kind = DelimKind::NixIndented;
    assert_eq!(
        PKL.escape(&delim, "x"),
        Err(Error::unsupported(LangId::Pkl, "escape"))
    );
}

// --- inline keeps the host's line breaks (`languages/pkl:B2`) -----------

#[test]
fn inline_into_a_crlf_config_writes_crlf() {
    let src = hk("\"bash scripts/hk/x.sh {{files}}\"").replace('\n', "\r\n");
    let found = PKL
        .loads(&src)
        .unwrap_or_else(|e| panic!("loads failed: {e}"));
    let [load] = found.as_slice() else {
        panic!("expected one load, got {found:#?}");
    };
    let body = "if a; then\n  b \\t\nfi\n";
    let inlined = PKL
        .inline(&src, load, body)
        .unwrap_or_else(|e| panic!("inline failed: {e}"));
    let bare = inlined.replace("\r\n", "");
    assert!(!bare.contains('\n'), "an LF line break in {inlined:?}");
    let back = sites(&inlined);
    let [site] = back.as_slice() else {
        panic!("expected one site, got {back:#?}");
    };
    assert_eq!(
        PKL.unescape(&site.delim, raw(&inlined, site)).as_deref(),
        Ok(body)
    );
}

#[test]
fn inline_into_an_lf_config_stays_lf() {
    let src = hk("\"bash scripts/hk/x.sh {{files}}\"");
    let found = PKL
        .loads(&src)
        .unwrap_or_else(|e| panic!("loads failed: {e}"));
    let [load] = found.as_slice() else {
        panic!("expected one load, got {found:#?}");
    };
    let inlined = PKL
        .inline(&src, load, "a\nb\n")
        .unwrap_or_else(|e| panic!("inline failed: {e}"));
    assert!(!inlined.contains('\r'), "{inlined:?}");
}

#[test]
fn inline_keeps_a_carriage_return_in_the_body() {
    // A raw CR in a pkl string reads as a line break (`languages/pkl:B1`),
    // so the body's own CR must be written as the escape.
    let src = hk("\"bash scripts/hk/x.sh {{files}}\"");
    let found = PKL
        .loads(&src)
        .unwrap_or_else(|e| panic!("loads failed: {e}"));
    let [load] = found.as_slice() else {
        panic!("expected one load, got {found:#?}");
    };
    let body = "printf 'a\rb'\n";
    let inlined = PKL
        .inline(&src, load, body)
        .unwrap_or_else(|e| panic!("inline failed: {e}"));
    let back = sites(&inlined);
    let [site] = back.as_slice() else {
        panic!("expected one site, got {back:#?}");
    };
    assert_eq!(
        PKL.unescape(&site.delim, raw(&inlined, site)).as_deref(),
        Ok(body)
    );
}

// --- rewrite refuses holes (`languages/pkl:V171`) ----------------------

#[test]
fn rewrite_refuses_a_site_with_holes() {
    // `\#(tool)` is pkl; in the script it would be text, and the step
    // would run `\#(tool) --diff` instead of `shfmt --diff`.
    for (name, src) in sources() {
        for site in sites(&src).iter().filter(|site| !site.holes.is_empty()) {
            let invoke = Invoke {
                argv: vec!["bash".to_owned(), "scripts/hk/x.sh".to_owned()],
            };
            assert_eq!(
                PKL.rewrite(&src, site, &invoke, Path::new("scripts/hk/x.sh")),
                Err(Error::unsupported(
                    LangId::Pkl,
                    "rewrite of a string with holes"
                )),
                "{name} {}",
                site.sink
            );
        }
    }
}

// --- the lens laws, LF and CRLF (`languages/api/src/lens:V34`) ----------

#[test]
fn the_lens_laws_hold_for_every_fixture_site_without_holes() {
    let normalized = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut checked = 0;
    for (name, src) in sources() {
        for site in sites(&src).iter().filter(|site| site.holes.is_empty()) {
            checked += 1;
            let path = PathBuf::from(format!("scripts/hk/site-{checked}.sh"));
            let invoke = Invoke {
                argv: vec!["bash".to_owned(), path.display().to_string()],
            };
            let rewritten = PKL
                .rewrite(&src, site, &invoke, &path)
                .unwrap_or_else(|e| panic!("{name} {}: rewrite failed: {e}", site.sink));
            // (b) the site is gone, (c) the load is there.
            assert_eq!(sites(&rewritten).len() + 1, sites(&src).len(), "{name}");
            let loads = PKL
                .loads(&rewritten)
                .unwrap_or_else(|e| panic!("{name}: loads failed: {e}"));
            let Some(load) = loads.iter().find(|load| load.path == path) else {
                panic!("{name} {}: no load of {}", site.sink, path.display());
            };
            // (a) inline puts the body back, in the host's own line breaks.
            let body = PKL
                .unescape(&site.delim, raw(&src, site))
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            let inlined = PKL
                .inline(&rewritten, load, &body)
                .unwrap_or_else(|e| panic!("{name} {}: inline failed: {e}", site.sink));
            assert_eq!(
                normalized(&inlined),
                normalized(&src),
                "{name} {}",
                site.sink
            );
            let crlf = src.contains("\r\n");
            assert_eq!(inlined.replace("\r\n", "").contains('\n'), !crlf, "{name}");
        }
    }
    assert!(checked >= 6, "only {checked} sites checked");
}
