//! The round trip over just (`languages/api/src/lens:V34`,
//! `languages/api/src/lens:V39`, `languages/ci/just:V180`).
//!
//! Over every site of every fixture: the escape law, and -- for each site
//! the extract direction does not refuse -- laws (a) to (c): inlining the
//! merged body at the load gives the host back (whitespace normalised,
//! `src/extract:V4`), the site is gone, and the load reads back.

use std::path::{Path, PathBuf};

use xenolith_lang_api::{Host, Invoke, Site, lens};
use xenolith_lang_just::{JustHost, unescape};

const JUST: JustHost = JustHost;

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
        let path = dir.join("input.just");
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
    JUST.sites(src)
        .unwrap_or_else(|e| panic!("sites failed: {e}"))
}

fn raw<'s>(src: &'s str, site: &Site) -> &'s str {
    site.delim
        .body
        .of(src)
        .unwrap_or_else(|| panic!("{}: body span does not fit", site.sink))
}

fn words(text: &str) -> Vec<&str> {
    text.split_whitespace().collect()
}

#[test]
fn the_escape_law_holds_for_every_fixture_site() {
    let mut checked = 0;
    let mut failures = Vec::new();
    for (name, src) in sources() {
        for site in sites(&src) {
            checked += 1;
            if let Err(e) = lens::escape_law(&JUST, &site.delim, raw(&src, &site)) {
                failures.push(format!("{name} {}: {e}", site.sink));
            }
        }
    }
    assert!(checked > 0, "no fixture site -- nothing was checked");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_site_the_extract_direction_moves_round_trips() {
    let mut moved = 0;
    let mut failures = Vec::new();
    for (name, src) in sources() {
        for site in sites(&src) {
            let path = format!("./scripts/just/{}.sh", site.sink);
            let invoke = Invoke {
                argv: vec!["sh".to_owned(), path.clone()],
            };
            let body = unescape(&site.delim, raw(&src, &site)).unwrap_or_default();
            let Ok(done) = JUST.rewrite_bound(&src, &site, &invoke, Path::new(&path), &body, &[])
            else {
                continue;
            };
            let at = format!("{name} {}", site.sink);
            if done.src == src {
                // A body that already IS its load (`pos-loaded`): the
                // rewrite is the identity, as `src/extract:V5` wants.
                continue;
            }
            moved += 1;
            // (c) the load reads back.
            let loads = JUST.loads(&done.src).unwrap_or_default();
            let Some(load) = loads.iter().find(|l| l.path == Path::new(&path)) else {
                failures.push(format!("{at}: (c) no load of {path} in {:?}", done.src));
                continue;
            };
            // (a) inlining the merged body gives the host back.
            match JUST.inline(&done.src, load, &done.body) {
                Ok(back) if words(&back) == words(&src) => {}
                Ok(back) => failures.push(format!("{at}: (a) {back:?} != {src:?}")),
                Err(e) => failures.push(format!("{at}: (a) inline failed: {e}")),
            }
            // (b) the site is gone: no site of that recipe keeps its body.
            let survives = sites(&done.src).iter().any(|s| {
                s.sink == site.sink && s.delim.body.of(&done.src) == Some(raw(&src, &site))
            });
            if survives {
                failures.push(format!("{at}: (b) the site survives the rewrite"));
            }
        }
    }
    assert!(moved > 0, "no fixture site moved -- nothing was checked");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn the_extract_on_disk_inlines_back_through_the_prelude() {
    // `languages/api/src/lens:V63`: the file is `wrap(body, prelude)` and
    // inline reads `strip_strict` of it -- a trailing newline included.
    let src = "a:\n    -rm x\n    ls\n";
    let site = sites(src)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("one site"));
    let invoke = Invoke {
        argv: vec!["sh".to_owned(), "./x.sh".to_owned()],
    };
    let done = JUST
        .rewrite_bound(src, &site, &invoke, Path::new("./x.sh"), "", &[])
        .unwrap_or_else(|e| panic!("{e}"));
    let on_disk = format!("{}\n", done.body);
    let loads = JUST.loads(&done.src).unwrap_or_default();
    let load = loads.first().unwrap_or_else(|| panic!("one load"));
    let back = JUST
        .inline(&done.src, load, &on_disk)
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(back, src);
}

#[test]
fn inline_keeps_a_crlf_host_crlf() {
    let src = "a:\r\n    sh ./x.sh\r\nb:\r\n";
    let loads = JUST.loads(src).unwrap_or_else(|e| panic!("{e}"));
    let load = loads.first().unwrap_or_else(|| panic!("one load"));
    let back = JUST
        .inline(src, load, "one\ntwo || true\n")
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(back, "a:\r\n    one\r\n    -two\r\nb:\r\n");
}

#[test]
fn inline_refuses_a_load_it_did_not_write() {
    let with_args = "a:\n    sh ./x.sh --flag\n";
    let loads = JUST.loads(with_args).unwrap_or_else(|e| panic!("{e}"));
    let load = loads.first().unwrap_or_else(|| panic!("one load"));
    assert!(JUST.inline(with_args, load, "ls").is_err());

    let infallible = "a:\n    -sh ./x.sh\n";
    let loads = JUST.loads(infallible).unwrap_or_else(|e| panic!("{e}"));
    let load = loads.first().unwrap_or_else(|| panic!("one load"));
    assert!(JUST.inline(infallible, load, "ls").is_err());

    let mut stranger = load.clone();
    stranger.path = PathBuf::from("./other.sh");
    assert!(JUST.inline(infallible, &stranger, "ls").is_err());
}
