//! XML as a host: launchd jobs holding shell (`languages/data/xml:T192`).
//!
//! A text property list whose top dict runs `sh -c <script>` through
//! `ProgramArguments` holds a shell program in a `<string>` element
//! (`languages/data/xml:V189`). The same argv anywhere else -- under a
//! `Program` key, below the top dict, outside a `<plist>` -- is inert
//! data (`languages:V2`), and each has a negative fixture here
//! (`tests:V15`).
//!
//! Fixtures live in this crate (`tests:V14`), one directory per case,
//! holding `input.plist` or `input.xml`.

use std::path::{Path, PathBuf};

use xenolith_lang_api::{DelimKind, GuestEnv, Host, LangId, Site};
use xenolith_lang_xml::{XmlHost, unescape};

fn fixture_path(case: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(case);
    ["input.plist", "input.xml"]
        .iter()
        .map(|name| dir.join(name))
        .find(|path| path.is_file())
        .unwrap_or_else(|| panic!("{}: no input.plist or input.xml", dir.display()))
}

fn fixture(case: &str) -> String {
    let path = fixture_path(case);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The first line, as the engine reads it for `claims`: at most 1 KiB,
/// lossy, so a binary file has a head too (`src/check:V13`).
fn head(path: &Path) -> String {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let first = bytes
        .iter()
        .take(1024)
        .take_while(|&&byte| byte != b'\n')
        .copied()
        .collect::<Vec<u8>>();
    String::from_utf8_lossy(&first).into_owned()
}

fn claimed(case: &str) -> bool {
    let path = fixture_path(case);
    XmlHost.claims(&path, &head(&path))
}

fn sites(src: &str) -> Vec<Site> {
    XmlHost
        .sites(src)
        .unwrap_or_else(|e| panic!("sites failed: {e}"))
}

fn one(case: &str) -> (String, Site) {
    let src = fixture(case);
    match sites(&src).as_slice() {
        [site] => {
            let site = site.clone();
            (src, site)
        }
        other => panic!("{case}: expected one site, found {other:?}"),
    }
}

fn body(src: &str, site: &Site) -> String {
    let raw = site
        .delim
        .body
        .of(src)
        .unwrap_or_else(|| panic!("body span {:?} does not fit", site.delim.body));
    unescape(&site.delim, raw).unwrap_or_else(|e| panic!("unescape failed: {e}"))
}

fn dialect(name: &str) -> GuestEnv {
    GuestEnv {
        dialect: Some(name.to_owned()),
        options: Vec::new(),
    }
}

#[test]
fn every_text_fixture_is_claimed() {
    for case in [
        "launchd-sh-c",
        "launchd-bash-single",
        "launchd-program-key",
        "launchd-entities",
        "no-sink",
    ] {
        assert!(claimed(case), "{case}");
    }
}

#[test]
fn a_binary_plist_is_not_claimed() {
    // Not claimed, so never parsed (`languages/data/xml:V188`,
    // `src/check:V13`): the file is not text at all. It is launchd-sh-c
    // converted by `plutil -convert binary1`, its label padded until the
    // trailer's last byte is 0x0a, so the byte-level gates pass it as is.
    let path = fixture_path("binary-plist");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert!(
        bytes.starts_with(b"bplist00"),
        "the fixture is a binary plist"
    );
    assert!(String::from_utf8(bytes).is_err(), "the fixture is not text");
    assert!(!claimed("binary-plist"));
}

#[test]
fn a_launchd_sh_dash_c_script_is_a_site() {
    let (src, site) = one("launchd-sh-c");
    assert_eq!(site.sink, "ProgramArguments");
    assert_eq!(site.guest, LangId::Shell);
    assert_eq!(site.env, dialect("sh"));
    assert_eq!(site.delim.kind, DelimKind::ArgvString);
    assert_eq!(
        body(&src, &site),
        "cd \"$HOME/data\" || exit 1\nfor f in *.db; do\n  cp \"$f\" \"$HOME/backup/$f\"\ndone"
    );
}

#[test]
fn a_bash_dash_c_single_command_is_a_site_the_guest_calls_trivial() {
    // The host reports it; the shell guest's verdict keeps it inline
    // (`languages/shells/shell:V3`), which the root crate's registry test
    // proves with both crates compiled in.
    let (src, site) = one("launchd-bash-single");
    assert_eq!(site.env, dialect("bash"));
    assert_eq!(body(&src, &site), "exec /usr/local/bin/sync-tool --quiet");
}

#[test]
fn a_program_key_leaves_no_site() {
    assert!(sites(&fixture("launchd-program-key")).is_empty());
}

#[test]
fn an_entity_escaped_body_is_decoded() {
    let (src, site) = one("launchd-entities");
    assert_eq!(site.env, dialect("zsh"));
    assert_eq!(
        body(&src, &site),
        "test -d /tmp/r && ls /tmp/r | wc -l > /tmp/r.count\n\
         [ \"$(cat /tmp/r.count)\" -lt 3 ] && echo low && echo \"<done>\""
    );
}

#[test]
fn an_xml_file_without_a_sink_is_clean() {
    assert!(sites(&fixture("no-sink")).is_empty());
}

#[test]
fn every_fixture_answers_the_same_under_crlf() {
    for case in [
        "launchd-sh-c",
        "launchd-bash-single",
        "launchd-program-key",
        "launchd-entities",
        "no-sink",
    ] {
        let lf = fixture(case);
        let crlf = lf.replace('\n', "\r\n");
        let bodies =
            |src: &str| -> Vec<String> { sites(src).iter().map(|site| body(src, site)).collect() };
        assert_eq!(bodies(&crlf), bodies(&lf), "{case}");
    }
}
