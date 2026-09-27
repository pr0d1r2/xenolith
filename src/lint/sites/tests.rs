//! Site linting: the mirror of `src/lint/sites.rs` (`src:C139`).
//!
//! Pinned (`src/lint:V93`, `src/lint` §I sites): the body a site's
//! checks read -- holes as markers, unescaped, the guest's prelude on
//! top -- and every position in it mapped back to the host file. The
//! fakes run in every feature subset (`src:V30`); the task's fixture, a
//! shellcheck finding in a nix `script`, runs with the real languages.

use std::path::{Path, PathBuf};

use xenolith_lang_api::{
    Delim, DelimKind, Error, FileArg, Format, Guest, GuestEnv, Host, Invoke, LangId, LintCmd,
    LoadRef, Prelude, Result, Shebang, Site, Span,
};

use super::{align, guest_text, materialise, position};
use crate::check::Langs;
use crate::config::{self, Config};
use crate::discover::{Sandbox, write};
use crate::lint::run::Tools;
use crate::lint::tests::stub;
use crate::lint::{Kind, LintReport, Options, Status, lint_with};

// ---------------------------------------------------------------------
// positions
// ---------------------------------------------------------------------

#[test]
fn position_is_one_based_and_counts_characters() {
    let src = "ab\ncé$x\n";
    assert_eq!(position(src, 0), (1, 1));
    assert_eq!(position(src, 3), (2, 1));
    // `$` follows the two-byte `é`: byte 6, character column 3.
    assert_eq!(position(src, 6), (2, 3));
}

// ---------------------------------------------------------------------
// align: unescaped characters back to raw bytes
// ---------------------------------------------------------------------

/// The raw byte each character of `unescaped` came from, as text.
fn origin(raw: &str, unescaped: &str, needle: char) -> usize {
    let at = unescaped
        .chars()
        .position(|c| c == needle)
        .unwrap_or_else(|| panic!("no {needle:?} in {unescaped:?}"));
    align(raw, unescaped)
        .get(at)
        .copied()
        .unwrap_or_else(|| panic!("no offset for char {at}"))
}

#[test]
fn text_unescape_left_alone_maps_onto_itself() {
    let text = "echo $a\nls\n";
    let offsets = align(text, text);
    assert_eq!(offsets.len(), text.chars().count() + 1);
    assert_eq!(offsets, (0..=text.len()).collect::<Vec<_>>());
}

#[test]
fn a_stripped_indent_and_a_dropped_first_line_are_stepped_over() {
    // A nix `''` string: the first line is empty and dropped, and the
    // common indent of four is stripped.
    let raw = "\n    if x; then\n      echo $a\n    fi\n  ";
    let unescaped = "if x; then\n  echo $a\nfi\n";
    let dollar = raw.find('$').unwrap_or_default();
    assert_eq!(origin(raw, unescaped, '$'), dollar);
    // The closing `fi`, back under its own indent (ASCII: bytes are
    // characters).
    let fi = unescaped.find("fi").unwrap_or_default();
    let offsets = align(raw, unescaped);
    assert_eq!(offsets.get(fi), raw.find("fi").as_ref());
}

#[test]
fn an_escape_maps_to_the_character_it_became() {
    // nix `''$` reads as `$`.
    let raw = "echo ''${a}";
    assert_eq!(origin(raw, "echo ${a}", '$'), 7);
}

#[test]
fn a_character_unmatched_on_its_line_stays_put() {
    let raw = "ab\ncd";
    let offsets = align(raw, "aZb\ncd");
    // `Z` is nowhere on line 1: it keeps the place after `a`, and `b`
    // is still found.
    assert_eq!(offsets.get(1), Some(&1));
    assert_eq!(offsets.get(2), Some(&1));
    assert_eq!(offsets.get(4), Some(&3));
}

// ---------------------------------------------------------------------
// the body a site's checks read
// ---------------------------------------------------------------------

/// `src` with one site whose body is the text between the first `[`
/// and the following `]`, holes at each `{…}` inside it.
fn site_in(src: &str) -> Site {
    let open = src.find('[').unwrap_or_default();
    let close = src[open..].find(']').map_or(src.len(), |i| open + i);
    let mut holes = Vec::new();
    let mut from = open;
    while let Some(i) = src[from..close].find('{') {
        let start = from + i;
        let end = src[start..].find('}').map_or(close, |j| start + j + 1);
        holes.push(Span::new(start, end));
        from = end;
    }
    Site {
        sink: "run".to_owned(),
        guest: LangId::Shell,
        env: GuestEnv::default(),
        delim: Delim {
            kind: DelimKind::ArgvString,
            open: Span::new(open, open + 1),
            body: Span::new(open + 1, close),
            close: Span::new(close, close + 1),
        },
        holes,
    }
}

#[test]
fn holes_become_markers_and_map_back_to_their_start() {
    let src = "x = [echo {a.b} {c}]\n";
    let site = site_in(src);
    let (text, pieces) = guest_text(src, &site);
    assert_eq!(text, "echo XNL_HOLE_0_ XNL_HOLE_1_");
    let hole = src.find("{c}").unwrap_or_default();
    let at = text.find("XNL_HOLE_1_").unwrap_or_default();
    assert_eq!(pieces.host(at + 3), hole);
    let echo = src.find("echo").unwrap_or_default();
    assert_eq!(pieces.host(1), echo + 1);
}

#[test]
fn materialise_wraps_the_unescaped_body_in_the_prelude_and_maps_back() {
    let src = "a\nx = [\n  echo $b\n  ls\n]\n";
    let site = site_in(src);
    let body = materialise(&UpperHost, &FakeGuest, src, &site)
        .unwrap_or_else(|e| panic!("materialise: {e}"));
    assert_eq!(
        body.text,
        "#!/usr/bin/env fake\nSET -E\n\n  ECHO $B\n  LS\n"
    );
    // Line 4 of the file is `  ECHO $B`: `$` at column 8 is host 3:8.
    assert_eq!(body.map.locate(src, 4, 8), (3, 8));
    // A finding on the prelude is placed at the body's start.
    let start = position(src, site.delim.body.start);
    assert_eq!(body.map.locate(src, 1, 1), start);
    assert_eq!(body.map.locate(src, 0, 0), start);
    // Past the end of a line or of the body stays inside the site.
    assert_eq!(body.map.locate(src, 5, 99).0, 4);
    assert!(body.map.locate(src, 99, 1).0 <= 5);
}

#[test]
fn a_host_that_cannot_unescape_is_an_error() {
    let src = "x = [echo]\n";
    let got = materialise(&BadHost, &FakeGuest, src, &site_in(src));
    assert!(got.is_err());
}

// ---------------------------------------------------------------------
// fakes
// ---------------------------------------------------------------------

fn cmd(argv: &[&str], format: Format) -> LintCmd {
    LintCmd {
        argv: argv.iter().map(|a| (*a).to_owned()).collect(),
        file_arg: FileArg::Append,
        format,
    }
}

/// A shell-like guest with a two-line prelude, one JSON check and one
/// raw check, and a fixer that must never run on a site.
struct FakeGuest;

impl Guest for FakeGuest {
    fn id(&self) -> LangId {
        LangId::Shell
    }

    fn extension(&self, _env: &GuestEnv) -> &'static str {
        "fk"
    }

    fn invoke(&self, path: &Path) -> Invoke {
        Invoke {
            argv: vec![path.display().to_string()],
        }
    }

    fn trivial(&self, _body: &str) -> Result<bool> {
        Ok(false)
    }

    fn prelude(&self, _env: &GuestEnv) -> Prelude {
        Prelude {
            shebang: Some(Shebang::env("fake")),
            strict: Some("SET -E".to_owned()),
        }
    }

    fn executable(&self) -> bool {
        true
    }

    fn checks(&self, _env: &GuestEnv) -> Vec<LintCmd> {
        vec![
            cmd(&["sc"], Format::Json("shellcheck")),
            cmd(&["fmtck"], Format::Raw),
        ]
    }

    fn fixers(&self, _env: &GuestEnv) -> Vec<LintCmd> {
        vec![cmd(&["fixit"], Format::Raw)]
    }
}

/// A host claiming `*.hx`: a site is `[…]`, unescape upper-cases.
struct UpperHost;

impl Host for UpperHost {
    fn id(&self) -> LangId {
        LangId::Nix
    }

    fn claims(&self, path: &Path, _head: &str) -> bool {
        path.extension().is_some_and(|e| e == "hx")
    }

    fn sites(&self, src: &str) -> Result<Vec<Site>> {
        if src.contains('[') {
            Ok(vec![site_in(src)])
        } else {
            Err(Error::parse(LangId::Nix, "no site"))
        }
    }

    fn loads(&self, _src: &str) -> Result<Vec<LoadRef>> {
        Err(Error::unsupported(LangId::Nix, "loads"))
    }

    fn rewrite(&self, _: &str, _: &Site, _: &Invoke, _: &Path) -> Result<String> {
        Err(Error::unsupported(LangId::Nix, "rewrite"))
    }

    fn inline(&self, _: &str, _: &LoadRef, _: &str) -> Result<String> {
        Err(Error::unsupported(LangId::Nix, "inline"))
    }

    fn unescape(&self, _: &Delim, raw: &str) -> Result<String> {
        Ok(raw.to_uppercase())
    }

    fn checks(&self) -> Vec<LintCmd> {
        Vec::new()
    }

    fn fixers(&self) -> Vec<LintCmd> {
        Vec::new()
    }
}

/// [`UpperHost`] whose unescape always refuses.
struct BadHost;

impl Host for BadHost {
    fn id(&self) -> LangId {
        LangId::Nix
    }

    fn claims(&self, path: &Path, _head: &str) -> bool {
        path.extension().is_some_and(|e| e == "bx")
    }

    fn sites(&self, src: &str) -> Result<Vec<Site>> {
        Ok(vec![site_in(src)])
    }

    fn loads(&self, _src: &str) -> Result<Vec<LoadRef>> {
        Err(Error::unsupported(LangId::Nix, "loads"))
    }

    fn rewrite(&self, _: &str, _: &Site, _: &Invoke, _: &Path) -> Result<String> {
        Err(Error::unsupported(LangId::Nix, "rewrite"))
    }

    fn inline(&self, _: &str, _: &LoadRef, _: &str) -> Result<String> {
        Err(Error::unsupported(LangId::Nix, "inline"))
    }

    fn unescape(&self, _: &Delim, _: &str) -> Result<String> {
        Err(Error::parse(LangId::Nix, "bad escape"))
    }

    fn checks(&self) -> Vec<LintCmd> {
        Vec::new()
    }

    fn fixers(&self) -> Vec<LintCmd> {
        Vec::new()
    }
}

// ---------------------------------------------------------------------
// the engine: `--sites`
// ---------------------------------------------------------------------

struct Fixture {
    sandbox: Sandbox,
    root: PathBuf,
    bin: PathBuf,
}

impl Fixture {
    fn new() -> Fixture {
        let sandbox = Sandbox::new();
        let root = sandbox.plain("tree");
        let bin = sandbox.plain("bin");
        for tool in ["sc", "fmtck"] {
            stub(&bin, tool, "exit 0");
        }
        // A fixer that ran would leave this behind.
        stub(&bin, "fixit", "echo > fixit-ran");
        Fixture { sandbox, root, bin }
    }

    fn lint(&self, langs: &Langs<'_>, file: &str, options: &Options) -> LintReport {
        let options = Options {
            paths: vec![PathBuf::from(file)],
            ..options.clone()
        };
        lint_with(
            &self.root,
            &Config::default(),
            &options,
            langs,
            &|| self.sandbox.git(),
            &Tools::on_path(&self.bin),
        )
        .unwrap_or_else(|e| panic!("lint refused: {e}"))
    }
}

fn sites() -> Options {
    Options {
        sites: true,
        ..Options::default()
    }
}

const FAKES: Langs<'static> = Langs {
    hosts: &[&UpperHost, &BadHost],
    guests: &[&FakeGuest],
};

#[test]
fn without_sites_a_host_file_has_no_site_results() {
    let fx = Fixture::new();
    write(&fx.root, "a.hx", "x = [echo $a]\n");
    let report = fx.lint(&FAKES, "a.hx", &Options::default());
    assert!(report.outcomes().is_empty(), "{:?}", report.outcomes());
}

#[test]
fn each_check_runs_on_the_site_and_is_reported_as_a_site_of_the_host() {
    let fx = Fixture::new();
    // The JSON check reports line 3 (the body's first line under the
    // two-line prelude), column 6: the `$` of `ECHO $A`.
    stub(
        &fx.bin,
        "sc",
        "/bin/cat \"$1\" > seen\necho \"$1\" > seen-path\n\
         echo '[{\"line\":3,\"column\":6,\"level\":\"info\",\"code\":2086,\
         \"message\":\"quote\"}]'\nexit 1",
    );
    stub(&fx.bin, "fmtck", "echo \"bad $1\"\nexit 1");
    write(&fx.root, "a.hx", "top\nx = [echo $a]\n");
    let report = fx.lint(&FAKES, "a.hx", &sites());
    let names: Vec<&str> = report.outcomes().iter().map(|o| o.check.as_str()).collect();
    assert_eq!(names, ["sc", "fmtck"]);
    let virtual_name = "a.hx:2:6";
    for outcome in report.outcomes() {
        assert_eq!(outcome.kind, Kind::Site);
        assert_eq!(outcome.file, PathBuf::from("a.hx"));
        assert_eq!(outcome.guest, Some(LangId::Shell));
        assert_eq!(outcome.status, Status::Fail);
        assert_eq!(outcome.argv.last().map(String::as_str), Some(virtual_name));
    }
    let seen = std::fs::read_to_string(fx.root.join("seen")).unwrap_or_default();
    assert_eq!(seen, "#!/usr/bin/env fake\nSET -E\nECHO $A");
    let sc = report.outcomes().first();
    let found: Vec<(usize, usize, &str)> = sc
        .map(|o| {
            o.findings
                .iter()
                .map(|f| (f.line, f.col, f.code.as_str()))
                .collect()
        })
        .unwrap_or_default();
    // `$a` is at column 11 of host line 2.
    assert_eq!(found, [(2, 11, "SC2086")]);
    // The temp path never reaches the report, and the file is gone.
    let fmtck = report.outcomes().get(1);
    let tail = fmtck.and_then(|o| o.raw_tail.clone()).unwrap_or_default();
    assert_eq!(tail, format!("bad {virtual_name}"));
    let json = report.to_json();
    assert!(!json.contains("xnl-lint-"), "{json}");
    let temp = std::fs::read_to_string(fx.root.join("seen-path")).unwrap_or_default();
    assert!(!Path::new(temp.trim()).exists(), "{temp} left behind");
    assert!(json.contains("\"kind\": \"site\""), "{json}");
}

#[test]
fn fix_never_runs_a_fixer_on_a_site() {
    let fx = Fixture::new();
    write(&fx.root, "a.hx", "x = [echo]\n");
    let options = Options {
        fix: true,
        ..sites()
    };
    let report = fx.lint(&FAKES, "a.hx", &options);
    assert!(report.outcomes().iter().all(|o| !o.fixer));
    assert!(!fx.root.join("fixit-ran").exists());
}

#[test]
fn a_site_that_cannot_be_read_is_a_warning_not_a_result() {
    let fx = Fixture::new();
    write(&fx.root, "a.bx", "x = [echo]\n");
    write(&fx.root, "b.hx", "no site here\n");
    for file in ["a.bx", "b.hx"] {
        let report = fx.lint(&FAKES, file, &sites());
        assert!(report.outcomes().is_empty(), "{:?}", report.outcomes());
        let codes: Vec<&str> = report.warnings().iter().map(|w| w.code.as_str()).collect();
        assert_eq!(codes, ["site-unlinted"], "{file}");
        assert_eq!(report.exit_code(), 0);
    }
}

#[test]
fn a_site_whose_guest_this_build_lacks_is_left_to_xnl_check() {
    let fx = Fixture::new();
    write(&fx.root, "a.hx", "x = [echo]\n");
    let langs = Langs {
        hosts: &[&UpperHost],
        guests: &[],
    };
    let report = fx.lint(&langs, "a.hx", &sites());
    assert!(report.outcomes().is_empty(), "{:?}", report.outcomes());
    assert!(report.warnings().is_empty(), "{:?}", report.warnings());
}

#[test]
fn untrusted_config_on_a_site_is_skipped_under_the_virtual_name() {
    let fx = Fixture::new();
    write(&fx.root, "a.hx", "x = [echo]\n");
    let config = config::parse("version = 1\n[lint.shell]\nchecks = [\"own\"]\n")
        .unwrap_or_else(|e| panic!("config: {e}"));
    let options = Options {
        paths: vec![PathBuf::from("a.hx")],
        ..sites()
    };
    let report = lint_with(
        &fx.root,
        &config,
        &options,
        &FAKES,
        &|| fx.sandbox.git(),
        &Tools::on_path(&fx.bin),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let own = report
        .outcomes()
        .iter()
        .find(|o| o.check == "own")
        .unwrap_or_else(|| panic!("{:?}", report.outcomes()));
    assert_eq!(own.status, Status::Skipped);
    assert_eq!(own.kind, Kind::Site);
    assert_eq!(own.file, PathBuf::from("a.hx"));
    assert_eq!(own.argv, ["own", "a.hx:1:6"]);
}

// ---------------------------------------------------------------------
// T94's fixture, with the real languages
// ---------------------------------------------------------------------

#[cfg(all(feature = "lang-nix", feature = "lang-shell"))]
mod nix_shell {
    use std::path::PathBuf;

    use super::{Fixture, sites};
    use crate::check::Langs;
    use crate::lint::run::Tools;
    use crate::lint::tests::stub;
    use crate::lint::{Kind, Options, lint_with};
    use crate::registry;

    const SERVICE: &str =
        "{\n  systemd.services.a.script = ''\n    foo=$1\n    echo $foo\n  '';\n}\n";

    #[test]
    fn a_shellcheck_finding_in_a_nix_script_is_reported_at_the_nix_line() {
        // `src/lint:T94`: the body under bash's two-line prelude is
        // `foo=$1` / `echo $foo`; shellcheck's SC2086 on `$foo`, line 4
        // column 6 of the file it read, is the nix file's line 4, column
        // 10.
        let fx = Fixture::new();
        stub(
            &fx.bin,
            "shellcheck",
            "echo '[{\"line\":4,\"column\":6,\"level\":\"info\",\"code\":2086,\
             \"message\":\"Double quote to prevent globbing.\"}]'\nexit 1",
        );
        stub(&fx.bin, "shfmt", "exit 0");
        crate::discover::write(&fx.root, "service.nix", SERVICE);
        let config = crate::config::parse("version = 1\n[lint]\nhosts = false\n")
            .unwrap_or_else(|e| panic!("config: {e}"));
        let langs = Langs {
            hosts: registry::hosts(),
            guests: registry::guests(),
        };
        let options = Options {
            paths: vec![PathBuf::from("service.nix")],
            ..sites()
        };
        let report = lint_with(
            &fx.root,
            &config,
            &options,
            &langs,
            &|| fx.sandbox.git(),
            &Tools::on_path(&fx.bin),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let shellcheck = report
            .outcomes()
            .iter()
            .find(|o| o.check == "shellcheck")
            .unwrap_or_else(|| panic!("{:?}", report.outcomes()));
        assert_eq!(shellcheck.kind, Kind::Site);
        assert_eq!(shellcheck.file, PathBuf::from("service.nix"));
        let found: Vec<(usize, usize, &str)> = shellcheck
            .findings
            .iter()
            .map(|f| (f.line, f.col, f.code.as_str()))
            .collect();
        assert_eq!(found, [(4, 10, "SC2086")]);
    }
}
