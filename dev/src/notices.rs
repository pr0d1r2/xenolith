//! `docs/THIRD-PARTY-NOTICES.md`, rendered from its owners (`dev:V347`).
//! RED: signatures only.

#[cfg(test)]
mod tests;

/// One vendored grammar directory, read by the caller.
#[derive(Debug, Clone, Default)]
pub struct Vendored {
    /// Repo-relative directory.
    pub dir: String,
    /// The `UPSTREAM` record.
    pub upstream: String,
    /// Licence and notice file names in the directory.
    pub files: Vec<String>,
    /// `NOTICE.txt`, verbatim.
    pub notice: Option<String>,
}

/// One third-party crate as the notices list it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Crate {
    /// Crate name.
    pub name: String,
    /// Exact version.
    pub version: String,
    /// The SPDX expression its manifest declares.
    pub license: String,
    /// A proc-macro runs inside the compiler.
    pub proc_macro: bool,
}

/// One field of an `UPSTREAM` record.
#[must_use]
pub fn upstream_field(_text: &str, _key: &str) -> Option<String> {
    None
}

/// The runtime and build-only third-party crates.
///
/// # Errors
/// Not yet.
pub fn closure(_metadata: &str) -> Result<(Vec<Crate>, Vec<Crate>), String> {
    Ok((Vec::new(), Vec::new()))
}

/// One tool the nix package wraps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tool {
    /// The command `xnl` runs.
    pub command: String,
    /// The nixpkgs package.
    pub package: String,
    /// Its version.
    pub version: String,
    /// Its SPDX licence ids.
    pub licenses: Vec<String>,
}

/// The tool table.
///
/// # Errors
/// Not yet.
pub fn tools(_json: &str) -> Result<Vec<Tool>, String> {
    Ok(Vec::new())
}

/// The whole file.
///
/// # Errors
/// Not yet.
pub fn render(
    _metadata: &str,
    _vendored: &[Vendored],
    _tools_json: &str,
) -> Result<String, String> {
    Ok(String::new())
}
