//! The round trip over xml (`languages/api/src/lens:V39`).
//!
//! `escape` is `unescape` read backwards, checked by the api's
//! [`lens::escape_law`] over every site of every fixture -- LF and CRLF
//! alike -- and over hand vectors. The rest of the lens,
//! `rewrite`/`inline` (`languages/api/src/lens:V34`), is refused until
//! `languages/data/xml:T190` decides the load, so no law is claimed for
//! it: a law over a refusal would pass vacuously.

use std::path::{Path, PathBuf};

use xenolith_lang_api::{Delim, DelimKind, Host, Span, lens};
use xenolith_lang_xml::XmlHost;

/// Every text fixture, as `(name, source)`, each also saved with CRLF.
fn sources() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&root)
        .unwrap_or_else(|e| panic!("{}: {e}", root.display()))
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .flat_map(|dir| [dir.join("input.plist"), dir.join("input.xml")])
        .filter(|path| path.is_file())
        .collect();
    files.sort();
    files
        .into_iter()
        // The binary plist is not text and never reaches a host.
        .filter_map(|path| {
            Some((
                path.display().to_string(),
                std::fs::read_to_string(path).ok()?,
            ))
        })
        .flat_map(|(name, src)| {
            [
                (format!("{name} (crlf)"), src.replace('\n', "\r\n")),
                (name, src),
            ]
        })
        .collect()
}

fn argv() -> Delim {
    Delim {
        kind: DelimKind::ArgvString,
        open: Span::new(0, 0),
        body: Span::new(0, 0),
        close: Span::new(0, 0),
    }
}

#[test]
fn the_escape_law_holds_for_every_fixture_site() {
    let mut checked = 0;
    let mut failures = Vec::new();
    for (name, src) in sources() {
        let sites = XmlHost
            .sites(&src)
            .unwrap_or_else(|e| panic!("{name}: sites failed: {e}"));
        for site in sites {
            checked += 1;
            let Some(raw) = site.delim.body.of(&src) else {
                failures.push(format!("{name}: body span does not fit"));
                continue;
            };
            if let Err(e) = lens::escape_law(&XmlHost, &site.delim, raw) {
                failures.push(format!("{name} {}: {e}", site.sink));
            }
        }
    }
    assert!(checked > 0, "no fixture site -- nothing was checked");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Raw texts the fixtures do not show: each entity, both reference
/// bases, CDATA holding markup, literal and referenced carriage returns.
const RAWS: &[&str] = &[
    "",
    "echo hi",
    "a &amp;&amp; b &lt; c &gt; d",
    "&quot;q&quot; &apos;a&apos;",
    "&#65;&#x42;&#13;",
    "<![CDATA[a && b < c]]>tail",
    "cr\r\nlf\rlone&#xD;",
    "\ttab and zażółć",
];

#[test]
fn the_escape_law_holds_for_hand_vectors() {
    for raw in RAWS {
        if let Err(e) = lens::escape_law(&XmlHost, &argv(), raw) {
            panic!("{e}");
        }
    }
}
