//! Named generated blocks inside a hand-written document (`dev:V343`).
//! RED: signatures only.

#[cfg(test)]
mod tests;

/// The markers around one block.
#[must_use]
pub fn markers(_name: &str) -> (String, String) {
    (String::new(), String::new())
}

/// What the document carries between one block's markers.
#[must_use]
pub fn current(_doc: &str, _name: &str) -> Option<String> {
    None
}

/// The document with one block replaced.
#[must_use]
pub fn splice(_doc: &str, _name: &str, _block: &str) -> Option<String> {
    None
}

/// Each generated block, by marker name.
pub type Blocks = Vec<(String, String)>;

/// What comparing a document with what it should carry concluded.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Every block already matches.
    Fresh,
    /// The document as a write would leave it.
    Wrote(String),
    /// `--check` found stale blocks.
    Stale(Vec<String>),
    /// A block's markers are absent.
    NoMarkers(Vec<String>),
}

/// Compare every block in one pass.
#[must_use]
pub fn apply(_doc: &str, _blocks: &Blocks, _check_only: bool) -> Outcome {
    Outcome::Fresh
}
