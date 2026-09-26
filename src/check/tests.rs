//! The check engine: the mirror of `src/check.rs` (`src:C139`).
//!
//! Two kinds of case. Most run the pipeline through [`check_with`] with
//! a FAKE host and fake guests, so they hold in every feature subset
//! `cargo hack --each-feature` builds (`src:V30`) and pin the engine's
//! own rules -- stage order, threshold, allow, staleness, policies --
//! rather than any one grammar's. The `src:T153` fixtures that need real
//! languages (nix and pkl finding shell) are gated on the features they
//! need and go through [`check`] and the registry, as `xnl` does.
//!
//! Every tree lives in a [`Sandbox`], and every git these tests run is
//! the sandbox's (`tests:V150`, `tests:B1`).

use std::path::{Path, PathBuf};

use xenolith_lang_api::{
    Delim, DelimKind, Error, Guest, GuestEnv, Host, Invoke, LangId, LintCmd, LoadRef, Prelude,
    Result, Site, Span,
};

use super::{
    CheckError, HOLE, Langs, Options, body_hash, check_with, guest_text, head, position, repo_name,
    under_root,
};
use crate::config::{self, Config};
use crate::discover::{Sandbox, write};
use crate::model::{Fix, Report, Rule, Violation};

// ---------------------------------------------------------------------
// fakes
// ---------------------------------------------------------------------

/// A host that claims `*.fake` and reads one site per line,
/// `<sink>=<guest>: <body>`. `{{…}}` in a body is a hole; a line `!` is
/// a parse error.
struct FakeHost;

impl Host for FakeHost {
    fn id(&self) -> LangId {
        LangId::Just
    }

    fn claims(&self, path: &Path, _head: &str) -> bool {
        path.extension().is_some_and(|ext| ext == "fake")
    }

    fn sites(&self, src: &str) -> Result<Vec<Site>> {
        let mut sites = Vec::new();
        let mut offset = 0;
        for line in src.split_inclusive('\n') {
            let start = offset;
            offset += line.len();
            let line = line.trim_end_matches('\n');
            if line == "!" {
                return Err(Error::parse(LangId::Just, "a fake syntax error"));
            }
            let Some((head, body)) = line.split_once(": ") else {
                continue;
            };
            let Some((sink, guest)) = head.split_once('=') else {
                continue;
            };
            let Some(guest) = LangId::from_name(guest) else {
                continue;
            };
            let open = start + head.len();
            let body_start = open + 2;
            let body_end = start + line.len();
            let holes = body
                .match_indices("{{")
                .filter_map(|(at, _)| {
                    let close = body.get(at..)?.find("}}")?;
                    Some(Span::new(body_start + at, body_start + at + close + 2))
                })
                .collect();
            sites.push(Site {
                sink: sink.to_owned(),
                guest,
                env: GuestEnv::default(),
                delim: Delim {
                    kind: DelimKind::JustRecipe,
                    open: Span::new(open, body_start),
                    body: Span::new(body_start, body_end),
                    close: Span::new(body_end, body_end),
                },
                holes,
            });
        }
        Ok(sites)
    }

    fn loads(&self, _src: &str) -> Result<Vec<LoadRef>> {
        Err(Error::unsupported(LangId::Just, "loads"))
    }

    fn rewrite(&self, _: &str, _: &Site, _: &Invoke, _: &Path) -> Result<String> {
        Err(Error::unsupported(LangId::Just, "rewrite"))
    }

    fn inline(&self, _: &str, _: &LoadRef, _: &str) -> Result<String> {
        Err(Error::unsupported(LangId::Just, "inline"))
    }

    /// `\&` is an escaped `&`; `\!` is an escape nothing decodes.
    fn unescape(&self, _: &Delim, raw: &str) -> Result<String> {
        if raw.contains("\\!") {
            return Err(Error::parse(LangId::Just, "a fake bad escape"));
        }
        Ok(raw.replace("\\&", "&"))
    }

    fn checks(&self) -> Vec<LintCmd> {
        Vec::new()
    }

    fn fixers(&self) -> Vec<LintCmd> {
        Vec::new()
    }
}

/// A guest with a construct vocabulary, like shell: `&&` is `and-or`,
/// `|` is `pipeline`, `BAD` does not parse, and a body holding
/// [`HOLE`]'s text records that the engine replaced a hole.
struct FakeShell;

/// A guest without one, like python so far: never trivial unless
/// `pass`, judged by `max_lines` / `max_bytes`.
struct FakePython;

fn fake_constructs(body: &str) -> Result<Vec<&'static str>> {
    if body.contains("BAD") || body.contains("{{") {
        return Err(Error::parse(
            LangId::Shell,
            "fake shell does not parse this",
        ));
    }
    let mut names = Vec::new();
    if body.contains("&&") {
        names.push("and-or");
    }
    if body.contains('|') {
        names.push("pipeline");
    }
    Ok(names)
}

macro_rules! fake_guest_parts {
    ($id:expr) => {
        fn id(&self) -> LangId {
            $id
        }
        fn extension(&self, _: &GuestEnv) -> &'static str {
            "x"
        }
        fn invoke(&self, path: &Path) -> Invoke {
            Invoke {
                argv: vec![path.display().to_string()],
            }
        }
        fn prelude(&self, _: &GuestEnv) -> Prelude {
            Prelude {
                shebang: None,
                strict: None,
            }
        }
        fn executable(&self) -> bool {
            false
        }
        fn checks(&self, _: &GuestEnv) -> Vec<LintCmd> {
            Vec::new()
        }
        fn fixers(&self, _: &GuestEnv) -> Vec<LintCmd> {
            Vec::new()
        }
    };
}

impl Guest for FakeShell {
    fake_guest_parts!(LangId::Shell);

    fn trivial(&self, body: &str) -> Result<bool> {
        fake_constructs(body).map(|names| names.is_empty())
    }

    fn constructs(&self, body: &str) -> Result<Vec<&'static str>> {
        fake_constructs(body)
    }
}

impl Guest for FakePython {
    fake_guest_parts!(LangId::Python);

    fn trivial(&self, body: &str) -> Result<bool> {
        Ok(body.trim() == "pass")
    }
}

const HOSTS: &[&dyn Host] = &[&FakeHost];
const GUESTS: &[&dyn Guest] = &[&FakePython, &FakeShell];

fn fakes() -> Langs<'static> {
    Langs {
        hosts: HOSTS,
        guests: GUESTS,
    }
}

// ---------------------------------------------------------------------
// driving it
// ---------------------------------------------------------------------

fn config(toml: &str) -> Config {
    config::parse(toml).unwrap_or_else(|e| panic!("fixture config parses: {e}"))
}

/// A plain directory holding `files`, and `xenolith.toml` when given.
fn tree(sandbox: &Sandbox, files: &[(&str, &str)]) -> PathBuf {
    let root = sandbox.plain("t");
    for (rel, text) in files {
        write(&root, rel, text);
    }
    root
}

fn run_with(root: &Path, config: &Config, paths: &[&str], langs: &Langs<'_>) -> Report {
    let options = Options {
        paths: paths.iter().map(PathBuf::from).collect(),
        ..Options::default()
    };
    let sandbox_git = || {
        // Paths are named, so discovery only runs git for a directory;
        // none of these name one. A git that fails loudly proves it.
        std::process::Command::new("false")
    };
    check_with(root, config, &options, langs, &sandbox_git)
        .unwrap_or_else(|e| panic!("expected a report, got: {e}"))
}

fn run(root: &Path, config: &Config, paths: &[&str]) -> Report {
    run_with(root, config, paths, &fakes())
}

fn refused(root: &Path, config: &Config, paths: &[&str], langs: &Langs<'_>) -> CheckError {
    let options = Options {
        paths: paths.iter().map(PathBuf::from).collect(),
        ..Options::default()
    };
    match check_with(root, config, &options, langs, &|| {
        std::process::Command::new("false")
    }) {
        Ok(report) => panic!("expected a refusal, got {report:?}"),
        Err(e) => e,
    }
}

fn rules(report: &Report) -> Vec<(String, usize, Rule)> {
    report
        .violations()
        .iter()
        .map(|v| (v.file.display().to_string(), v.line, v.rule))
        .collect()
}

fn only(report: &Report) -> &Violation {
    match report.violations() {
        [one] => one,
        other => panic!("expected one violation, got {other:#?}"),
    }
}

// ---------------------------------------------------------------------
// the verdict (`src:V152` stages 4-5, `src/config:V55`)
// ---------------------------------------------------------------------

#[test]
fn a_script_is_a_violation_carrying_every_field() {
    // `src:V1`: rule, position, both languages, sink, delimiter kind, a
    // reason naming what makes it a script, and directions.
    let sandbox = Sandbox::new();
    let root = tree(
        &sandbox,
        &[("a.fake", "# top\nbuild=shell: make && make install\n")],
    );
    let report = run(&root, &Config::default(), &["a.fake"]);
    let v = only(&report);
    assert_eq!(v.rule, Rule::Xenolith);
    assert_eq!(v.file, PathBuf::from("a.fake"));
    assert_eq!((v.line, v.col), (2, 12));
    assert_eq!((v.host, v.guest), (LangId::Just, LangId::Shell));
    assert_eq!(v.sink, "build");
    assert_eq!(v.site, DelimKind::JustRecipe);
    assert!(v.why.contains("and-or"), "{}", v.why);
    let kinds: Vec<Fix> = v.directions.iter().map(|d| d.kind).collect();
    assert_eq!(kinds, vec![Fix::Judgment, Fix::Judgment]);
    let allow = v
        .directions
        .last()
        .map(|d| d.action.clone())
        .unwrap_or_default();
    assert!(
        allow.contains(&body_hash("make && make install")),
        "the allow direction carries the key to paste: {allow}"
    );
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn the_extract_direction_never_claims_to_run_before_extract_can() {
    // `xnl extract` refuses every host until `src/extract:T22` lands, so
    // no direction may promise a mechanical fix -- least of all for a
    // body the guest or the host cannot even read.
    let sandbox = Sandbox::new();
    let root = tree(
        &sandbox,
        &[(
            "a.fake",
            "one=shell: a && b\ntwo=shell: BAD\nthree=shell: x \\!\n",
        )],
    );
    let report = run(&root, &Config::default(), &["a.fake"]);
    assert_eq!(report.violations().len(), 3, "{report:?}");
    for v in report.violations() {
        let first = v
            .directions
            .first()
            .unwrap_or_else(|| panic!("{v:?} has a direction"));
        assert_eq!(first.kind, Fix::Judgment, "{v:?}");
        assert!(first.action.contains("src/extract:T22"), "{}", first.action);
        assert!(
            v.directions.iter().all(|d| d.kind != Fix::Mechanical),
            "{v:?}"
        );
    }
}

#[test]
fn a_single_command_is_clean() {
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &[("a.fake", "build=shell: make install\n")]);
    let report = run(&root, &Config::default(), &["a.fake"]);
    assert!(report.violations().is_empty(), "{report:?}");
    assert_eq!(report.exit_code(), 0);
}

#[test]
fn an_unparseable_body_is_flagged_not_passed() {
    // `languages:V77`: calling it trivial would leave broken code inline.
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &[("a.fake", "build=shell: BAD\n")]);
    let v = run(&root, &Config::default(), &["a.fake"]);
    assert!(only(&v).why.starts_with("unparseable shell"), "{v:?}");
}

#[test]
fn a_threshold_relaxes_exactly_the_constructs_it_lists() {
    let sandbox = Sandbox::new();
    let root = tree(
        &sandbox,
        &[("a.fake", "one=shell: a && b\ntwo=shell: a && b | c\n")],
    );
    let relaxed = config("version = 1\n[threshold.shell]\nallow = [\"and-or\"]\n");
    let report = run(&root, &relaxed, &["a.fake"]);
    let v = only(&report);
    assert_eq!(v.sink, "two");
    assert!(v.why.contains("pipeline"), "{}", v.why);
    assert!(
        !v.why.contains("and-or"),
        "a relaxed construct is not the reason: {}",
        v.why
    );
}

#[test]
fn a_threshold_never_flags_a_trivial_body() {
    // `src/config:V55`: a threshold only relaxes. The strictest ceiling
    // there is still leaves a trivial body alone.
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &[("a.fake", "a=shell: ls\nb=python: pass\n")]);
    let strict = config("version = 1\n[threshold.python]\nmax_lines = 0\nmax_bytes = 0\n");
    assert!(run(&root, &strict, &["a.fake"]).violations().is_empty());
}

#[test]
fn a_guest_without_constructs_is_judged_by_its_size_ceiling() {
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &[("a.fake", "p=python: print(1)\n")]);
    // Default ceiling: one line, 80 bytes. `print(1)` fits.
    assert!(
        run(&root, &Config::default(), &["a.fake"])
            .violations()
            .is_empty()
    );
    let tight = config("version = 1\n[threshold.python]\nmax_bytes = 4\n");
    let report = run(&root, &tight, &["a.fake"]);
    let why = &only(&report).why;
    assert!(why.contains("max_bytes = 4"), "{why}");
}

#[test]
fn a_hole_reaches_the_guest_as_a_plain_word_and_makes_extraction_a_judgement() {
    // The fake shell refuses `{{`, so a clean verdict proves the hole was
    // replaced; the `&&` then makes it a script with a hole in it.
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &[("a.fake", "a=shell: {{x}} && y\n")]);
    let report = run(&root, &Config::default(), &["a.fake"]);
    let v = only(&report);
    assert!(v.why.contains("and-or"), "{}", v.why);
    assert_eq!(v.directions.first().map(|d| d.kind), Some(Fix::Judgment));
}

#[test]
fn the_guest_reads_the_body_the_host_unescaped() {
    // `languages/api/src/lens:V39`: raw, the fake shell sees `\&\&` and
    // no `&&`; through the host's `unescape` it sees the and-or.
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &[("a.fake", "a=shell: x \\&\\& y\n")]);
    let report = run(&root, &Config::default(), &["a.fake"]);
    let why = &only(&report).why;
    assert!(why.contains("and-or"), "{why}");
}

#[test]
fn a_body_the_host_cannot_unescape_is_flagged_not_judged_raw() {
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &[("a.fake", "a=shell: x \\!\n")]);
    let report = run(&root, &Config::default(), &["a.fake"]);
    let v = only(&report);
    assert_eq!(v.rule, Rule::Xenolith);
    assert!(v.why.starts_with("unparseable just string"), "{}", v.why);
}

// ---------------------------------------------------------------------
// allow (`src/config:V9`, `src/config:V10`)
// ---------------------------------------------------------------------

fn allow(path: &str, sink: &str, hash: &str) -> String {
    format!(
        "version = 1\n\n[[allow]]\npath = \"{path}\"\nsink = \"{sink}\"\nhash = \"{hash}\"\n\
         reason = \"fixture\"\n"
    )
}

#[test]
fn an_allowed_site_is_clean() {
    let sandbox = Sandbox::new();
    let root = tree(
        &sandbox,
        &[("a.fake", "build=shell: make && make install\n")],
    );
    let allowed = config(&allow(
        "a.fake",
        "build",
        &body_hash("make && make install"),
    ));
    let report = run(&root, &allowed, &["a.fake"]);
    assert!(report.violations().is_empty(), "{report:?}");
}

#[test]
fn an_allow_whose_body_changed_is_stale_at_the_site() {
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &[("a.fake", "build=shell: make && make check\n")]);
    let old = config(&allow(
        "a.fake",
        "build",
        &body_hash("make && make install"),
    ));
    let report = run(&root, &old, &["a.fake"]);
    assert_eq!(
        rules(&report),
        vec![
            ("a.fake".to_owned(), 1, Rule::StaleAllow),
            ("a.fake".to_owned(), 1, Rule::Xenolith),
        ]
    );
}

#[test]
fn an_allow_whose_site_is_gone_is_stale_at_its_entry() {
    let sandbox = Sandbox::new();
    let text = allow("a.fake", "gone", "0000000000000000");
    let root = tree(
        &sandbox,
        &[("a.fake", "build=shell: ls\n"), ("xenolith.toml", &text)],
    );
    let report = run(&root, &config(&text), &["a.fake"]);
    assert_eq!(
        rules(&report),
        vec![("xenolith.toml".to_owned(), 3, Rule::StaleAllow)]
    );
    assert!(only(&report).why.contains("gone"), "{report:?}");
}

#[test]
fn an_allow_for_a_file_this_run_did_not_scan_is_not_judged() {
    // hk passes the changed files; an allow about another file says
    // nothing about this run.
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &[("a.fake", "build=shell: ls\n"), ("b.fake", "")]);
    let other = config(&allow("b.fake", "gone", "0000000000000000"));
    assert!(run(&root, &other, &["a.fake"]).violations().is_empty());
}

/// `[[allow]]` for `a.fake`'s `build` site, `[parse] host_errors` under
/// `policy`.
fn allow_under(policy: &str) -> String {
    format!(
        "{}[parse]\nhost_errors = \"{policy}\"\n",
        allow("a.fake", "build", &body_hash("a && b"))
    )
}

#[test]
fn an_allow_for_a_file_that_did_not_parse_is_not_stale() {
    // `src/config:V9`: stale means no site matches, and a file its host
    // could not parse had its sites never looked at.
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &[("a.fake", "!\nbuild=shell: a && b\n")]);
    let ignore = config(&allow_under("ignore"));
    assert_eq!(run(&root, &ignore, &["a.fake"]), Report::new());
    std::fs::write(root.join("a.fake"), [0xff, b'\n']).unwrap_or_else(|e| panic!("write: {e}"));
    assert_eq!(run(&root, &ignore, &["a.fake"]), Report::new());
}

#[test]
fn a_whole_tree_run_keeps_the_allows_of_a_file_that_did_not_parse() {
    let sandbox = Sandbox::new();
    let root = sandbox.repo("r");
    write(&root, "a.fake", "!\nbuild=shell: a && b\n");
    sandbox.run_git(&root, &["add", "a.fake"]);
    let report = check_with(
        &root,
        &config(&allow_under("error")),
        &Options::default(),
        &fakes(),
        &|| sandbox.git(),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        rules(&report),
        vec![("a.fake".to_owned(), 1, Rule::HostParseError)]
    );
}

#[test]
fn an_allow_for_a_file_no_host_in_this_build_claims_is_not_stale() {
    // `src:V30`: in a build without the nix host, `x.nix` is unclaimed
    // and never scanned, so its allow cannot be judged -- while an allow
    // naming a file that is gone still is.
    let sandbox = Sandbox::new();
    let root = sandbox.repo("r");
    write(&root, "x.nix", "{ }\n");
    sandbox.run_git(&root, &["add", "x.nix"]);
    let text = format!(
        "{}\n[[allow]]\npath = \"gone.nix\"\nsink = \"s\"\nhash = \"0\"\nreason = \"fixture\"\n",
        allow("x.nix", "s", "0")
    );
    let report = check_with(
        &root,
        &config(&text),
        &Options::default(),
        &fakes(),
        &|| sandbox.git(),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let stale: Vec<&str> = report
        .violations()
        .iter()
        .filter(|v| v.rule == Rule::StaleAllow)
        .map(|v| v.why.as_str())
        .collect();
    assert_eq!(stale.len(), 1, "{stale:?}");
    assert!(
        stale.iter().all(|why| why.contains("gone.nix")),
        "{stale:?}"
    );
}

// ---------------------------------------------------------------------
// candidates and claims (`src:V57`, `src/config:V79`, `src:V13`)
// ---------------------------------------------------------------------

#[test]
fn an_excluded_file_is_never_read() {
    // Not UTF-8 and claimed: read, it would be a host parse warning.
    let sandbox = Sandbox::new();
    let root = sandbox.plain("t");
    std::fs::create_dir_all(root.join("vendor")).unwrap_or_else(|e| panic!("mkdir vendor: {e}"));
    std::fs::write(root.join("vendor/x.fake"), [0xff, 0xfe])
        .unwrap_or_else(|e| panic!("write: {e}"));
    let skip = config("version = 1\n[[exclude]]\nglob = \"vendor\"\nreason = \"third party\"\n");
    let report = run(&root, &skip, &["vendor/x.fake"]);
    assert_eq!(report, Report::new());
    let read = run(&root, &Config::default(), &["vendor/x.fake"]);
    assert_eq!(
        rules(&read),
        vec![("vendor/x.fake".to_owned(), 1, Rule::HostParseError)]
    );
}

#[test]
fn an_absolute_or_dotted_path_under_the_root_is_named_repo_relative() {
    // Allow paths, exclude globs and nested configs all speak
    // repo-relative; a path named `/abs/root/sub/a.fake` or `./a.fake` is
    // the same file and must meet them.
    let sandbox = Sandbox::new();
    let body = "make && make install";
    let sub_toml = format!(
        "version = 1\n[[allow]]\npath = \"a.fake\"\nsink = \"build\"\nhash = \"{}\"\n\
         reason = \"fixture\"\n",
        body_hash(body)
    );
    let root = tree(
        &sandbox,
        &[
            ("sub/a.fake", "build=shell: make && make install\n"),
            ("sub/xenolith.toml", &sub_toml),
            ("gen/b.fake", SCRIPT),
        ],
    );
    let skip = config("version = 1\n[[exclude]]\nglob = \"gen\"\nreason = \"generated\"\n");
    let abs = root.join("sub/a.fake").display().to_string();
    let abs_gen = root.join("gen/b.fake").display().to_string();
    for paths in [vec![abs.as_str(), abs_gen.as_str()], vec!["./sub/a.fake"]] {
        let report = run(&root, &skip, &paths);
        assert_eq!(report, Report::new(), "{paths:?}");
    }
}

#[test]
fn an_absolute_path_outside_the_root_is_refused() {
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &[("a.fake", SCRIPT)]);
    let other = sandbox.plain("elsewhere");
    write(&other, "b.fake", SCRIPT);
    let outside = other.join("b.fake").display().to_string();
    let e = refused(&root, &Config::default(), &[&outside], &fakes());
    assert_eq!(e.exit_code(), 2);
    let text = e.to_string();
    assert!(text.contains(&outside), "{text}");
    assert!(text.contains("outside"), "{text}");
}

#[test]
fn an_unclaimed_file_is_neither_scanned_nor_reported_by_default() {
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &[("notes.md", "build=shell: a && b\n")]);
    assert_eq!(run(&root, &Config::default(), &["notes.md"]), Report::new());
}

#[test]
fn without_paths_the_tracked_files_are_checked() {
    let sandbox = Sandbox::new();
    let root = sandbox.repo("r");
    write(&root, "a.fake", "build=shell: a && b\n");
    write(&root, "untracked.fake", "build=shell: a && b\n");
    sandbox.run_git(&root, &["add", "a.fake"]);
    let report = check_with(
        &root,
        &Config::default(),
        &Options::default(),
        &fakes(),
        &|| sandbox.git(),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        rules(&report),
        vec![("a.fake".to_owned(), 1, Rule::Xenolith)]
    );
}

#[test]
fn outside_git_without_paths_is_refused_with_exit_two() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("p");
    let e = check_with(
        &root,
        &Config::default(),
        &Options::default(),
        &fakes(),
        &|| sandbox.git(),
    )
    .err()
    .unwrap_or_else(|| panic!("outside git, no paths: refused"));
    assert!(matches!(e, CheckError::Discover(_)), "{e:?}");
    assert_eq!(e.exit_code(), 2);
}

#[test]
fn violations_come_out_sorted_whatever_the_order_named() {
    // `src:V11`: file, then line, then column.
    let sandbox = Sandbox::new();
    let root = tree(
        &sandbox,
        &[
            ("b.fake", "x=shell: a | b\n"),
            ("a.fake", "y=shell: a | b\nz=shell: a && b\n"),
        ],
    );
    let report = run(&root, &Config::default(), &["b.fake", "a.fake"]);
    assert_eq!(
        rules(&report),
        vec![
            ("a.fake".to_owned(), 1, Rule::Xenolith),
            ("a.fake".to_owned(), 2, Rule::Xenolith),
            ("b.fake".to_owned(), 1, Rule::Xenolith),
        ]
    );
    assert_eq!(
        report.to_json(),
        run(&root, &Config::default(), &["a.fake", "b.fake"]).to_json()
    );
}

// ---------------------------------------------------------------------
// policies: `[langs] missing_guest` (`src:V42`), `[parse] host_errors`
// ---------------------------------------------------------------------

fn shell_only_host() -> Langs<'static> {
    Langs {
        hosts: HOSTS,
        guests: &[],
    }
}

#[test]
fn a_compiled_out_guest_refuses_by_default_naming_its_feature() {
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &[("a.fake", "build=shell: ls\n")]);
    let e = refused(&root, &Config::default(), &["a.fake"], &shell_only_host());
    assert!(matches!(e, CheckError::MissingGuest(_)), "{e:?}");
    assert_eq!(e.exit_code(), 2);
    assert!(e.to_string().contains("`lang-shell`"), "{e}");
}

#[test]
fn a_compiled_out_guest_under_warn_is_a_warning_and_under_ignore_nothing() {
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &[("a.fake", "build=shell: a && b\n")]);
    let warn = config("version = 1\n[langs]\nmissing_guest = \"warn\"\n");
    let report = run_with(&root, &warn, &["a.fake"], &shell_only_host());
    assert!(report.violations().is_empty(), "never guessed about");
    let codes: Vec<&str> = report.warnings().iter().map(|w| w.code.as_str()).collect();
    assert_eq!(codes, vec!["missing-guest"]);
    let ignore = config("version = 1\n[langs]\nmissing_guest = \"ignore\"\n");
    let report = run_with(&root, &ignore, &["a.fake"], &shell_only_host());
    assert_eq!(report, Report::new());
}

#[test]
fn a_host_parse_error_is_a_violation_by_default() {
    // `src/config` §I: a file xenolith could not read was not checked,
    // and a gate must not read that as clean.
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &[("a.fake", "!\n")]);
    let report = run(&root, &Config::default(), &["a.fake"]);
    assert_eq!(
        rules(&report),
        vec![("a.fake".to_owned(), 1, Rule::HostParseError)]
    );
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn a_file_that_is_not_utf8_is_a_host_parse_error_by_default() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("t");
    std::fs::write(root.join("a.fake"), [b'x', 0xff, b'\n'])
        .unwrap_or_else(|e| panic!("write: {e}"));
    let report = run(&root, &Config::default(), &["a.fake"]);
    assert_eq!(
        rules(&report),
        vec![("a.fake".to_owned(), 1, Rule::HostParseError)]
    );
}

#[test]
fn a_host_parse_error_follows_parse_host_errors() {
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &[("a.fake", "!\n")]);
    let warn = config("version = 1\n[parse]\nhost_errors = \"warn\"\n");
    let warned = run(&root, &warn, &["a.fake"]);
    let codes: Vec<&str> = warned.warnings().iter().map(|w| w.code.as_str()).collect();
    assert_eq!(codes, vec!["host-parse-error"]);
    assert_eq!(warned.exit_code(), 0);
    let error = config("version = 1\n[parse]\nhost_errors = \"error\"\n");
    assert_eq!(
        rules(&run(&root, &error, &["a.fake"])),
        vec![("a.fake".to_owned(), 1, Rule::HostParseError)]
    );
    let ignore = config("version = 1\n[parse]\nhost_errors = \"ignore\"\n");
    assert_eq!(run(&root, &ignore, &["a.fake"]), Report::new());
}

// ---------------------------------------------------------------------
// unclaimed files (`src:V13`, `src:T75`)
// ---------------------------------------------------------------------

/// The shape hk hands over: `{{files}}` is every staged file, and most of
/// them are in no language a host claims.
const HK_FILES: &[&str] = &["a.fake", "docs/README.md", "logo.png"];

fn hk_tree(sandbox: &Sandbox) -> PathBuf {
    let root = tree(
        sandbox,
        &[
            ("a.fake", "build=shell: a && b\n"),
            ("docs/README.md", "# readme\n"),
        ],
    );
    std::fs::write(root.join("logo.png"), [0x89, b'P', b'N', b'G', 0xff, 0x00])
        .unwrap_or_else(|e| panic!("write logo.png: {e}"));
    root
}

fn strict(paths: &[&str]) -> Options {
    Options {
        paths: paths.iter().map(PathBuf::from).collect(),
        strict_hosts: true,
    }
}

#[test]
fn by_default_unclaimed_files_are_neither_scanned_nor_mentioned() {
    let sandbox = Sandbox::new();
    let root = hk_tree(&sandbox);
    let report = run(&root, &Config::default(), HK_FILES);
    assert_eq!(
        rules(&report),
        vec![("a.fake".to_owned(), 1, Rule::Xenolith)]
    );
    assert!(report.warnings().is_empty(), "{report:?}");
}

#[test]
fn unclaimed_warn_is_one_warning_per_file_and_no_exit_code() {
    let sandbox = Sandbox::new();
    let root = hk_tree(&sandbox);
    let warn = config("version = 1\n[langs]\nunclaimed = \"warn\"\n");
    let report = run(&root, &warn, HK_FILES);
    let warned: Vec<(&str, String)> = report
        .warnings()
        .iter()
        .map(|w| {
            let file = w.file.as_deref().map(|f| f.display().to_string());
            (w.code.as_str(), file.unwrap_or_default())
        })
        .collect();
    assert_eq!(
        warned,
        vec![
            ("host-unsupported", "docs/README.md".to_owned()),
            ("host-unsupported", "logo.png".to_owned()),
        ]
    );
    assert!(
        report
            .warnings()
            .iter()
            .all(|w| w.message.contains("host unsupported")),
        "{report:?}"
    );
    // The claimed file is still judged.
    assert_eq!(
        rules(&report),
        vec![("a.fake".to_owned(), 1, Rule::Xenolith)]
    );
}

#[test]
fn unclaimed_error_refuses_with_exit_two_host_unsupported() {
    let sandbox = Sandbox::new();
    let root = hk_tree(&sandbox);
    let error = config("version = 1\n[langs]\nunclaimed = \"error\"\n");
    let e = refused(&root, &error, HK_FILES, &fakes());
    assert_eq!(e.exit_code(), 2);
    let text = e.to_string();
    assert!(
        text.starts_with("docs/README.md: host unsupported"),
        "the first unclaimed file, in report order: {text}"
    );
}

#[test]
fn strict_hosts_refuses_whatever_the_config_says() {
    let sandbox = Sandbox::new();
    let root = hk_tree(&sandbox);
    let ignore = config("version = 1\n[langs]\nunclaimed = \"ignore\"\n");
    let e = check_with(&root, &ignore, &strict(HK_FILES), &fakes(), &|| {
        std::process::Command::new("false")
    })
    .err()
    .unwrap_or_else(|| panic!("--strict-hosts refuses an unclaimed file"));
    assert!(matches!(e, CheckError::Unclaimed { .. }), "{e:?}");
    // Every file claimed: strict has nothing to refuse.
    let clean = check_with(&root, &ignore, &strict(&["a.fake"]), &fakes(), &|| {
        std::process::Command::new("false")
    });
    assert!(clean.is_ok(), "{clean:?}");
}

#[test]
fn a_file_of_a_compiled_out_host_names_the_feature_under_strict() {
    // `src:V30`: the fake build has no nix host, so a `.nix` file is
    // unclaimed, and the refusal says which feature would claim it.
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &[("default.nix", "{ }\n")]);
    let e = check_with(
        &root,
        &Config::default(),
        &strict(&["default.nix"]),
        &fakes(),
        &|| std::process::Command::new("false"),
    )
    .err()
    .unwrap_or_else(|| panic!("refused"));
    let text = e.to_string();
    assert!(text.contains("`lang-nix`"), "{text}");
    // The same under `[langs] unclaimed = "error"`.
    let error = config("version = 1\n[langs]\nunclaimed = \"error\"\n");
    let e = refused(&root, &error, &["default.nix"], &fakes());
    assert!(e.to_string().contains("`lang-nix`"), "{e}");
    // An extension naming no language names no feature.
    write(&root, "x.txt", "");
    let e = refused(&root, &error, &["x.txt"], &fakes());
    assert!(!e.to_string().contains("lang-"), "{e}");
}

// ---------------------------------------------------------------------
// nested configs (`src/config` §I discovery, `src/config:V88`, `.:T91`)
// ---------------------------------------------------------------------

/// The same `&&` script in the root, in `sub/` and in `subway/`, which
/// shares a prefix with `sub` and is not under it.
const SCRIPT: &str = "build=shell: a && b\n";

fn nested_tree(sandbox: &Sandbox, sub_toml: &str) -> PathBuf {
    tree(
        sandbox,
        &[
            ("a.fake", SCRIPT),
            ("sub/b.fake", SCRIPT),
            ("sub/deep/c.fake", SCRIPT),
            ("subway/d.fake", SCRIPT),
            ("sub/xenolith.toml", sub_toml),
        ],
    )
}

const NESTED: &[&str] = &["a.fake", "sub/b.fake", "sub/deep/c.fake", "subway/d.fake"];

#[test]
fn a_nested_config_governs_its_subtree_and_nothing_else() {
    let sandbox = Sandbox::new();
    let root = nested_tree(
        &sandbox,
        "version = 1\n[threshold.shell]\nallow = [\"and-or\"]\n",
    );
    let report = run(&root, &Config::default(), NESTED);
    assert_eq!(
        rules(&report),
        vec![
            ("a.fake".to_owned(), 1, Rule::Xenolith),
            ("subway/d.fake".to_owned(), 1, Rule::Xenolith),
        ]
    );
}

#[test]
fn a_nested_scalar_overrides_the_root_for_its_subtree() {
    let sandbox = Sandbox::new();
    let root = tree(
        &sandbox,
        &[
            ("a.fake", "!\n"),
            ("sub/b.fake", "!\n"),
            (
                "sub/xenolith.toml",
                "version = 1\n[parse]\nhost_errors = \"ignore\"\n",
            ),
        ],
    );
    let error = config("version = 1\n[parse]\nhost_errors = \"error\"\n");
    let report = run(&root, &error, &["a.fake", "sub/b.fake"]);
    assert_eq!(
        rules(&report),
        vec![("a.fake".to_owned(), 1, Rule::HostParseError)]
    );
    assert!(report.warnings().is_empty(), "{report:?}");
}

#[test]
fn a_nested_exclude_glob_is_relative_to_its_file() {
    let sandbox = Sandbox::new();
    let root = tree(
        &sandbox,
        &[
            ("gen/x.fake", SCRIPT),
            ("sub/gen/x.fake", SCRIPT),
            (
                "sub/xenolith.toml",
                "version = 1\n[[exclude]]\nglob = \"gen\"\nreason = \"generated\"\n",
            ),
        ],
    );
    let report = run(&root, &Config::default(), &["gen/x.fake", "sub/gen/x.fake"]);
    assert_eq!(
        rules(&report),
        vec![("gen/x.fake".to_owned(), 1, Rule::Xenolith)]
    );
}

#[test]
fn a_nested_allow_path_is_relative_to_its_file_and_holds_standalone() {
    // `src/config:V88`: the same verdict from the root and from inside
    // `sub` with no ancestor read, since the entry says `b.fake`, not
    // `sub/b.fake`.
    let sandbox = Sandbox::new();
    let sub_toml = format!(
        "version = 1\n[[allow]]\npath = \"b.fake\"\nsink = \"build\"\nhash = \"{}\"\n\
         reason = \"fixture\"\n",
        body_hash("a && b")
    );
    let root = nested_tree(&sandbox, &sub_toml);
    let from_root = run(
        &root,
        &Config::default(),
        &["sub/b.fake", "sub/deep/c.fake"],
    );
    assert_eq!(
        rules(&from_root),
        vec![("sub/deep/c.fake".to_owned(), 1, Rule::Xenolith)]
    );
    let standalone = run(
        &root.join("sub"),
        &config(&sub_toml),
        &["b.fake", "deep/c.fake"],
    );
    assert_eq!(
        rules(&standalone),
        vec![("deep/c.fake".to_owned(), 1, Rule::Xenolith)]
    );
}

#[test]
fn the_allow_direction_names_the_nearest_config_and_a_path_relative_to_it() {
    let sandbox = Sandbox::new();
    let root = nested_tree(&sandbox, "version = 1\n");
    let report = run(&root, &Config::default(), &["sub/deep/c.fake"]);
    let action = only(&report)
        .directions
        .last()
        .map(|d| d.action.clone())
        .unwrap_or_default();
    assert!(action.contains("path = \"deep/c.fake\""), "{action}");
    assert!(action.ends_with("in sub/xenolith.toml"), "{action}");
}

#[test]
fn a_stale_allow_in_a_nested_file_is_reported_at_that_file() {
    let sandbox = Sandbox::new();
    let root = nested_tree(
        &sandbox,
        "version = 1\n\n[[allow]]\npath = \"b.fake\"\nsink = \"gone\"\nhash = \"0\"\n\
         reason = \"fixture\"\n",
    );
    let report = run(&root, &Config::default(), &["sub/b.fake"]);
    let stale: Vec<_> = report
        .violations()
        .iter()
        .filter(|v| v.rule == Rule::StaleAllow)
        .collect();
    match stale.as_slice() {
        [v] => {
            assert_eq!(
                (v.file.display().to_string(), v.line),
                ("sub/xenolith.toml".into(), 3)
            );
            assert!(v.why.contains("sub/b.fake"), "{}", v.why);
        }
        other => panic!("expected one stale allow, got {other:#?}"),
    }
}

#[test]
fn a_nested_allow_never_covers_a_site_outside_its_subtree() {
    // The root's `b.fake` is not the nested file's `b.fake`.
    let sandbox = Sandbox::new();
    let sub_toml = format!(
        "version = 1\n[[allow]]\npath = \"b.fake\"\nsink = \"build\"\nhash = \"{}\"\n\
         reason = \"fixture\"\n",
        body_hash("a && b")
    );
    let root = tree(
        &sandbox,
        &[("b.fake", SCRIPT), ("sub/xenolith.toml", &sub_toml)],
    );
    let report = run(&root, &Config::default(), &["b.fake"]);
    assert_eq!(
        rules(&report),
        vec![("b.fake".to_owned(), 1, Rule::Xenolith)]
    );
}

#[test]
fn a_broken_nested_config_refuses_with_exit_two_naming_it() {
    let sandbox = Sandbox::new();
    let root = nested_tree(&sandbox, "version = 1\n[langs]\nbogus = 1\n");
    let e = refused(&root, &Config::default(), &["sub/b.fake"], &fakes());
    assert!(matches!(e, CheckError::Config(_)), "{e:?}");
    assert_eq!(e.exit_code(), 2);
    assert!(
        e.to_string().starts_with("sub/xenolith.toml: langs.bogus"),
        "{e}"
    );
}

#[test]
fn a_config_inside_an_excluded_tree_is_never_read() {
    // `src/config:V79`: an excluded file is not read, and a
    // `xenolith.toml` under an excluded directory is one. Broken, it
    // would refuse the run.
    let sandbox = Sandbox::new();
    let root = tree(
        &sandbox,
        &[
            ("a.fake", SCRIPT),
            ("vendor/x.fake", SCRIPT),
            ("vendor/xenolith.toml", "not toml at all"),
            ("sub/gen/y.fake", SCRIPT),
            ("sub/gen/xenolith.toml", "not toml either"),
            (
                "sub/xenolith.toml",
                "version = 1\n[check]\nexclude = [{ glob = \"gen\", reason = \"generated\" }]\n",
            ),
        ],
    );
    let skip = config("version = 1\n[[exclude]]\nglob = \"vendor\"\nreason = \"third party\"\n");
    let report = run(&root, &skip, &["a.fake", "vendor/x.fake", "sub/gen/y.fake"]);
    assert_eq!(
        rules(&report),
        vec![("a.fake".to_owned(), 1, Rule::Xenolith)]
    );
}

#[test]
fn a_whole_tree_run_reports_an_exclude_matching_no_tracked_file() {
    // `src/config:V79`: a skip nobody can see the point of is a skip
    // nobody removes. Only a whole-tree run knows every tracked file.
    let sandbox = Sandbox::new();
    let root = sandbox.repo("r");
    write(&root, "vendor/x.fake", SCRIPT);
    write(
        &root,
        "sub/xenolith.toml",
        "version = 1\n[lint]\nexclude = [{ glob = \"gone\", reason = \"old\" }]\n",
    );
    sandbox.run_git(&root, &["add", "."]);
    let root_toml = config(
        "version = 1\n[[exclude]]\nglob = \"vendor\"\nreason = \"third party\"\n\
         [[exclude]]\nglob = \"build\"\nreason = \"output\"\n",
    );
    let report = check_with(&root, &root_toml, &Options::default(), &fakes(), &|| {
        sandbox.git()
    })
    .unwrap_or_else(|e| panic!("{e}"));
    let stale: Vec<(String, bool)> = report
        .warnings()
        .iter()
        .filter(|w| w.code == "stale-exclude")
        .map(|w| {
            let file = w.file.as_deref().map(|f| f.display().to_string());
            (file.unwrap_or_default(), w.message.contains("exclude[1]"))
        })
        .collect();
    assert_eq!(
        stale,
        vec![
            ("sub/xenolith.toml".to_owned(), false),
            ("xenolith.toml".to_owned(), true),
        ]
    );
    // Named paths are a partial view: nothing is judged stale.
    let named = run(&root, &root_toml, &["sub/xenolith.toml"]);
    assert!(named.warnings().is_empty(), "{named:?}");
}

#[test]
fn a_whole_tree_run_reads_nested_configs_too() {
    let sandbox = Sandbox::new();
    let root = sandbox.repo("r");
    write(&root, "a.fake", SCRIPT);
    write(&root, "sub/b.fake", SCRIPT);
    write(
        &root,
        "sub/xenolith.toml",
        "version = 1\n[threshold.shell]\nallow = [\"and-or\"]\n",
    );
    sandbox.run_git(&root, &["add", "."]);
    let report = check_with(
        &root,
        &Config::default(),
        &Options::default(),
        &fakes(),
        &|| sandbox.git(),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        rules(&report),
        vec![("a.fake".to_owned(), 1, Rule::Xenolith)]
    );
}

// ---------------------------------------------------------------------
// the parts
// ---------------------------------------------------------------------

#[test]
fn body_hash_is_fnv1a_64_in_hex() {
    // Published FNV-1a 64 vectors: the key in a committed `[[allow]]`
    // must mean the same thing on every machine and in every release.
    assert_eq!(body_hash(""), "cbf29ce484222325");
    assert_eq!(body_hash("a"), "af63dc4c8601ec8c");
    assert_eq!(body_hash("foobar"), "85944171f73967e8");
}

#[test]
fn position_is_one_based_and_counts_characters() {
    assert_eq!(position("abc", 0), (1, 1));
    assert_eq!(position("ab\ncd", 3), (2, 1));
    assert_eq!(position("ab\n\u{e9}d", 5), (2, 2));
    assert_eq!(position("x", 99), (1, 2));
}

#[test]
fn guest_text_replaces_every_hole_inside_the_body() {
    let src = "<<a {{x}} b {{y}}>>";
    let site = Site {
        sink: "s".to_owned(),
        guest: LangId::Shell,
        env: GuestEnv::default(),
        delim: Delim {
            kind: DelimKind::JustRecipe,
            open: Span::new(0, 2),
            body: Span::new(2, 17),
            close: Span::new(17, 19),
        },
        holes: vec![Span::new(12, 17), Span::new(4, 9)],
    };
    assert_eq!(guest_text(src, &site), format!("a {HOLE} b {HOLE}"));
}

#[test]
fn head_is_the_first_line_only() {
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &[("s", "#!/bin/sh\necho hi\n")]);
    assert_eq!(head(&root.join("s")), "#!/bin/sh");
    assert_eq!(head(&root.join("missing")), "");
}

#[test]
fn repo_name_drops_dot_components_and_uses_slashes() {
    assert_eq!(repo_name(Path::new("./a/./b.nix")), "a/b.nix");
    assert_eq!(repo_name(Path::new("a.nix")), "a.nix");
}

#[test]
fn under_root_resolves_dots_and_refuses_a_climb_out() {
    let root = Path::new("/r");
    let named = |p: &str| under_root(root, Path::new(p)).ok();
    assert_eq!(named("./a/../b.nix"), Some(PathBuf::from("b.nix")));
    assert_eq!(named("."), Some(PathBuf::from(".")));
    assert_eq!(named("/r/sub/./c.nix"), Some(PathBuf::from("sub/c.nix")));
    assert_eq!(named("/r"), Some(PathBuf::from(".")));
    assert_eq!(named("../x.nix"), None);
    assert_eq!(named("/r/../x.nix"), None);
    assert_eq!(named("/rr/x.nix"), None);
}

// ---------------------------------------------------------------------
// the `src:T153` fixtures, with the real languages
// ---------------------------------------------------------------------

#[cfg(all(feature = "lang-nix", feature = "lang-shell"))]
mod nix_shell {
    use super::{Sandbox, rules, tree};
    use crate::check::{Options, body_hash, check};
    use crate::config::{self, Config};
    use crate::model::Rule;

    const SCRIPT: &str =
        "{\n  systemd.services.a.script = ''\n    make && make install\n  '';\n}\n";
    const COMMAND: &str = "{\n  systemd.services.a.script = \"make install\";\n}\n";

    fn check_nix(text: &str, config: &Config) -> crate::model::Report {
        let sandbox = Sandbox::new();
        let root = tree(&sandbox, &[("service.nix", text)]);
        let options = Options {
            paths: vec!["service.nix".into()],
            ..Options::default()
        };
        check(&root, config, &options).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn a_nix_and_and_script_is_flagged() {
        let report = check_nix(SCRIPT, &Config::default());
        assert_eq!(
            rules(&report),
            vec![("service.nix".to_owned(), 2, Rule::Xenolith)]
        );
        let human = report
            .violations()
            .first()
            .map(crate::model::Violation::to_human)
            .unwrap_or_default();
        assert!(
            human.starts_with(
                "service.nix:2:31 xenolith: shell in nix systemd.services.a.script \
                 (non-trivial shell: and-or"
            ),
            "{human}"
        );
    }

    #[test]
    fn a_nix_file_led_by_a_byte_order_mark_fails_the_run_by_default() {
        // `src/config` §I: the BOM is not nix, so the file was not
        // checked -- a violation, not a clean pass.
        let report = check_nix(&format!("\u{feff}{SCRIPT}"), &Config::default());
        assert_eq!(
            rules(&report),
            vec![("service.nix".to_owned(), 1, Rule::HostParseError)]
        );
    }

    #[test]
    fn a_single_nix_command_is_clean() {
        assert!(
            check_nix(COMMAND, &Config::default())
                .violations()
                .is_empty()
        );
    }

    /// The one violation's `why`, after asserting it is a xenolith one
    /// on `line`.
    fn flagged_at(text: &str, line: usize) -> String {
        let report = check_nix(text, &Config::default());
        assert_eq!(
            rules(&report),
            vec![("service.nix".to_owned(), line, Rule::Xenolith)],
            "{text}"
        );
        report
            .violations()
            .first()
            .map(|v| v.why.clone())
            .unwrap_or_default()
    }

    fn clean(text: &str) {
        let report = check_nix(text, &Config::default());
        assert!(report.violations().is_empty(), "{text}\n{report:?}");
    }

    #[test]
    fn a_three_command_pre_check_is_flagged() {
        // `languages/nix:T156`: a phase hook is shell like a phase.
        let why = flagged_at(
            "{ stdenv }:\nstdenv.mkDerivation {\n  name = \"d\";\n  preCheck = ''\n    \
             export HOME=$TMPDIR\n    mkdir -p \"$HOME/.cache\"\n    patchShebangs tests\n  \
             '';\n}\n",
            4,
        );
        assert!(why.contains("sequence"), "{why}");
    }

    #[test]
    fn a_single_command_post_install_is_clean() {
        clean(
            "{ stdenv }:\nstdenv.mkDerivation {\n  name = \"d\";\n  \
             postInstall = \"installManPage d.1\";\n}\n",
        );
    }

    const XINITRC: &str = "{\n  environment.etc.\"xinitrc\".text = ''\n    #!/bin/sh\n    \
                           xrdb -merge \"$HOME/.Xresources\"\n    xsetroot -solid black\n    \
                           setxkbmap -option ctrl:nocaps\n    xset r rate 200 40\n    \
                           exec i3\n  '';\n}\n";

    #[test]
    fn a_shebang_led_etc_text_is_flagged() {
        // `languages/nix:T157`: five commands under `#!/bin/sh`.
        let why = flagged_at(XINITRC, 2);
        assert!(why.contains("sequence"), "{why}");
    }

    #[test]
    fn a_plain_etc_text_is_not_a_site() {
        clean(&XINITRC.replace("#!/bin/sh", "# sh"));
    }

    #[test]
    fn a_multi_command_zsh_init_content_is_flagged() {
        // `languages/nix:T159`: home-manager's zsh init is zsh.
        let why = flagged_at(
            "{\n  programs.zsh.initContent = ''\n    bindkey -e\n    \
             autoload -U compinit && compinit\n  '';\n}\n",
            2,
        );
        assert!(why.contains("sequence"), "{why}");
    }

    #[test]
    fn a_single_export_in_bash_init_is_clean() {
        clean("{\n  programs.bash.initExtra = \"export PATH=$HOME/bin:$PATH\";\n}\n");
    }

    #[test]
    fn an_indented_heredoc_is_judged_by_construct_not_as_unparseable() {
        // `languages/nix:T158`: nix strips the common indent before bash
        // runs the body, so the terminator IS `EOF`; the guest must see
        // the body that runs, not the host's bytes.
        let why = flagged_at(
            "{ pkgs }:\n{\n  gen = pkgs.writeShellScript \"gen\" ''\n    if [ -n \"''${A:-}\" ]; \
             then\n      cat > out <<EOF\n    hi\n    EOF\n    fi\n  '';\n}\n",
            3,
        );
        assert!(why.contains("heredoc"), "{why}");
        assert!(why.contains("if"), "{why}");
        assert!(!why.contains("unparseable"), "{why}");
    }

    #[test]
    fn a_concatenated_shell_hook_is_flagged_in_its_literal() {
        // `languages/nix:T155`: the literal half of `''…'' + extra`.
        let why = flagged_at(
            "{ pkgs, extra }:\npkgs.mkShell {\n  shellHook = ''\n    [ -f x ] || cmd\n  ''\n  \
             + extra;\n}\n",
            3,
        );
        assert!(why.contains("and-or"), "{why}");
    }

    #[test]
    fn an_allowed_nix_script_is_clean_and_a_stale_allow_is_flagged() {
        let raw = "\n    make && make install\n  ";
        let entry = |hash: &str| {
            config::parse(&format!(
                "version = 1\n[[allow]]\npath = \"service.nix\"\n\
                 sink = \"systemd.services.a.script\"\nhash = \"{hash}\"\nreason = \"legacy\"\n"
            ))
            .unwrap_or_else(|e| panic!("{e}"))
        };
        assert!(
            check_nix(SCRIPT, &entry(&body_hash(raw)))
                .violations()
                .is_empty()
        );
        let stale = check_nix(SCRIPT, &entry("0000000000000000"));
        assert_eq!(
            rules(&stale),
            vec![
                ("service.nix".to_owned(), 2, Rule::StaleAllow),
                ("service.nix".to_owned(), 2, Rule::Xenolith),
            ]
        );
    }
}

#[cfg(all(feature = "lang-pkl", feature = "lang-shell"))]
#[test]
fn a_pkl_hk_step_holding_a_script_is_flagged() {
    // One line per element keeps the fixture readable within the 100-column
    // limit; the joined text is the same bytes the hk file would hold.
    let hk = [
        "amends \"package://github.com/jdx/hk/releases/download/\
         v1.2.0/hk@1.2.0#/Config.pkl\"",
        "",
        "hooks {",
        "  [\"pre-commit\"] {",
        "    steps {",
        "      [\"lint\"] {",
        "        check = \"\"\"",
        "          cargo fmt --check && cargo clippy",
        "        \"\"\"",
        "      }",
        "    }",
        "  }",
        "}",
        "",
    ]
    .join("\n");
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &[("hk.pkl", hk.as_str())]);
    let options = Options {
        paths: vec!["hk.pkl".into()],
        ..Options::default()
    };
    let report =
        super::check(&root, &Config::default(), &options).unwrap_or_else(|e| panic!("{e}"));
    let v = only(&report);
    assert_eq!((v.host, v.guest), (LangId::Pkl, LangId::Shell));
    assert_eq!(v.sink, "lint.check");
    assert!(v.why.contains("and-or"), "{}", v.why);
}

/// `src:T46`'s nix-only build, end to end: a nix site holding shell,
/// with no shell guest, is exit 2 naming `lang-shell` (`src:V42`).
#[cfg(all(feature = "lang-nix", not(feature = "lang-shell")))]
#[test]
fn a_nix_only_build_refuses_a_shell_site_naming_lang_shell() {
    let sandbox = Sandbox::new();
    let root = tree(
        &sandbox,
        &[(
            "service.nix",
            "{ systemd.services.a.script = ''\n  make && make install\n''; }\n",
        )],
    );
    let options = Options {
        paths: vec!["service.nix".into()],
        ..Options::default()
    };
    let e = super::check(&root, &Config::default(), &options)
        .err()
        .unwrap_or_else(|| panic!("shell is compiled out"));
    assert_eq!(e.exit_code(), 2);
    assert!(e.to_string().contains("`lang-shell`"), "{e}");
}
