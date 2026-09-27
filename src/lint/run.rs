//! Running one lint command: spawn, collect, judge (`src/lint:V8`).
//!
//! The one place `xnl lint` starts a process. A tool that is not on
//! PATH is an `error` naming it and how to get it -- never a silent skip,
//! because a gate that passes when its linter is missing has checked
//! nothing (`src/lint:V8`). stdin is closed, so a tool waiting on it
//! cannot hang the gate.

use std::ffi::OsString;
use std::io::{ErrorKind, Read};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use super::report::Status;

#[cfg(test)]
mod tests;

/// How many trailing lines of a tool's output a result keeps
/// (`src/lint:V92`).
pub const TAIL_LINES: usize = 40;

/// Where commands are looked up: the inherited `PATH`, or -- the seam
/// the tests use (`tests:V150`) -- a `PATH` of the caller's choosing, so
/// a test sees only the stub tools it wrote.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tools {
    path: Option<OsString>,
}

impl Tools {
    /// The process's own `PATH`.
    #[must_use]
    pub fn inherit() -> Tools {
        Tools::default()
    }

    /// Exactly `path` as the tools' `PATH`.
    #[must_use]
    pub fn on_path(path: impl Into<OsString>) -> Tools {
        Tools {
            path: Some(path.into()),
        }
    }
}

/// How one command ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ran {
    /// Pass, fail or error.
    pub status: Status,
    /// The exit code, when there was one.
    pub exit: Option<i32>,
    /// The output's last [`TAIL_LINES`] lines, or why the tool did not
    /// run; `None` on a pass.
    pub tail: Option<String>,
    /// Standard output alone, for the findings parsers: stderr noise
    /// must not break a tool's JSON (`src/lint` §I findings).
    pub stdout: String,
}

impl Ran {
    fn error(why: String) -> Ran {
        Ran {
            status: Status::Error,
            exit: None,
            tail: Some(why),
            stdout: String::new(),
        }
    }
}

/// Run `argv` in `root` and judge it: exit 0 is a pass, any other exit a
/// fail, and a tool that never ran, died on a signal or was still
/// running at `limit` an error (`src/lint` §I, `src/lint:V126`).
#[must_use]
pub fn run(root: &Path, argv: &[String], limit: Option<Duration>, tools: &Tools) -> Ran {
    let Some((program, rest)) = argv.split_first() else {
        return Ran::error("an empty command: nothing to run".to_owned());
    };
    let mut cmd = Command::new(program);
    cmd.args(rest)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(path) = &tools.path {
        cmd.env("PATH", path);
    }
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ran::error(absent(program)),
        Err(e) => return Ran::error(format!("`{program}` could not be run: {e}")),
    };
    let (printed, complained) = readers(&mut child);
    let status = match wait(&mut child, limit) {
        Ok(Some(status)) => status,
        // The readers are left to finish on their own: a grandchild the
        // tool started may hold its pipes open past the kill, and joining
        // them would be the hang the limit exists to prevent.
        Ok(None) => return Ran::error(overran(program, limit.unwrap_or_default())),
        Err(e) => return Ran::error(format!("`{program}` could not be waited for: {e}")),
    };
    let stdout = collect(printed);
    let mut text = stdout.clone();
    text.push_str(&collect(complained));
    match status.code() {
        Some(0) => Ran {
            status: Status::Pass,
            exit: Some(0),
            tail: None,
            stdout,
        },
        Some(code) => Ran {
            status: Status::Fail,
            exit: Some(code),
            tail: Some(tail(&text).unwrap_or_else(|| format!("exited {code}, printing nothing"))),
            stdout,
        },
        None => Ran::error(match tail(&text) {
            Some(out) => format!("`{program}` was killed by a signal\n{out}"),
            None => format!("`{program}` was killed by a signal"),
        }),
    }
}

/// How often a limited run looks at its child.
const POLL: Duration = Duration::from_millis(10);

/// Wait for `child`; past `limit`, kill it and answer `None`.
fn wait(child: &mut Child, limit: Option<Duration>) -> std::io::Result<Option<ExitStatus>> {
    let Some(limit) = limit else {
        return child.wait().map(Some);
    };
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }
        if started.elapsed() >= limit {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(None);
        }
        thread::sleep(POLL);
    }
}

/// Why a tool killed at the limit has no verdict (`src/lint:V126`).
#[must_use]
pub fn overran(program: &str, limit: Duration) -> String {
    format!(
        "`{program}` did not finish within {}s and was killed; raise `[lint] timeout` \
         if it needs longer (src/lint:V126)",
        limit.as_secs()
    )
}

/// Why a tool that is not on PATH did not run, and how to get it
/// (`src/lint:V8`).
#[must_use]
pub fn absent(program: &str) -> String {
    format!(
        "`{program}` is not on PATH: install it, or use the xenolith nix package, \
         which ships every confirmed tool (src/lint:V8)"
    )
}

/// The last [`TAIL_LINES`] lines of `text`, or `None` when it is blank.
#[must_use]
pub fn tail(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let kept = lines.get(lines.len().saturating_sub(TAIL_LINES)..)?;
    let joined = kept.join("\n");
    (!joined.trim().is_empty()).then_some(joined)
}

/// A stream being read on its own thread.
type Reader = Option<JoinHandle<Vec<u8>>>;

/// One reader thread per stream, so a tool filling one pipe while the
/// other is unread cannot deadlock.
fn readers(child: &mut Child) -> (Reader, Reader) {
    (
        child.stdout.take().map(drain),
        child.stderr.take().map(drain),
    )
}

fn drain(mut stream: impl Read + Send + 'static) -> JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stream.read_to_end(&mut buf);
        buf
    })
}

fn collect(handle: Reader) -> String {
    handle
        .and_then(|h| h.join().ok())
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default()
}
