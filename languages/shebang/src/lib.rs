//! Shebang lines, for any guest language.
//!
//! A Rust port of [`nix-shebang`](https://github.com/pr0d1r2/nix-shebang),
//! generalised past shell: the nix version answers questions about bash
//! and sh scripts, and xenolith asks the same questions about awk, jq and
//! python extracts too.
//!
//! PORT, not reimplementation. `tests/vectors.json` is nix-shebang's own
//! vector output, vendored, and every text function here is asserted
//! against it (`languages/shebang` §G). Where the two disagree, one of
//! them is wrong and the vector says so.
//!
//! # Why this is its own crate
//!
//! `xenolith-lang-api` may depend on exactly one crate, and this is it
//! (`languages/api:V32`). The extract file a guest produces is
//! [`wrap`]`(body, prelude)` and inlining reads it back through
//! [`strip_strict`]; those two are inverses
//! (`languages/api/src/lens:V63`), and a lens law that holds in memory but
//! not on disk is not the law anybody needed.

#![forbid(unsafe_code)]

#[cfg(test)]
mod tests;

/// A parsed shebang line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shebang {
    /// The interpreter exactly as written: `/usr/bin/env`, `/bin/sh`.
    pub interpreter: String,
    /// Whitespace-separated arguments after the interpreter.
    pub args: Vec<String>,
    /// Whether the interpreter is `/usr/bin/env`, in which case the real
    /// interpreter is the first argument.
    pub is_env: bool,
}

impl Shebang {
    /// `#!/usr/bin/env <interpreter>`, the portable form.
    #[must_use]
    pub fn env(interpreter: &str) -> Shebang {
        Shebang {
            interpreter: "/usr/bin/env".to_owned(),
            args: vec![interpreter.to_owned()],
            is_env: true,
        }
    }

    /// `#!<path>`, an absolute interpreter with no arguments.
    #[must_use]
    pub fn absolute(path: &str) -> Shebang {
        Shebang {
            interpreter: path.to_owned(),
            args: Vec::new(),
            is_env: path == ENV,
        }
    }

    /// `#!<path> <args...>`, for interpreters that need one:
    /// `#!/usr/bin/awk -f`.
    #[must_use]
    pub fn with_args(path: &str, args: &[&str]) -> Shebang {
        Shebang {
            interpreter: path.to_owned(),
            args: args.iter().map(|a| (*a).to_owned()).collect(),
            is_env: path == ENV,
        }
    }

    /// The interpreter that actually runs: the first argument for `env`,
    /// otherwise the interpreter itself.
    ///
    /// `env -S` is understood, pinned by the `envSplitString` vector:
    /// `#!/usr/bin/env -S jq -f` resolves to `jq`, not to `-S`. The flag
    /// exists because a kernel hands the whole tail to the interpreter as
    /// ONE argument, so any shebang wanting two of them goes through it --
    /// which makes it common in exactly the multi-argument cases a guest
    /// like jq or awk needs.
    #[must_use]
    pub fn resolved_interpreter(&self) -> &str {
        if !self.is_env {
            return self.interpreter.as_str();
        }
        let mut args = self.args.iter();
        match args.next() {
            // `env -S jq -f`: the interpreter is one further along.
            Some(first) if first == "-S" => args.next().map_or(first.as_str(), String::as_str),
            Some(first) => first.as_str(),
            None => self.interpreter.as_str(),
        }
    }

    /// The line as it appears in a file, without the trailing newline.
    #[must_use]
    pub fn line(&self) -> String {
        let mut out = String::from("#!");
        out.push_str(&self.interpreter);
        for arg in &self.args {
            out.push(' ');
            out.push_str(arg);
        }
        out
    }
}

/// What goes above an extract's body: the shebang and the strict-mode
/// line, if the guest has them.
///
/// One value per guest (`languages/api` §I), so `bash` carries
/// `set -euo pipefail` and `sh` does not -- `pipefail` is not POSIX -- and
/// sql carries neither, because a `.sql` file is not executed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Prelude {
    /// The shebang line, or `None` for a guest whose files are read
    /// rather than run.
    pub shebang: Option<Shebang>,
    /// The strict-mode line, written exactly as it should appear.
    ///
    /// Owned rather than `&'static str`: the line has to reproduce the
    /// options in force at the site (`languages/shell:V82`), and an
    /// arbitrary `set -o` state -- `set -eu`, `set -eo pipefail`,
    /// `setopt err_exit`-- cannot be one of a fixed set of literals. A
    /// prelude that could only say `set -euo pipefail` would either add
    /// options the body never ran under or drop ones it did, and either
    /// way extraction would change what the code does.
    pub strict: Option<String>,
}

const ENV: &str = "/usr/bin/env";

/// Whether `text` begins with a shebang.
#[must_use]
pub fn has(text: &str) -> bool {
    text.starts_with("#!")
}

/// The shebang line without its newline, or `None`.
#[must_use]
pub fn get(text: &str) -> Option<&str> {
    if has(text) {
        Some(first_line(text))
    } else {
        None
    }
}

/// Parse the shebang at the top of `text`.
///
/// Returns `None` when there is none. Repeated whitespace collapses:
/// `#!/usr/bin/env   bash` has one argument, not three, which matters
/// because an empty argument would become the resolved interpreter.
#[must_use]
pub fn parse(text: &str) -> Option<Shebang> {
    let line = get(text)?;
    let mut parts = line.trim_start_matches("#!").split_whitespace();
    let interpreter = parts.next()?.to_owned();
    let args: Vec<String> = parts.map(str::to_owned).collect();
    let is_env = interpreter == ENV;
    Some(Shebang {
        interpreter,
        args,
        is_env,
    })
}

/// `text` without its shebang line.
///
/// Text with no shebang comes back unchanged -- not an error, because
/// "this body was already bare" is the normal case for a guest that has
/// no shebang at all.
#[must_use]
pub fn strip(text: &str) -> &str {
    if has(text) {
        rest_after_first_line(text)
    } else {
        text
    }
}

/// `text` without its shebang and without the prelude's strict line.
///
/// The inverse of [`wrap`] (`languages/api/src/lens:V63`), and inverse
/// is meant literally: it removes what THIS prelude would have added and
/// nothing more.
///
/// So a prelude with no shebang strips no shebang. Measured, by the
/// round-trip test: a guest body that itself begins with `#!` -- a just
/// recipe holding a script, a heredoc containing one -- came back one
/// line short when the shebang was stripped unconditionally, silently
/// rewriting the file the inline was supposed to restore.
///
/// When the prelude DOES declare a shebang, whatever shebang the file
/// carries is removed, not only a byte-identical one: the extract may
/// have been edited since, and the line is the prelude's to own.
///
/// The strict line is removed only when it sits immediately below, and
/// matches exactly: a `set -euo pipefail` further down is the script's
/// own code, and eating it would change what the file does.
#[must_use]
pub fn strip_strict<'a>(text: &'a str, prelude: &Prelude) -> &'a str {
    let body = if prelude.shebang.is_some() {
        strip(text)
    } else {
        text
    };
    match prelude.strict.as_deref() {
        Some(strict) if first_line(body) == strict => rest_after_first_line(body),
        _ => body,
    }
}

/// `text` without its shebang and without ANY leading `set -…` line.
///
/// Wider than [`strip_strict`], which matches one exact line. This is for
/// reading a file whose strict line was written by hand -- `set -eu`,
/// `set -e` -- where the question is "is there a preamble" rather than
/// "is this our preamble".
#[must_use]
pub fn strip_preamble(text: &str) -> &str {
    let body = strip(text);
    let first = first_line(body);
    if first.starts_with("set -") {
        rest_after_first_line(body)
    } else {
        body
    }
}

/// An extract file's content: the prelude, then the body verbatim.
///
/// The body is never reformatted. It came out of a host file and it has
/// to go back in unchanged (`src/extract:V4`), so anything this function
/// adds it must also be able to take away.
#[must_use]
pub fn wrap(body: &str, prelude: &Prelude) -> String {
    let mut out = String::new();
    if let Some(shebang) = &prelude.shebang {
        out.push_str(&shebang.line());
        out.push('\n');
    }
    if let Some(strict) = prelude.strict.as_deref() {
        out.push_str(strict);
        out.push('\n');
    }
    out.push_str(body);
    out
}

/// The first line of `text`, without its newline.
fn first_line(text: &str) -> &str {
    match text.find('\n') {
        Some(end) => text.get(..end).unwrap_or(text),
        None => text,
    }
}

/// Everything after the first line, with the newline consumed.
///
/// A final line with no newline leaves the empty string, which is what
/// makes `strip("#!/bin/bash")` an empty body rather than an unchanged
/// one.
fn rest_after_first_line(text: &str) -> &str {
    match text.find('\n') {
        Some(end) => text.get(end + 1..).unwrap_or(""),
        None => "",
    }
}
