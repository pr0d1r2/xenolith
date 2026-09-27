//! The lint engine: `xnl lint` as a library call (`src/lint:V8`).
//!
//! Every extract is run through its guest's checks, every host file
//! through its host's (`[lint] hosts`), each command reported on its own
//! and none stopping the rest. The stages are joined here and owned
//! elsewhere:
//!
//! 1. candidates -- [`crate::discover`] (`src/discover:V57`), minus what
//!    `[[exclude]]` and `[lint] exclude` skip (`src/config:V79`).
//! 2. targets -- until `src/graph` names extracts, a file is an extract
//!    when its shebang names a compiled-in guest, or it has none and its
//!    extension is the guest's; a host file is one a host claims; a file
//!    that is neither follows `src/check:V13` (`src/lint` §I, targets).
//! 3. commands -- [`plan`]: the language crate's defaults and the
//!    config's own, which run only when trusted (`src/lint:V91`).
//! 4. runs -- [`run`]: a tool not on PATH is an `error`, exit 2
//!    (`src/lint:V8`). Under `--fix` an extract's fixers run first and
//!    its checks judge the result; a host file is never rewritten.
//!
//! `src/cli` renders the [`LintReport`] and maps its exit code.

use std::fmt;
use std::fs;
use std::io::Read as _;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use xenolith_lang_api::{Guest, GuestEnv, Host, LangId, LintCmd, shebang};

use crate::check::{Langs, UNCLAIMED, repo_name};
use crate::cli::EXIT_USAGE;
use crate::config::{Config, Policy, Tree, TreeError, Verb};
use crate::discover::{DiscoverError, discover_with};
use crate::model::Warning;
use crate::registry;

pub mod findings;
pub mod plan;
pub mod report;
pub mod run;

pub use findings::Finding;
pub use report::{Kind, LintReport, Outcome, Source, Status};

use self::plan::{Cmd, Configured};
use self::run::Tools;

#[cfg(test)]
mod tests;

/// The warning naming a config command held back for want of
/// `--trust-config` (`src/lint:V91`).
pub const UNTRUSTED_COMMAND: &str = "untrusted-command";

/// What a run is asked to do.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    /// The paths named; empty means every tracked file (`src/discover:V57`).
    pub paths: Vec<PathBuf>,
    /// `--strict-hosts` (`src/check:V13`).
    pub strict_hosts: bool,
    /// `--fix`: each extract's fixers first, then its checks; host
    /// files are never rewritten (`src/lint:V8`).
    pub fix: bool,
    /// `--trust-config`: run the commands a `xenolith.toml` defines;
    /// only the flag grants it, never a config key (`src/lint:V91`).
    pub trust_config: bool,
}

/// Why a run was refused rather than carried out; every variant exit 2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LintError {
    /// Discovery refused (`src/discover:V57`, `src/discover:V128`).
    Discover(DiscoverError),
    /// A nested `xenolith.toml` refused (`src/config` §I discovery).
    Config(TreeError),
    /// A file nothing lints, under `--strict-hosts` or `[langs]
    /// unclaimed = "error"` (`src/check:V13`).
    Unclaimed {
        /// The file, as reports name it.
        file: PathBuf,
    },
    /// A named path outside the root.
    Outside {
        /// The path as discovery kept it.
        path: PathBuf,
    },
}

impl LintError {
    /// The process exit code: always 2.
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        EXIT_USAGE
    }
}

impl fmt::Display for LintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LintError::Discover(e) => e.fmt(f),
            LintError::Config(e) => e.fmt(f),
            LintError::Unclaimed { file } => write!(
                f,
                "{}: host unsupported: no host in this build claims it and no guest reads \
                 it, so it cannot be linted (src/check:V13)",
                file.display()
            ),
            LintError::Outside { path } => write!(
                f,
                "{}: outside the root: xnl lints the tree it runs in",
                path.display()
            ),
        }
    }
}

impl std::error::Error for LintError {}

impl From<DiscoverError> for LintError {
    fn from(e: DiscoverError) -> LintError {
        LintError::Discover(e)
    }
}

impl From<TreeError> for LintError {
    fn from(e: TreeError) -> LintError {
        LintError::Config(e)
    }
}

/// Lint the tree at `root` under `config` (`src/lint:V8`).
///
/// # Errors
///
/// [`LintError`], exit 2: discovery or a nested config refused, a named
/// path outside the root, or an unclaimed file under strict hosts.
pub fn lint(root: &Path, config: &Config, options: &Options) -> Result<LintReport, LintError> {
    let langs = Langs {
        hosts: registry::hosts(),
        guests: registry::guests(),
    };
    lint_with(
        root,
        config,
        options,
        &langs,
        &|| Command::new("git"),
        &Tools::inherit(),
    )
}

/// [`lint`] with the languages, `git` and the tools' `PATH` supplied by
/// the caller -- the seam the tests use (`tests:V150`).
pub(crate) fn lint_with(
    root: &Path,
    config: &Config,
    options: &Options,
    langs: &Langs<'_>,
    git: &dyn Fn() -> Command,
    tools: &Tools,
) -> Result<LintReport, LintError> {
    let candidates = discover_with(root, &options.paths, git)?;
    if let Some(path) = candidates.files.iter().find(|f| outside(f)) {
        return Err(LintError::Outside { path: path.clone() });
    }
    let names: Vec<String> = candidates.files.iter().map(|f| repo_name(f)).collect();
    let tree = Tree::load(
        root,
        config.clone(),
        Verb::Lint,
        names.iter().map(String::as_str),
    )?;
    let mut report = LintReport::new();
    for warning in candidates.warnings {
        report.warn(warning);
    }
    let run = Run {
        root,
        tools,
        fix: options.fix,
        trusted: options.trust_config,
    };
    for (file, name) in candidates.files.iter().zip(&names) {
        let config = tree.config_for(name);
        if config.excluded(Verb::Lint, name).is_some() {
            continue;
        }
        let head = head(&root.join(file));
        let hosts: Vec<&dyn Host> = langs
            .hosts
            .iter()
            .copied()
            .filter(|host| host.claims(file, &head))
            .collect();
        let found = extract_of(langs.guests, file, &head);
        if hosts.is_empty() && matches!(found, Found::Nothing) {
            unclaimed(&mut report, config, options, name)?;
            continue;
        }
        if config.lint.hosts {
            for host in hosts {
                run.host(&mut report, host, name, limit(config.lint.timeout));
            }
        }
        match found {
            Found::Guest(guest, env) => run.extract(&mut report, config, guest, &env, name),
            Found::Missing(id, interpreter) => report.warn(registry::missing_shebang_guest(
                id,
                Path::new(name),
                Some(&interpreter),
            )),
            Found::Nothing => {}
        }
    }
    Ok(report)
}

/// What a file is to its guests.
enum Found<'a> {
    /// An extract of this guest, in this env.
    Guest(&'a dyn Guest, GuestEnv),
    /// Its shebang names a guest this build lacks (`src/check:V42`).
    Missing(LangId, String),
    /// No guest reads it.
    Nothing,
}

/// Stage 2 for guests (`src/lint` §I, targets): a shebang decides, and a
/// shebang naming no language decides "none" rather than letting the
/// extension guess; with no shebang, the extension a guest gives its
/// extracts under the default env.
fn extract_of<'a>(guests: &[&'a dyn Guest], file: &Path, head: &str) -> Found<'a> {
    if let Some(bang) = shebang::parse(head) {
        let Some(id) = shebang::guest_of(&bang) else {
            return Found::Nothing;
        };
        let interpreter = bang.resolved_interpreter();
        let dialect = interpreter.rsplit('/').next().unwrap_or(interpreter);
        return match guests.iter().copied().find(|g| g.id() == id) {
            Some(guest) => Found::Guest(
                guest,
                GuestEnv {
                    dialect: Some(dialect.to_owned()),
                    options: Vec::new(),
                },
            ),
            None => Found::Missing(id, interpreter.to_owned()),
        };
    }
    let Some(ext) = file.extension().and_then(|e| e.to_str()) else {
        return Found::Nothing;
    };
    let env = GuestEnv::default();
    guests
        .iter()
        .copied()
        .find(|g| g.extension(&env) == ext)
        .map_or(Found::Nothing, |guest| Found::Guest(guest, env))
}

/// A file nothing lints (`src/check:V13`): ignored by default, a warning under
/// `warn`, a refusal under `error` or `--strict-hosts`.
fn unclaimed(
    report: &mut LintReport,
    config: &Config,
    options: &Options,
    name: &str,
) -> Result<(), LintError> {
    let policy = if options.strict_hosts {
        Policy::Error
    } else {
        config.langs.unclaimed
    };
    match policy {
        Policy::Ignore => Ok(()),
        Policy::Warn => {
            report.warn(Warning {
                code: UNCLAIMED.to_owned(),
                file: Some(PathBuf::from(name)),
                message: format!(
                    "{name}: host unsupported: no host claims it and no guest reads it, so \
                     it was not linted (src/check:V13)"
                ),
            });
            Ok(())
        }
        Policy::Error => Err(LintError::Unclaimed {
            file: PathBuf::from(name),
        }),
    }
}

/// The run's fixed inputs: where commands run and how they are found.
struct Run<'a> {
    root: &'a Path,
    tools: &'a Tools,
    /// `--fix`: fixers before checks, extracts only.
    fix: bool,
    /// `--trust-config`: config commands run (`src/lint:V91`).
    trusted: bool,
}

impl Run<'_> {
    /// `Host::checks` on a host file; defaults only, config names none
    /// for hosts (`src/lint` §I).
    fn host(&self, report: &mut LintReport, host: &dyn Host, name: &str, limit: Option<Duration>) {
        let target = Target {
            name,
            kind: Kind::Host,
            guest: None,
            dialect: None,
            limit,
        };
        for cmd in host.checks().iter().map(Cmd::builtin) {
            report.push(self.one(&target, &cmd, false));
        }
    }

    /// An extract's checks, defaults and config (`src/lint:V8`); under
    /// `--fix`, its fixers first, so the checks judge the fixed file.
    fn extract(
        &self,
        report: &mut LintReport,
        config: &Config,
        guest: &dyn Guest,
        env: &GuestEnv,
        name: &str,
    ) {
        let id = guest.id();
        let entry = config.lint.guests.get(&id);
        let extend = extend(config, id);
        let target = Target {
            name,
            kind: Kind::Extract,
            guest: Some(id),
            dialect: env.dialect.clone(),
            limit: limit(config.lint.timeout),
        };
        if self.fix {
            let fixers: Vec<LintCmd> = guest.fixers(env);
            let planned = plan::plan(&fixers, Configured::fixers(entry, extend), self.trusted);
            for cmd in &planned.untrusted {
                untrusted(report, &target, cmd, true);
            }
            for cmd in &planned.run {
                // A fixer that did its job is not news; one that did not
                // is (`src/lint` §I, status).
                let outcome = self.one(&target, cmd, true);
                if outcome.status != Status::Pass {
                    report.push(outcome);
                }
            }
        }
        let checks: Vec<LintCmd> = guest.checks(env);
        let planned = plan::plan(
            &checks,
            Configured::checks(entry, extend, &config.lint.all),
            self.trusted,
        );
        for cmd in &planned.untrusted {
            untrusted(report, &target, cmd, false);
        }
        for cmd in &planned.run {
            report.push(self.one(&target, cmd, false));
        }
    }

    /// Run one command on the target and make its outcome.
    fn one(&self, target: &Target<'_>, cmd: &Cmd, fixer: bool) -> Outcome {
        let argv = cmd.argv(target.name);
        let ran = run::run(self.root, &argv, target.limit, self.tools);
        Outcome {
            file: PathBuf::from(target.name),
            kind: target.kind,
            guest: target.guest,
            dialect: target.dialect.clone(),
            check: cmd.check(),
            argv,
            source: cmd.source,
            status: ran.status,
            exit: ran.exit,
            raw_tail: ran.tail,
            fixer,
        }
    }
}

/// The file a command runs on, as outcomes name it.
struct Target<'a> {
    name: &'a str,
    kind: Kind,
    guest: Option<LangId>,
    dialect: Option<String>,
    /// `[lint] timeout` for the file (`src/lint:V126`).
    limit: Option<Duration>,
}

/// A config command held back (`src/lint:V91`): a `skipped` outcome, and
/// one warning per distinct command naming it.
fn untrusted(report: &mut LintReport, target: &Target<'_>, cmd: &Cmd, fixer: bool) {
    let argv = cmd.argv(target.name);
    report.warn(Warning {
        code: UNTRUSTED_COMMAND.to_owned(),
        file: None,
        message: format!(
            "`{}` from xenolith.toml was not run: commands a config defines run only with \
             --trust-config (src/lint:V91)",
            cmd.words.join(" ")
        ),
    });
    report.push(Outcome {
        file: PathBuf::from(target.name),
        kind: target.kind,
        guest: target.guest,
        dialect: target.dialect.clone(),
        check: cmd.check(),
        argv,
        source: cmd.source,
        status: Status::Skipped,
        exit: None,
        raw_tail: None,
        fixer,
    });
}

/// `[lint] timeout` as a wall clock per command, `None` for 0: no limit
/// (`src/lint:V126`, `src/lint` §I).
#[must_use]
pub fn limit(seconds: u64) -> Option<Duration> {
    (seconds > 0).then(|| Duration::from_secs(seconds))
}

/// `[lint.<guest>] extend`, through the defaults table (`src/config:V73`).
fn extend(config: &Config, id: LangId) -> bool {
    matches!(
        config.effective(&format!("lint.{id}.extend")),
        Some(crate::config::Effective::Bool(true))
    )
}

/// A discovered path that is not below the root: absolute, or climbing
/// out of it.
fn outside(path: &Path) -> bool {
    path.is_absolute() || matches!(path.components().next(), Some(Component::ParentDir))
}

/// The first line of the file, read without reading the rest: the
/// shebang and `Host::claims` need no more.
fn head(path: &Path) -> String {
    let mut buf = Vec::new();
    if let Ok(file) = fs::File::open(path) {
        let _ = file.take(1024).read_to_end(&mut buf);
    }
    let first = buf.split(|b| *b == b'\n').next().unwrap_or_default();
    String::from_utf8_lossy(first).into_owned()
}
