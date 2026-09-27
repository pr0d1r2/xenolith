//! `xenolith-dev` -- tooling that maintains THIS repository (`dev` §G).
//!
//! It regenerates what the README and `docs/THIRD-PARTY-NOTICES.md` would
//! otherwise have typed by hand -- the badge block, the Languages table and
//! the notices -- from the files that own each fact, and `--check` refuses
//! any of them that has drifted (`dev:V340`, `dev:V343`). It ships to
//! nobody: `publish = false`, and the nix package builds `-p xenolith`
//! (`nix:V349`).
//!
//! The shape is sherd's `dev/` crate (`dev:R341`), with this repository's
//! lib-and-shim rule on top (`src:C139`): `main.rs` calls [`main`], and
//! [`run`] takes the repository root, the two external readings and the
//! error stream as parameters, so every branch here is reachable from a
//! unit test on a scratch directory. Everything that DECIDES lives in the
//! pure modules below and is handed its text.
//!
//! Exit codes are `src/cli:V24`'s: 0 clean, 1 stale or refused, 2 usage.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use xenolith_lang_api::LangId;

pub mod badge;
pub mod langs;
pub mod notices;
pub mod select;
pub mod splice;

#[cfg(test)]
mod tests;

/// What the binary prints for a usage error or `--help`.
pub const USAGE: &str = "\
xenolith-dev -- tooling for the xenolith repository itself; never published

  xenolith-dev --check [<path>...]   compare every generated output with its owners.
                                     paths narrow the work to what they feed;
                                     none compares everything.
  xenolith-dev --fix [<path>...]     regenerate what --check would refuse
  xenolith-dev readme [--check] [<path>...]
                                     the README's generated blocks: badges, langs
  xenolith-dev notices [--check] [<path>...]
                                     docs/THIRD-PARTY-NOTICES.md

exit: 0 clean, 1 stale or refused, 2 usage
";

/// Where the notices' two external readings come from. `None` runs the
/// owner (`cargo metadata`, `nix eval`); a path reads a recorded answer
/// instead, which is how the tests and `XENOLITH_DEV_CARGO_METADATA` /
/// `XENOLITH_DEV_TOOL_LICENSES` hand one in.
#[derive(Debug, Clone, Default)]
pub struct External {
    /// A file holding `cargo metadata --format-version 1` output.
    pub metadata: Option<PathBuf>,
    /// A file holding `nix eval --json .#default.toolLicenses` output.
    pub tools: Option<PathBuf>,
}

/// The binary: arguments and environment in, exit code out.
#[must_use]
pub fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let external = External {
        metadata: std::env::var_os("XENOLITH_DEV_CARGO_METADATA").map(PathBuf::from),
        tools: std::env::var_os("XENOLITH_DEV_TOOL_LICENSES").map(PathBuf::from),
    };
    let root = std::env::current_dir().ok().and_then(|d| repo_root(&d));
    ExitCode::from(run(
        &args,
        root.as_deref(),
        &external,
        &mut std::io::stderr(),
    ))
}

/// The repository root: the nearest ancestor holding `.git`, `SPEC.md` and
/// `Cargo.toml` -- run from anywhere inside, the binary finds the same root.
#[must_use]
pub fn repo_root(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|d| {
            d.join(".git").exists() && d.join("SPEC.md").is_file() && d.join("Cargo.toml").is_file()
        })
        .map(Path::to_path_buf)
}

/// One maintained output: its name and the function that checks or writes it.
type Job = fn(&Path, bool, &[String], &External, &mut dyn Write) -> u8;

const JOBS: [(&str, Job); 2] = [("readme", readme), ("notices", notices)];

/// Dispatch `args` against the repository at `root`.
pub fn run(args: &[String], root: Option<&Path>, external: &External, err: &mut dyn Write) -> u8 {
    let usage = |err: &mut dyn Write, code: u8| {
        let _ = err.write_all(USAGE.as_bytes());
        code
    };
    let Some((verb, rest)) = args.split_first() else {
        return usage(err, 2);
    };
    if matches!(verb.as_str(), "help" | "--help" | "-h") {
        return usage(err, 0);
    }
    let check = rest.iter().any(|a| a == "--check");
    if let Some(flag) = rest.iter().find(|a| a.starts_with("--") && *a != "--check") {
        let _ = writeln!(err, "xenolith-dev: unknown flag {flag}\n");
        return usage(err, 2);
    }
    let paths: Vec<String> = rest
        .iter()
        .filter(|a| !a.starts_with("--"))
        .cloned()
        .collect();
    let job: Option<Job> = match verb.as_str() {
        "--check" | "--fix" => None,
        "readme" => Some(readme),
        "notices" => Some(notices),
        other => {
            let _ = writeln!(err, "xenolith-dev: unknown verb {other}\n");
            return usage(err, 2);
        }
    };
    let Some(root) = root else {
        let _ = writeln!(
            err,
            "xenolith-dev: not inside the repository -- no ancestor holds .git, \
             SPEC.md and Cargo.toml"
        );
        return 2;
    };
    match job {
        Some(job) => job(root, check, &paths, external, err),
        None => all(root, verb == "--check", &paths, external, err),
    }
}

/// Every job, CONCURRENTLY: one entry point for the hooks, so a hook never
/// carries its own list of what this binary maintains. Each job writes into
/// its own buffer and the buffers are replayed in job order, so the output
/// is the same bytes however the threads interleave.
fn all(
    root: &Path,
    check_only: bool,
    paths: &[String],
    external: &External,
    err: &mut dyn Write,
) -> u8 {
    let results: Vec<(&str, u8, Vec<u8>)> = std::thread::scope(|s| {
        let handles: Vec<_> = JOBS
            .iter()
            .map(|(name, job)| {
                let handle = s.spawn(move || {
                    let mut buf = Vec::new();
                    let code = job(root, check_only, paths, external, &mut buf);
                    (code, buf)
                });
                (*name, handle)
            })
            .collect();
        handles
            .into_iter()
            .map(|(name, h)| {
                let (code, buf) = h
                    .join()
                    .unwrap_or_else(|_| (1, b"the job panicked\n".to_vec()));
                (name, code, buf)
            })
            .collect()
    });
    let mut worst = 0;
    for (name, code, buf) in results {
        let _ = err.write_all(&buf);
        if code != 0 {
            let _ = writeln!(err, "xenolith-dev: {name} failed");
        }
        worst = worst.max(code);
    }
    worst
}

fn read(root: &Path, rel: &str) -> Result<String, String> {
    fs::read_to_string(root.join(rel)).map_err(|e| format!("xenolith-dev: {rel}: {e}"))
}

/// Every file under `root` for which `keep` holds, repo-relative with `/`,
/// sorted. Dot-directories, `target` and symlinks are never entered: a
/// build or a nested worktree is not the tree being described.
fn walk(root: &Path, keep: &dyn Fn(&str) -> bool) -> Vec<String> {
    let mut out = Vec::new();
    let mut todo = vec![String::new()];
    while let Some(rel) = todo.pop() {
        let Ok(entries) = fs::read_dir(root.join(&rel)) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = if rel.is_empty() {
                name.clone()
            } else {
                format!("{rel}/{name}")
            };
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() && !name.starts_with('.') && name != "target" {
                todo.push(path);
            } else if kind.is_file() && keep(&path) {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// Every `SPEC.md` in the tree: the federated nodes (`.:C22`).
#[must_use]
pub fn spec_paths(root: &Path) -> Vec<String> {
    walk(root, &|p| p == "SPEC.md" || p.ends_with("/SPEC.md"))
}

/// Every badge input, read from its owner (`dev:V340`).
///
/// # Errors
/// An owning file that cannot be read.
pub fn sources(root: &Path) -> Result<badge::Sources, String> {
    let manifest = read(root, "Cargo.toml")?;
    let members = badge::members(&manifest)
        .into_iter()
        .map(|m| {
            let text = read(root, &format!("{m}/Cargo.toml"))?;
            Ok((m, text))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(badge::Sources {
        members,
        coverage: read(root, ".coverage")?,
        debt: read(root, ".lint-debt")?,
        pkl: read(root, "hk.pkl")?,
        lock: read(root, "flake.lock")?,
        workflow: read(root, ".github/workflows/ci.yml")?,
        manifest,
    })
}

/// The README's generated blocks, each rendered from its owners.
///
/// # Errors
/// An owner that cannot be read, or that lacks the value it owns.
pub fn readme_blocks(root: &Path) -> Result<splice::Blocks, String> {
    let src = sources(root)?;
    let specs = spec_paths(root);
    let facts = badge::facts(&src, specs.len(), LangId::ALL.len())?;
    let ids: Vec<&str> = LangId::ALL.iter().map(|l| l.as_str()).collect();
    let langs = langs::render(
        &ids,
        &badge::features(&src.manifest),
        &badge::default_languages(&src.manifest),
        &specs,
    );
    Ok(vec![
        ("badges".to_string(), badge::render(&facts)),
        ("langs".to_string(), langs),
    ])
}

fn readme(
    root: &Path,
    check_only: bool,
    paths: &[String],
    _: &External,
    err: &mut dyn Write,
) -> u8 {
    let wanted: Vec<&str> = select::selected(paths)
        .into_iter()
        .filter(|n| *n != "notices")
        .collect();
    if wanted.is_empty() {
        return 0;
    }
    let outcome = readme_blocks(root).and_then(|blocks| {
        let text = read(root, "README.md")?;
        let scoped: splice::Blocks = blocks
            .into_iter()
            .filter(|(name, _)| wanted.contains(&name.as_str()))
            .collect();
        Ok(splice::apply(&text, &scoped, check_only))
    });
    match outcome {
        Err(e) => {
            let _ = writeln!(err, "{e}");
            1
        }
        Ok(splice::Outcome::Fresh) => 0,
        Ok(splice::Outcome::NoMarkers(names)) => {
            let _ = writeln!(
                err,
                "xenolith-dev: README.md has no markers for {}. Each generated block sits \
                 between `<!-- BEGIN <name> -->` and `<!-- END <name> -->`; the file is left \
                 as it is.",
                names.join(", ")
            );
            1
        }
        Ok(splice::Outcome::Stale(diff)) => {
            let _ = writeln!(
                err,
                "xenolith-dev: a generated README block is STALE. Run `xenolith-dev readme` (or \
                 `hk fix`) to regenerate it from the files that own each number (dev:V340)."
            );
            for line in diff {
                let _ = writeln!(err, "  {line}");
            }
            1
        }
        Ok(splice::Outcome::Wrote(next)) => write(root, "README.md", &next, err),
    }
}

fn write(root: &Path, rel: &str, text: &str, err: &mut dyn Write) -> u8 {
    let path = root.join(rel);
    let made = path.parent().map_or(Ok(()), fs::create_dir_all);
    match made.and_then(|()| fs::write(&path, text)) {
        Ok(()) => 0,
        Err(e) => {
            let _ = writeln!(err, "xenolith-dev: {rel}: {e}");
            1
        }
    }
}

/// Run an owner and take what it prints. A tool that cannot be started is
/// named as a MISSING TOOL, never reported as a finding.
///
/// # Errors
/// The program cannot be started, exits non-zero, or prints non-UTF-8.
pub fn capture(root: &Path, program: &str, args: &[&str]) -> Result<String, String> {
    let shown = format!("{program} {}", args.join(" "));
    let out = Command::new(program)
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| {
            format!(
                "xenolith-dev: cannot run `{shown}`: {e}. This is a MISSING TOOL, not a finding: \
                 enter the dev shell (`nix develop`)."
            )
        })?;
    if !out.status.success() {
        return Err(format!(
            "xenolith-dev: `{shown}` failed ({}):\n{}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim_end()
        ));
    }
    String::from_utf8(out.stdout)
        .map_err(|e| format!("xenolith-dev: `{shown}` printed non-UTF-8: {e}"))
}

fn reading(
    root: &Path,
    recorded: Option<&Path>,
    program: &str,
    args: &[&str],
) -> Result<String, String> {
    match recorded {
        Some(path) => {
            fs::read_to_string(path).map_err(|e| format!("xenolith-dev: {}: {e}", path.display()))
        }
        None => capture(root, program, args),
    }
}

/// Every vendored grammar: a `vendor/<name>/UPSTREAM` record, with the
/// licence and notice files beside it.
///
/// # Errors
/// A record or notice that cannot be read.
pub fn vendored(root: &Path) -> Result<Vec<notices::Vendored>, String> {
    let records = walk(root, &|p| {
        let mut parts = p.rsplit('/');
        parts.next() == Some("UPSTREAM") && parts.nth(1) == Some("vendor")
    });
    records
        .iter()
        .map(|record| {
            let dir = record.trim_end_matches("/UPSTREAM").to_string();
            let mut files: Vec<String> = walk(&root.join(&dir), &|p| {
                !p.contains('/') && (p.starts_with("LICENSE") || p.starts_with("NOTICE"))
            });
            files.sort();
            let notice = match files.iter().find(|f| f.starts_with("NOTICE")) {
                Some(f) => Some(read(root, &format!("{dir}/{f}"))?),
                None => None,
            };
            Ok(notices::Vendored {
                upstream: read(root, record)?,
                dir,
                files,
                notice,
            })
        })
        .collect()
}

/// The notices as they should read today.
///
/// # Errors
/// An owner that cannot be run or read, or that answers in the wrong shape.
pub fn notices_text(root: &Path, external: &External) -> Result<String, String> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let metadata = reading(
        root,
        external.metadata.as_deref(),
        &cargo,
        &[
            "metadata",
            "--format-version",
            "1",
            "--locked",
            "--offline",
            "--all-features",
        ],
    )?;
    let tools = reading(
        root,
        external.tools.as_deref(),
        "nix",
        &["eval", "--json", ".#default.toolLicenses"],
    )?;
    notices::render(&metadata, &vendored(root)?, &tools)
}

const NOTICES: &str = "docs/THIRD-PARTY-NOTICES.md";

fn notices(
    root: &Path,
    check_only: bool,
    paths: &[String],
    external: &External,
    err: &mut dyn Write,
) -> u8 {
    if !select::selected(paths).contains(&"notices") {
        return 0;
    }
    let want = match notices_text(root, external) {
        Ok(text) => text,
        Err(e) => {
            let _ = writeln!(err, "{e}");
            return 1;
        }
    };
    let have = fs::read_to_string(root.join(NOTICES)).unwrap_or_default();
    if have == want {
        return 0;
    }
    if !check_only {
        return write(root, NOTICES, &want, err);
    }
    let _ = writeln!(
        err,
        "xenolith-dev: {NOTICES} is STALE against cargo metadata, the vendored grammars or \
         the nix tool table. Run `xenolith-dev notices` (or `hk fix`) to regenerate it (dev:V347)."
    );
    let differs = |a: &str, b: &str, tag: &str| -> Vec<String> {
        a.lines()
            .filter(|l| !l.trim().is_empty() && !b.lines().any(|m| m == *l))
            .take(3)
            .map(|l| format!("  {tag}: {l}"))
            .collect()
    };
    for line in differs(&want, &have, "want")
        .into_iter()
        .chain(differs(&have, &want, "have"))
    {
        let _ = writeln!(err, "{line}");
    }
    1
}
