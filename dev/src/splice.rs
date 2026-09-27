//! Named generated blocks inside a hand-written document (`dev:V343`).
//!
//! A block lives between `<!-- BEGIN <name> -->` and `<!-- END <name> -->`.
//! Splicing is idempotent, which is what lets `--check` be a plain diff of
//! what a write would produce.

#[cfg(test)]
mod tests;

/// The markers around one block.
#[must_use]
pub fn markers(name: &str) -> (String, String) {
    (
        format!("<!-- BEGIN {name} -->"),
        format!("<!-- END {name} -->"),
    )
}

/// What the document carries between one block's markers, if both exist.
#[must_use]
pub fn current(doc: &str, name: &str) -> Option<String> {
    let (begin, end) = markers(name);
    let after = doc.split_once(&begin)?.1;
    let (block, _) = after.split_once(&end)?;
    Some(block.trim_start_matches('\n').to_string())
}

/// The document with one block replaced; `None` when a marker is missing.
#[must_use]
pub fn splice(doc: &str, name: &str, block: &str) -> Option<String> {
    let (begin, end) = markers(name);
    let (head, rest) = doc.split_once(&begin)?;
    let (_, tail) = rest.split_once(&end)?;
    Some(format!("{head}{begin}\n{block}{end}{tail}"))
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
    /// `--check` found stale blocks; one line per finding.
    Stale(Vec<String>),
    /// A block's markers are absent: the document has not opted in.
    NoMarkers(Vec<String>),
}

/// Up to three lines of `want` that `have` lacks, prefixed.
fn sample(want: &str, have: &str, prefix: &str) -> Vec<String> {
    want.lines()
        .filter(|l| !l.trim().is_empty() && !have.lines().any(|h| h == *l))
        .take(3)
        .map(|l| format!("  {prefix}: {l}"))
        .collect()
}

/// Compare every block in one pass: a document is current or it is not,
/// and one fresh block must not hide a stale one beside it.
#[must_use]
pub fn apply(doc: &str, blocks: &Blocks, check_only: bool) -> Outcome {
    let mut next = doc.to_string();
    let mut diff = Vec::new();
    let mut missing = Vec::new();
    for (name, want) in blocks {
        let Some(have) = current(&next, name) else {
            missing.push(name.clone());
            continue;
        };
        if &have == want {
            continue;
        }
        if check_only {
            diff.push(format!("stale: {name}"));
            diff.extend(sample(want, &have, "want"));
            diff.extend(sample(&have, want, "have"));
        } else if let Some(s) = splice(&next, name, want) {
            next = s;
        }
    }
    if !missing.is_empty() {
        Outcome::NoMarkers(missing)
    } else if !diff.is_empty() {
        Outcome::Stale(diff)
    } else if next == doc {
        Outcome::Fresh
    } else {
        Outcome::Wrote(next)
    }
}
