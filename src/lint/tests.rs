//! The lint engine: the mirror of `src/lint/mod.rs` (`src:C139`).
//!
//! Languages are fakes (as in `src/check/tests.rs`), so every case runs
//! in every feature subset (`src:V30`); tools are stub scripts on a
//! `PATH` the test controls and git is the sandbox's (`tests:V150`), so
//! nothing here depends on which linters the machine has.

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use xenolith_lang_api::{
    Delim, Error, FileArg, Format, Guest, GuestEnv, Host, Invoke, LangId, LintCmd, LoadRef,
    Prelude, Result, Site,
};

use super::{Kind, LintError, LintReport, Options, Outcome, Source, Status, limit, lint_with};
use crate::check::Langs;
use crate::config::{self, Config};
use crate::discover::{Sandbox, write};
use crate::lint::run::Tools;

// ---------------------------------------------------------------------
// stubs and fakes
// ---------------------------------------------------------------------

/// An executable `#!/bin/sh` script `name` in `dir` running `body`. The
/// `PATH` it runs under is the stub dir alone, so a body uses builtins
/// or absolute paths.
pub(super) fn stub(dir: &Path, name: &str, body: &str) {
    let path = dir.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n"))
        .unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
        .unwrap_or_else(|e| panic!("chmod {}: {e}", path.display()));
}

/// The absolute path of `name` on the TEST process's `PATH` (`src/lint:B2`).
/// A stub runs under the stub dir alone, so it names tools absolutely --
/// and `/bin/<tool>` exists on macOS but not in the Linux nix build
/// sandbox, which has only `/bin/sh`. The test's own `PATH` always does.
pub(super) fn on_path(name: &str) -> String {
    std::env::var_os("PATH")
        .and_then(|p| {
            std::env::split_paths(&p)
                .map(|d| d.join(name))
                .find(|c| c.is_file())
        })
        .map_or_else(
            || panic!("`{name}` is not on the test's PATH"),
            |p| p.display().to_string(),
        )
}

fn cmd(argv: &[&str]) -> LintCmd {
    LintCmd {
        argv: argv.iter().map(|a| (*a).to_owned()).collect(),
        file_arg: FileArg::Append,
        format: Format::Raw,
    }
}

/// A guest like shell: extracts end `.sh`, two checks -- the first
/// saying which dialect it was given -- and one fixer.
struct FakeGuest;

impl Guest for FakeGuest {
    fn id(&self) -> LangId {
        LangId::Shell
    }

    fn extension(&self, _env: &GuestEnv) -> &'static str {
        "sh"
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
        Prelude::default()
    }

    fn executable(&self) -> bool {
        true
    }

    fn checks(&self, env: &GuestEnv) -> Vec<LintCmd> {
        let dialect = format!("--dialect={}", env.dialect.as_deref().unwrap_or("none"));
        // beta reads as shellcheck's JSON: a stub printing prose is
        // unparseable and keeps its tail (`src/lint:V92`).
        let beta = LintCmd {
            format: Format::Json("shellcheck"),
            ..cmd(&["beta"])
        };
        vec![cmd(&["alpha", &dialect]), beta]
    }

    fn fixers(&self, _env: &GuestEnv) -> Vec<LintCmd> {
        vec![cmd(&["fixit"])]
    }
}

/// A host claiming `*.nx`, with one check and one fixer of its own.
struct FakeHost;

impl Host for FakeHost {
    fn id(&self) -> LangId {
        LangId::Nix
    }

    fn claims(&self, path: &Path, _head: &str) -> bool {
        path.extension().is_some_and(|e| e == "nx")
    }

    fn sites(&self, _src: &str) -> Result<Vec<Site>> {
        Ok(Vec::new())
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
        Ok(raw.to_owned())
    }

    fn checks(&self) -> Vec<LintCmd> {
        vec![cmd(&["hostcheck"])]
    }

    fn fixers(&self) -> Vec<LintCmd> {
        vec![cmd(&["hostfix"])]
    }
}

fn fakes() -> Langs<'static> {
    Langs {
        hosts: &[&FakeHost],
        guests: &[&FakeGuest],
    }
}

/// A sandbox with a `root` to lint and a `bin` of stub tools.
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
        Fixture { sandbox, root, bin }
    }

    /// Every fake tool present and passing.
    fn all_pass() -> Fixture {
        let fixture = Fixture::new();
        for tool in ["alpha", "beta", "fixit", "hostcheck", "hostfix"] {
            fixture.tool(tool, "exit 0");
        }
        fixture
    }

    fn tool(&self, name: &str, body: &str) {
        stub(&self.bin, name, body);
    }

    fn file(&self, rel: &str, contents: &str) {
        write(&self.root, rel, contents);
    }

    fn lint(
        &self,
        config: &Config,
        options: &Options,
    ) -> std::result::Result<LintReport, LintError> {
        lint_with(
            &self.root,
            config,
            options,
            &fakes(),
            &|| self.sandbox.git(),
            &Tools::on_path(&self.bin),
        )
    }

    fn report(&self, config: &Config, paths: &[&str]) -> LintReport {
        self.lint(config, &named(paths))
            .unwrap_or_else(|e| panic!("lint refused: {e}"))
    }
}

fn named(paths: &[&str]) -> Options {
    Options {
        paths: paths.iter().map(PathBuf::from).collect(),
        ..Options::default()
    }
}

fn parsed(text: &str) -> Config {
    config::parse(text).unwrap_or_else(|e| panic!("config: {e}"))
}

fn checks(report: &LintReport) -> Vec<(String, Status)> {
    report
        .outcomes()
        .iter()
        .map(|o| (o.check.clone(), o.status))
        .collect()
}

fn nth(report: &LintReport, n: usize) -> &Outcome {
    report
        .outcomes()
        .get(n)
        .unwrap_or_else(|| panic!("no outcome {n}: {:?}", report.outcomes()))
}

fn pair(check: &str, status: Status) -> (String, Status) {
    (check.to_owned(), status)
}

// ---------------------------------------------------------------------
// T24: the linter map, defaults, a missing binary
// ---------------------------------------------------------------------

#[test]
fn an_extract_by_extension_runs_each_default_check_in_order() {
    let fx = Fixture::all_pass();
    fx.file("a.sh", "echo hi\n");
    let report = fx.report(&Config::default(), &["a.sh"]);
    assert_eq!(
        checks(&report),
        [pair("alpha", Status::Pass), pair("beta", Status::Pass)]
    );
    let first = nth(&report, 0);
    assert_eq!(first.kind, Kind::Extract);
    assert_eq!(first.guest, Some(LangId::Shell));
    assert_eq!(first.dialect, None);
    assert_eq!(first.source, Source::Default);
    assert_eq!(first.argv, ["alpha", "--dialect=none", "a.sh"]);
    assert_eq!(report.exit_code(), 0);
}

#[test]
fn a_shebang_names_the_guest_and_its_dialect() {
    let fx = Fixture::all_pass();
    fx.file("bin/tool", "#!/usr/bin/env zsh\necho hi\n");
    let report = fx.report(&Config::default(), &["bin/tool"]);
    let first = nth(&report, 0);
    assert_eq!(first.dialect.as_deref(), Some("zsh"));
    assert_eq!(first.argv, ["alpha", "--dialect=zsh", "bin/tool"]);
}

#[test]
fn a_shebang_naming_no_language_wins_over_the_extension() {
    let fx = Fixture::all_pass();
    fx.file("x.sh", "#!/usr/bin/env tclsh\nputs hi\n");
    let report = fx.report(&Config::default(), &["x.sh"]);
    assert!(report.outcomes().is_empty(), "{:?}", report.outcomes());
}

#[test]
fn a_shebang_naming_a_guest_this_build_lacks_is_a_warning() {
    let fx = Fixture::all_pass();
    fx.file("x.py", "#!/usr/bin/env python3\nprint(1)\n");
    let report = fx.report(&Config::default(), &["x.py"]);
    assert!(report.outcomes().is_empty());
    let codes: Vec<&str> = report.warnings().iter().map(|w| w.code.as_str()).collect();
    assert_eq!(codes, ["missing-guest"]);
}

#[test]
fn a_tool_not_on_path_is_an_error_exit_2_and_the_rest_still_run() {
    // `src/lint:V8`: named, never skipped, and not the end of the run.
    let fx = Fixture::new();
    fx.tool("beta", "exit 0");
    fx.file("a.sh", "echo hi\n");
    let report = fx.report(&Config::default(), &["a.sh"]);
    assert_eq!(
        checks(&report),
        [pair("alpha", Status::Error), pair("beta", Status::Pass)]
    );
    let why = nth(&report, 0).raw_tail.clone().unwrap_or_default();
    assert!(why.contains("`alpha`") && why.contains("install"), "{why}");
    assert_eq!(report.exit_code(), 2);
}

#[test]
fn a_failing_check_is_a_fail_exit_1() {
    let fx = Fixture::all_pass();
    fx.tool("beta", "echo \"beta: $1: bad\"\nexit 1");
    fx.file("a.sh", "echo hi\n");
    let report = fx.report(&Config::default(), &["a.sh"]);
    let beta = nth(&report, 1);
    assert_eq!(beta.status, Status::Fail);
    assert_eq!(beta.exit, Some(1));
    assert_eq!(beta.raw_tail.as_deref(), Some("beta: a.sh: bad"));
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn a_host_file_runs_its_host_checks_unless_hosts_is_off() {
    let fx = Fixture::all_pass();
    fx.file("a.nx", "{}\n");
    let report = fx.report(&Config::default(), &["a.nx"]);
    assert_eq!(checks(&report), [pair("hostcheck", Status::Pass)]);
    let only = nth(&report, 0);
    assert_eq!(only.kind, Kind::Host);
    assert_eq!(only.guest, None);
    let off = parsed("version = 1\n[lint]\nhosts = false\n");
    assert!(fx.report(&off, &["a.nx"]).outcomes().is_empty());
}

#[test]
fn a_file_nothing_lints_follows_unclaimed() {
    let fx = Fixture::all_pass();
    fx.file("notes.txt", "hi\n");
    let quiet = fx.report(&Config::default(), &["notes.txt"]);
    assert!(quiet.outcomes().is_empty() && quiet.warnings().is_empty());
    let warn = parsed("version = 1\n[langs]\nunclaimed = \"warn\"\n");
    let codes: Vec<String> = fx
        .report(&warn, &["notes.txt"])
        .warnings()
        .iter()
        .map(|w| w.code.clone())
        .collect();
    assert_eq!(codes, ["host-unsupported"]);
    let strict = Options {
        strict_hosts: true,
        ..named(&["notes.txt"])
    };
    let refused = fx.lint(&Config::default(), &strict);
    assert!(
        matches!(&refused, Err(LintError::Unclaimed { file }) if file == Path::new("notes.txt")),
        "{refused:?}"
    );
    assert!(refused.is_err_and(|e| e.exit_code() == 2));
}

#[test]
fn an_excluded_file_has_no_result() {
    let fx = Fixture::all_pass();
    fx.file("skip.sh", "echo hi\n");
    let config =
        parsed("version = 1\n[lint]\nexclude = [{ glob = \"skip.sh\", reason = \"vendored\" }]\n");
    assert!(fx.report(&config, &["skip.sh"]).outcomes().is_empty());
}

#[test]
fn a_config_command_without_trust_is_skipped_with_a_warning_and_defaults_run() {
    // `src/lint:V91`.
    let fx = Fixture::all_pass();
    fx.file("a.sh", "echo hi\n");
    let config = parsed(
        "version = 1\n[lint]\nall = [\"typos {file}\"]\n[lint.shell]\nchecks = [\"own\"]\n\
         extend = false\n",
    );
    let report = fx.report(&config, &["a.sh"]);
    assert_eq!(
        checks(&report),
        [
            pair("own", Status::Skipped),
            pair("typos", Status::Skipped),
            pair("alpha", Status::Pass),
            pair("beta", Status::Pass),
        ]
    );
    let skipped = nth(&report, 1);
    assert_eq!(skipped.source, Source::Config);
    assert_eq!(skipped.argv, ["typos", "a.sh"]);
    let messages: Vec<&str> = report
        .warnings()
        .iter()
        .filter(|w| w.code == "untrusted-command")
        .map(|w| w.message.as_str())
        .collect();
    assert_eq!(messages.len(), 2, "{messages:?}");
    assert!(
        messages.iter().any(|m| m.contains("`typos {file}`")),
        "{messages:?}"
    );
    assert_eq!(report.exit_code(), 0);
}

#[test]
fn a_named_path_outside_the_root_is_refused() {
    let fx = Fixture::all_pass();
    let elsewhere = fx.sandbox.plain("elsewhere");
    write(&elsewhere, "a.sh", "echo hi\n");
    let path = elsewhere.join("a.sh");
    let options = Options {
        paths: vec![path],
        ..Options::default()
    };
    let refused = fx.lint(&Config::default(), &options);
    assert!(
        matches!(refused, Err(LintError::Outside { .. })),
        "{refused:?}"
    );
}

#[test]
fn the_whole_tree_is_the_tracked_files_in_order() {
    let fx = Fixture::all_pass();
    fx.file("b.sh", "echo b\n");
    fx.file("a.nx", "{}\n");
    fx.file("untracked.sh", "echo u\n");
    fx.sandbox.run_git(&fx.root, &["init", "-q"]);
    fx.sandbox.run_git(&fx.root, &["add", "b.sh", "a.nx"]);
    let report = fx
        .lint(&Config::default(), &Options::default())
        .unwrap_or_else(|e| panic!("{e}"));
    let files: Vec<String> = report
        .outcomes()
        .iter()
        .map(|o| o.file.display().to_string())
        .collect();
    assert_eq!(files, ["a.nx", "b.sh", "b.sh"]);
}

// ---------------------------------------------------------------------
// B1: a host file that is also an extract of the host's own language
// ---------------------------------------------------------------------

/// A host of the guest's own language, like shell's and tcl's: claims
/// `*.sh` and a shell shebang, its first check the very argv the guest
/// runs on a shebang-less `.sh`, and sites it cannot read.
struct SameLangHost;

impl Host for SameLangHost {
    fn id(&self) -> LangId {
        LangId::Shell
    }

    fn claims(&self, path: &Path, head: &str) -> bool {
        path.extension().is_some_and(|e| e == "sh")
            || xenolith_lang_api::shebang::parse(head)
                .is_some_and(|line| xenolith_lang_api::shebang::resolves_to(&line, LangId::Shell))
    }

    fn sites(&self, _src: &str) -> Result<Vec<Site>> {
        Err(Error::parse(LangId::Shell, "this fake reads no sites"))
    }

    fn loads(&self, _src: &str) -> Result<Vec<LoadRef>> {
        Err(Error::unsupported(LangId::Shell, "loads"))
    }

    fn rewrite(&self, _: &str, _: &Site, _: &Invoke, _: &Path) -> Result<String> {
        Err(Error::unsupported(LangId::Shell, "rewrite"))
    }

    fn inline(&self, _: &str, _: &LoadRef, _: &str) -> Result<String> {
        Err(Error::unsupported(LangId::Shell, "inline"))
    }

    fn unescape(&self, _: &Delim, raw: &str) -> Result<String> {
        Ok(raw.to_owned())
    }

    fn checks(&self) -> Vec<LintCmd> {
        vec![cmd(&["alpha", "--dialect=none"]), cmd(&["hostcheck"])]
    }

    fn fixers(&self) -> Vec<LintCmd> {
        vec![cmd(&["hostfix"])]
    }
}

/// Both hosts: the guest's own language's, and another's.
fn same_lang() -> Langs<'static> {
    Langs {
        hosts: &[&FakeHost, &SameLangHost],
        guests: &[&FakeGuest],
    }
}

impl Fixture {
    fn same_lang(&self, options: &Options) -> LintReport {
        lint_with(
            &self.root,
            &Config::default(),
            options,
            &same_lang(),
            &|| self.sandbox.git(),
            &Tools::on_path(&self.bin),
        )
        .unwrap_or_else(|e| panic!("lint refused: {e}"))
    }
}

fn kinds(report: &LintReport) -> Vec<(String, Kind)> {
    report
        .outcomes()
        .iter()
        .map(|o| (o.check.clone(), o.kind))
        .collect()
}

fn kinded(check: &str, kind: Kind) -> (String, Kind) {
    (check.to_owned(), kind)
}

#[test]
fn a_file_its_own_language_hosts_is_linted_once_as_the_extract() {
    // `src/lint` §I targets, `src/lint:B1`: `xenolith-tcl-syntax` ran
    // twice on every `.tcl`, shellcheck twice on every `.sh`.
    let fx = Fixture::all_pass();
    fx.file("a.sh", "echo hi\n");
    let report = fx.same_lang(&named(&["a.sh"]));
    assert_eq!(
        kinds(&report),
        [
            kinded("alpha", Kind::Extract),
            kinded("beta", Kind::Extract)
        ]
    );
    assert_eq!(nth(&report, 0).argv, ["alpha", "--dialect=none", "a.sh"]);
}

#[test]
fn a_shebang_extract_its_host_claims_runs_the_dialect_checks_alone() {
    // A zsh script: the host's dialect-blind checks (shellcheck, which
    // cannot read zsh) must not run beside the guest's `zsh -n`.
    let fx = Fixture::all_pass();
    fx.file("bin/tool", "#!/usr/bin/env zsh\necho hi\n");
    let report = fx.same_lang(&named(&["bin/tool"]));
    assert_eq!(
        kinds(&report),
        [
            kinded("alpha", Kind::Extract),
            kinded("beta", Kind::Extract)
        ]
    );
    assert_eq!(nth(&report, 0).dialect.as_deref(), Some("zsh"));
}

#[test]
fn a_host_of_another_language_still_runs_beside_the_extract() {
    let fx = Fixture::all_pass();
    fx.file("x.nx", "#!/usr/bin/env bash\necho hi\n");
    let report = fx.same_lang(&named(&["x.nx"]));
    assert_eq!(
        kinds(&report),
        [
            kinded("hostcheck", Kind::Host),
            kinded("alpha", Kind::Extract),
            kinded("beta", Kind::Extract),
        ]
    );
}

#[test]
fn the_same_language_host_still_has_its_sites_linted() {
    // Only `Host::checks` gives way; `--sites` still reads the host.
    let fx = Fixture::all_pass();
    fx.file("a.sh", "echo hi\n");
    let options = Options {
        sites: true,
        ..named(&["a.sh"])
    };
    let report = fx.same_lang(&options);
    assert_eq!(
        kinds(&report),
        [
            kinded("alpha", Kind::Extract),
            kinded("beta", Kind::Extract)
        ]
    );
    let codes: Vec<&str> = report.warnings().iter().map(|w| w.code.as_str()).collect();
    assert_eq!(codes, ["site-unlinted"]);
}

// ---------------------------------------------------------------------
// T87: every check reported, `--fix`
// ---------------------------------------------------------------------

fn fixing(paths: &[&str]) -> Options {
    Options {
        fix: true,
        ..named(paths)
    }
}

#[test]
fn two_failing_checks_are_both_reported() {
    // `src/lint:V8`: each reported separately, never stopping at the first.
    let fx = Fixture::all_pass();
    fx.tool("alpha", "echo alpha-said\nexit 1");
    fx.tool("beta", "echo beta-said\nexit 2");
    fx.file("a.sh", "echo hi\n");
    let report = fx.report(&Config::default(), &["a.sh"]);
    assert_eq!(
        checks(&report),
        [pair("alpha", Status::Fail), pair("beta", Status::Fail)]
    );
    assert_eq!(nth(&report, 0).raw_tail.as_deref(), Some("alpha-said"));
    assert_eq!(nth(&report, 1).exit, Some(2));
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn fix_runs_the_fixers_then_checks_again() {
    let fx = Fixture::all_pass();
    fx.tool("fixit", "printf 'fixed\\n' > \"$1\"");
    fx.tool("alpha", "read -r line < \"$2\"; test \"$line\" = fixed");
    fx.file("a.sh", "broken\n");
    let before = fx.report(&Config::default(), &["a.sh"]);
    assert_eq!(nth(&before, 0).status, Status::Fail);
    let after = fx
        .lint(&Config::default(), &fixing(&["a.sh"]))
        .unwrap_or_else(|e| panic!("{e}"));
    // A fixer that passed is not listed (`src/lint` §I, status).
    assert_eq!(
        checks(&after),
        [pair("alpha", Status::Pass), pair("beta", Status::Pass)]
    );
    let text = fs::read_to_string(fx.root.join("a.sh")).unwrap_or_default();
    assert_eq!(text, "fixed\n");
}

#[test]
fn a_fixer_that_fails_is_listed_and_the_checks_still_run() {
    let fx = Fixture::all_pass();
    fx.tool("fixit", "echo cannot\nexit 5");
    fx.file("a.sh", "echo hi\n");
    let report = fx
        .lint(&Config::default(), &fixing(&["a.sh"]))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        checks(&report),
        [
            pair("fixit", Status::Fail),
            pair("alpha", Status::Pass),
            pair("beta", Status::Pass),
        ]
    );
    assert!(nth(&report, 0).fixer);
    assert!(!nth(&report, 1).fixer);
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn fix_never_rewrites_a_host_file() {
    // `src/lint:V8`: `--fix` touches extracts only.
    let fx = Fixture::all_pass();
    fx.tool("hostfix", ": > hostfix-ran");
    fx.file("a.nx", "{}\n");
    let report = fx
        .lint(&Config::default(), &fixing(&["a.nx"]))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(checks(&report), [pair("hostcheck", Status::Pass)]);
    assert!(!fx.root.join("hostfix-ran").exists());
}

#[test]
fn an_untrusted_config_fixer_is_skipped_with_a_warning() {
    let fx = Fixture::all_pass();
    fx.file("a.sh", "echo hi\n");
    let config = parsed("version = 1\n[lint.shell]\nfixers = [\"ownfix\"]\n");
    let report = fx
        .lint(&config, &fixing(&["a.sh"]))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(nth(&report, 0).check, "ownfix");
    assert_eq!(nth(&report, 0).status, Status::Skipped);
    assert!(nth(&report, 0).fixer);
    assert!(
        report
            .warnings()
            .iter()
            .any(|w| w.code == "untrusted-command" && w.message.contains("`ownfix`")),
        "{:?}",
        report.warnings()
    );
}

// ---------------------------------------------------------------------
// T92: `--trust-config`
// ---------------------------------------------------------------------

fn trusting(paths: &[&str]) -> Options {
    Options {
        trust_config: true,
        ..named(paths)
    }
}

#[test]
fn trusted_config_checks_run_after_the_defaults_they_extend() {
    // `src/lint:V91`: the same config as the untrusted case, run.
    let fx = Fixture::all_pass();
    fx.tool("own", "exit 0");
    fx.tool("typos", "echo \"typo in $1\"\nexit 1");
    fx.file("a.sh", "echo hi\n");
    let config =
        parsed("version = 1\n[lint]\nall = [\"typos {file}\"]\n[lint.shell]\nchecks = [\"own\"]\n");
    let report = fx
        .lint(&config, &trusting(&["a.sh"]))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        checks(&report),
        [
            pair("alpha", Status::Pass),
            pair("beta", Status::Pass),
            pair("own", Status::Pass),
            pair("typos", Status::Fail),
        ]
    );
    let typos = nth(&report, 3);
    assert_eq!(typos.source, Source::Config);
    assert_eq!(typos.argv, ["typos", "a.sh"]);
    assert_eq!(typos.raw_tail.as_deref(), Some("typo in a.sh"));
    assert!(report.warnings().is_empty(), "{:?}", report.warnings());
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn trusted_config_with_extend_false_replaces_the_defaults() {
    let fx = Fixture::all_pass();
    fx.tool("own", "exit 0");
    fx.file("a.sh", "echo hi\n");
    let config = parsed("version = 1\n[lint.shell]\nchecks = [\"own\"]\nextend = false\n");
    let report = fx
        .lint(&config, &trusting(&["a.sh"]))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(checks(&report), [pair("own", Status::Pass)]);
}

#[test]
fn a_trusted_config_fixer_runs_under_fix() {
    let fx = Fixture::all_pass();
    fx.tool("ownfix", ": > ownfix-ran");
    fx.file("a.sh", "echo hi\n");
    let config = parsed("version = 1\n[lint.shell]\nfixers = [\"ownfix\"]\n");
    let options = Options {
        fix: true,
        ..trusting(&["a.sh"])
    };
    let report = fx.lint(&config, &options).unwrap_or_else(|e| panic!("{e}"));
    assert!(fx.root.join("ownfix-ran").exists());
    assert!(report.warnings().is_empty(), "{:?}", report.warnings());
}

// ---------------------------------------------------------------------
// T93: findings
// ---------------------------------------------------------------------

#[test]
fn a_json_check_is_parsed_into_findings_and_its_tail_dropped() {
    // `src/lint:V92`: shellcheck's `--format=json` counts a tab to the
    // next stop of 8; the finding reports the editor's column.
    let fx = Fixture::all_pass();
    fx.tool(
        "beta",
        "echo '[{\"line\":2,\"column\":15,\"level\":\"info\",\"code\":2086,\
         \"message\":\"quote\"}]'\necho chatter >&2\nexit 1",
    );
    fx.file("a.sh", "a=1\n\techo  $a\n");
    let report = fx.report(&Config::default(), &["a.sh"]);
    let beta = nth(&report, 1);
    assert_eq!(beta.status, Status::Fail);
    assert_eq!(beta.raw_tail, None);
    let found: Vec<(usize, usize, &str)> = beta
        .findings
        .iter()
        .map(|f| (f.line, f.col, f.code.as_str()))
        .collect();
    assert_eq!(found, [(2, 8, "SC2086")]);
}

#[test]
fn a_raw_check_and_unparseable_json_keep_the_tail_and_no_findings() {
    let fx = Fixture::all_pass();
    fx.tool("alpha", "echo '[]'\nexit 1");
    fx.tool("beta", "echo 'In a.sh line 1:'\nexit 1");
    fx.file("a.sh", "echo hi\n");
    let report = fx.report(&Config::default(), &["a.sh"]);
    for n in 0..2 {
        let outcome = nth(&report, n);
        assert!(outcome.findings.is_empty(), "{outcome:?}");
        assert!(outcome.raw_tail.is_some(), "{outcome:?}");
    }
}

#[test]
fn parsed_json_with_no_finding_on_a_fail_keeps_the_tail() {
    // Nothing is dropped (`src/lint:V92`): a fail with no finding to show
    // must still say why.
    let fx = Fixture::all_pass();
    fx.tool("beta", "echo '[]'\necho broke >&2\nexit 1");
    fx.file("a.sh", "echo hi\n");
    let report = fx.report(&Config::default(), &["a.sh"]);
    assert_eq!(nth(&report, 1).raw_tail.as_deref(), Some("[]\nbroke"));
}

// ---------------------------------------------------------------------
// T125: the timeout
// ---------------------------------------------------------------------

#[test]
fn the_timeout_is_seconds_and_zero_is_no_limit() {
    assert_eq!(limit(0), None);
    assert_eq!(limit(90), Some(Duration::from_secs(90)));
}

#[test]
fn a_check_past_the_timeout_is_an_error_and_the_rest_still_run() {
    // `src/lint:V126`, the task's fixture: a sleeping tool errors at the
    // limit, exit 2, naming the tool and the limit.
    let fx = Fixture::all_pass();
    fx.tool("alpha", &format!("exec {} 30", on_path("sleep")));
    fx.file("a.sh", "echo hi\n");
    let config = parsed("version = 1\n[lint]\ntimeout = 1\n");
    let started = Instant::now();
    let report = fx.report(&config, &["a.sh"]);
    assert!(
        started.elapsed() < Duration::from_secs(20),
        "{:?}",
        started.elapsed()
    );
    // beta still ran. Its own status is not pinned: it shares the 1s
    // limit, and on macOS the first exec of a freshly written script
    // waits on a policy check that, with the whole suite spawning at
    // once, was measured past a second.
    let names: Vec<&str> = report.outcomes().iter().map(|o| o.check.as_str()).collect();
    assert_eq!(names, ["alpha", "beta"]);
    assert_eq!(nth(&report, 0).status, Status::Error);
    let why = nth(&report, 0).raw_tail.clone().unwrap_or_default();
    assert!(why.contains("`alpha`") && why.contains("1s"), "{why}");
    assert_eq!(report.exit_code(), 2);
}

// ---------------------------------------------------------------------
// refusals and what discovery passes on
// ---------------------------------------------------------------------

#[test]
fn a_refusal_from_discovery_is_passed_on_in_its_own_words() {
    let fx = Fixture::all_pass();
    let refused = fx.lint(&Config::default(), &named(&["nope.sh"]));
    let Err(LintError::Discover(inner)) = &refused else {
        panic!("{refused:?}");
    };
    let said = refused.as_ref().err().map(ToString::to_string);
    assert_eq!(said, Some(inner.to_string()));
    assert!(inner.to_string().contains("nope.sh"), "{inner}");
    assert!(refused.is_err_and(|e| e.exit_code() == 2));
}

#[test]
fn a_nested_config_that_does_not_parse_is_refused_in_its_own_words() {
    // `src/config` §I discovery: the file's own layer, not the root's.
    let fx = Fixture::all_pass();
    fx.file("sub/xenolith.toml", "version = 1\n[lint]\nbogus = 1\n");
    fx.file("sub/a.sh", "echo hi\n");
    let refused = fx.lint(&Config::default(), &named(&["sub/a.sh"]));
    let Err(LintError::Config(inner)) = &refused else {
        panic!("{refused:?}");
    };
    let said = refused.as_ref().err().map(ToString::to_string);
    assert_eq!(said, Some(inner.to_string()));
    assert!(inner.to_string().contains("bogus"), "{inner}");
    assert!(refused.is_err_and(|e| e.exit_code() == 2));
}

#[test]
fn a_path_outside_the_root_says_so() {
    let refused = LintError::Outside {
        path: PathBuf::from("../elsewhere/a.sh"),
    };
    assert_eq!(
        refused.to_string(),
        "../elsewhere/a.sh: outside the root: xnl lints the tree it runs in"
    );
    assert_eq!(refused.exit_code(), 2);
}

#[test]
fn a_skipped_symlink_is_warned_about_and_its_target_linted_once() {
    // `src/discover:V128`: the link is not scanned, and says so.
    let fx = Fixture::all_pass();
    fx.file("a.sh", "echo hi\n");
    std::os::unix::fs::symlink("a.sh", fx.root.join("link.sh"))
        .unwrap_or_else(|e| panic!("symlink: {e}"));
    fx.sandbox.run_git(&fx.root, &["init", "-q"]);
    fx.sandbox.run_git(&fx.root, &["add", "a.sh", "link.sh"]);
    let report = fx
        .lint(&Config::default(), &Options::default())
        .unwrap_or_else(|e| panic!("{e}"));
    let files: Vec<String> = report
        .outcomes()
        .iter()
        .map(|o| o.file.display().to_string())
        .collect();
    assert_eq!(files, ["a.sh", "a.sh"]);
    let warned: Vec<(&str, Option<&Path>)> = report
        .warnings()
        .iter()
        .map(|w| (w.code.as_str(), w.file.as_deref()))
        .collect();
    assert_eq!(warned, [("symlink-skipped", Some(Path::new("link.sh")))]);
}

#[test]
fn a_file_with_no_shebang_and_no_extension_is_unclaimed() {
    let fx = Fixture::all_pass();
    fx.file("NOTES", "hi\n");
    let warn = parsed("version = 1\n[langs]\nunclaimed = \"warn\"\n");
    let report = fx.report(&warn, &["NOTES"]);
    assert!(report.outcomes().is_empty(), "{:?}", report.outcomes());
    let warned: Vec<(&str, Option<&Path>)> = report
        .warnings()
        .iter()
        .map(|w| (w.code.as_str(), w.file.as_deref()))
        .collect();
    assert_eq!(warned, [("host-unsupported", Some(Path::new("NOTES")))]);
}
