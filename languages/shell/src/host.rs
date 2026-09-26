//! Shell as a host: the files it claims (`languages/shell:T135`).
//!
//! A shell file encloses other languages the way a nix string encloses
//! bash -- a heredoc fed to `python`, a `-c` string handed to `bash`
//! (`languages/shell` §I). Before any of that, the host has to say which
//! files are its to parse, and the answer has one exception that matters
//! more than the rule: a `*.bats` file is never shell's
//! (`languages/shell:V137`).

use std::path::Path;

use xenolith_lang_api::{
    Delim, Error, FileArg, Format, Host, Invoke, LangId, LintCmd, LoadRef, Result, Site, shebang,
};

#[cfg(test)]
mod tests;

/// Shell as a host.
///
/// A unit struct, as [`crate::ShellGuest`] is: everything it answers
/// comes from the text it is handed (`languages/api:V36`), so the
/// registry can hold one `&'static dyn Host` for it (`src:V41`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShellHost;

/// Extensions that make a file shell whatever its first line says.
const EXTENSIONS: &[&str] = &["sh", "bash"];

/// Filenames that are shell with no extension at all.
const FILENAMES: &[&str] = &[".envrc"];

/// The extension of bats, the language based on shell that shell must
/// never claim (`languages:V130`).
const BATS: &str = "bats";

impl Host for ShellHost {
    fn id(&self) -> LangId {
        LangId::Shell
    }

    /// `*.sh`, `*.bash`, `.envrc`, or a shebang resolving to a shell
    /// dialect (`languages/shell` §I) -- and never `*.bats`, which is
    /// checked FIRST so no shebang can talk the host into it.
    ///
    /// The bash grammar reads a `@test` block as a command and a brace
    /// group, so a claimed bats file would be offered for extraction as a
    /// script: confident and wrong (`languages/shell:V137`). Refusing by
    /// extension is the only place that mistake can be stopped, because
    /// nothing downstream would ever see an error.
    fn claims(&self, path: &Path, head: &str) -> bool {
        if path.extension().is_some_and(|ext| ext == BATS) {
            return false;
        }
        let by_extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| EXTENSIONS.contains(&ext));
        let by_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| FILENAMES.contains(&name));
        by_extension
            || by_name
            || shebang::parse(head).is_some_and(|line| shebang::resolves_to(&line, LangId::Shell))
    }

    /// Not offered yet (`languages/shell:T15`). Refused loudly rather
    /// than answered with an empty list, which would read as "no sites
    /// here" (`languages/api:V37`).
    fn sites(&self, _src: &str) -> Result<Vec<Site>> {
        Err(Error::unsupported(LangId::Shell, "sites"))
    }

    /// Not offered: the load idiom (`python scripts/x.py`, `jq -f x.jq`)
    /// arrives with extraction, and an empty list would read as "nothing
    /// loaded here" (`languages/api:V37`).
    fn loads(&self, _src: &str) -> Result<Vec<LoadRef>> {
        Err(Error::unsupported(LangId::Shell, "loads"))
    }

    /// Not offered yet; see [`ShellHost::loads`].
    fn rewrite(&self, _src: &str, _site: &Site, _invoke: &Invoke, _path: &Path) -> Result<String> {
        Err(Error::unsupported(LangId::Shell, "rewrite"))
    }

    /// Not offered yet; see [`ShellHost::loads`].
    fn inline(&self, _src: &str, _load: &LoadRef, _body: &str) -> Result<String> {
        Err(Error::unsupported(LangId::Shell, "inline"))
    }

    /// Not offered yet; see [`ShellHost::sites`].
    fn unescape(&self, _delim: &Delim, _raw: &str) -> Result<String> {
        Err(Error::unsupported(LangId::Shell, "unescape"))
    }

    /// shellcheck and shfmt, the default linters (`languages/shell` §G).
    ///
    /// Neither is told a dialect. A host file names its own -- a
    /// shebang, a `# shellcheck shell=` directive, an extension -- and
    /// both tools read it; forcing `bash` would check every `#!/bin/sh`
    /// script against the wrong shell.
    fn checks(&self) -> Vec<LintCmd> {
        vec![
            LintCmd {
                argv: vec!["shellcheck".to_owned(), "--format=json".to_owned()],
                file_arg: FileArg::Append,
                format: Format::Json("shellcheck"),
            },
            raw(&["shfmt", "--diff"]),
        ]
    }

    /// shfmt in its rewriting mode. shellcheck fixes nothing itself.
    fn fixers(&self) -> Vec<LintCmd> {
        vec![raw(&["shfmt", "--write"])]
    }
}

/// A command whose output has no machine-readable form.
fn raw(argv: &[&str]) -> LintCmd {
    LintCmd {
        argv: argv.iter().map(|a| (*a).to_owned()).collect(),
        file_arg: FileArg::Append,
        format: Format::Raw,
    }
}
