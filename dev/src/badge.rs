//! The README badge block, rendered from the files that own each number
//! (`dev:V340`). RED: signatures only.

#[cfg(test)]
mod tests;

/// The value of the first `key = "value"` line, unquoted.
#[must_use]
pub fn manifest_value(_manifest: &str, _key: &str) -> Option<String> {
    None
}

/// The workspace members.
#[must_use]
pub fn members(_manifest: &str) -> Vec<String> {
    Vec::new()
}

/// The languages the default build compiles in.
#[must_use]
pub fn default_languages(_manifest: &str) -> Vec<String> {
    Vec::new()
}

/// Every `[dependencies]` key, and whether it is a `path` dependency.
#[must_use]
pub fn dependencies(_manifest: &str) -> Vec<(String, bool)> {
    Vec::new()
}

/// Every feature the manifest declares.
#[must_use]
pub fn features(_manifest: &str) -> Vec<String> {
    Vec::new()
}

/// A manifest that says `publish = false` ships nowhere.
#[must_use]
pub fn published(_manifest: &str) -> bool {
    true
}

/// The `unsafe_code` level a manifest sets for itself.
#[must_use]
pub fn unsafe_level(_manifest: &str) -> Option<String> {
    None
}

/// A ratchet file's single number.
#[must_use]
pub fn ratchet(_text: &str, _key: &str) -> Option<String> {
    None
}

/// A percentage truncated to one decimal.
#[must_use]
pub fn truncate_tenth(_value: &str) -> Option<String> {
    None
}

/// What `flake.lock` records for one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Locked {
    /// The pinned rev, short.
    pub rev: String,
    /// The day that rev was last modified, UTC.
    pub date: String,
    /// The release branch it follows, if it follows one.
    pub release: Option<String>,
}

/// Read one named node out of `flake.lock`.
#[must_use]
pub fn locked(_lock: &str, _node: &str) -> Option<Locked> {
    None
}

/// A unix timestamp as `YYYY-MM-DD`, UTC.
#[must_use]
pub fn civil_date(_epoch_secs: i64) -> String {
    String::new()
}

/// How many steps the gate declares.
#[must_use]
pub fn gate_steps(_pkl: &str) -> usize {
    0
}

/// The platforms CI gates.
#[must_use]
pub fn ci_platforms(_workflow: &str) -> Vec<(String, String)> {
    Vec::new()
}

/// Every file the block is rendered from.
#[derive(Debug, Clone, Default)]
pub struct Sources {
    /// The root `Cargo.toml`.
    pub manifest: String,
    /// Each workspace member's path and `Cargo.toml`.
    pub members: Vec<(String, String)>,
    /// `.coverage`.
    pub coverage: String,
    /// `.lint-debt`.
    pub debt: String,
    /// `hk.pkl`.
    pub pkl: String,
    /// `flake.lock`.
    pub lock: String,
    /// `.github/workflows/ci.yml`.
    pub workflow: String,
}

/// Everything the block reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Facts {
    /// `license`.
    pub license: String,
    /// `edition`.
    pub edition: String,
    /// `rust-version`.
    pub msrv: String,
    /// Distinct third-party dependencies.
    pub deps: usize,
    /// Shipped crates that deny rather than forbid unsafe code.
    pub unsafe_deny: usize,
    /// Languages the default build compiles in.
    pub languages: usize,
    /// Languages known and not built.
    pub planned: usize,
    /// Gate steps.
    pub gate_steps: usize,
    /// The coverage floor.
    pub coverage_floor: String,
    /// Lint debt.
    pub lint_debt: String,
    /// Spec nodes.
    pub nodes: usize,
    /// The nixpkgs pin.
    pub nixpkgs: Locked,
    /// CI platforms.
    pub platforms: Vec<(String, String)>,
}

/// Gather every fact.
///
/// # Errors
/// Not yet.
pub fn facts(_s: &Sources, _nodes: usize, _known_languages: usize) -> Result<Facts, String> {
    Err(String::new())
}

/// One field of a shields.io static badge.
#[must_use]
pub fn shield_text(raw: &str) -> String {
    raw.to_string()
}

/// Render the block.
#[must_use]
pub fn render(_f: &Facts) -> String {
    String::new()
}
