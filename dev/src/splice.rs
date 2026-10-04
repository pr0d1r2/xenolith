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
///
/// # Errors
///
/// Returns an error when the document does not contain exactly one BEGIN and
/// END marker in the expected order.
pub fn current(doc: &str, name: &str) -> Result<String, String> {
    let (begin_marker, end_marker) = markers(name);
    let begins: Vec<_> = doc
        .lines()
        .enumerate()
        .filter(|(_, l)| *l == begin_marker)
        .collect();
    let ends: Vec<_> = doc
        .lines()
        .enumerate()
        .filter(|(_, l)| *l == end_marker)
        .collect();
    let (Some(begin_position), Some(end_position)) = (begins.first(), ends.first()) else {
        return Err(format!(
            "no markers for {name}: marker error, expected one BEGIN before one END"
        ));
    };
    if begins.len() != 1 || ends.len() != 1 || end_position.0 < begin_position.0 {
        return Err(format!(
            "no markers for {name}: marker error, expected one BEGIN before one END"
        ));
    }
    let after = doc
        .split_once(&begin_marker)
        .ok_or_else(|| format!("marker error for `{name}`"))?
        .1;
    let (block, _) = after
        .split_once(&end_marker)
        .ok_or_else(|| format!("marker error for `{name}`"))?;
    Ok(block.trim_start_matches('\n').to_string())
}

/// The document with one block replaced; `None` when a marker is missing.
///
/// # Errors
///
/// Returns an error when the document does not contain valid markers for the
/// named block.
pub fn splice(doc: &str, name: &str, block: &str) -> Result<String, String> {
    let (begin, end) = markers(name);
    let _ = current(doc, name)?;
    let (head, rest) = doc
        .split_once(&begin)
        .ok_or_else(|| format!("marker error for `{name}`"))?;
    let (_, tail) = rest
        .split_once(&end)
        .ok_or_else(|| format!("marker error for `{name}`"))?;
    Ok(format!("{head}{begin}\n{block}{end}{tail}"))
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
    /// Markers are missing, duplicated, or out of order.
    Invalid(String),
}

/// Up to three lines of `want` that `have` lacks, prefixed.
pub(crate) fn sample(want: &str, have: &str, prefix: &str) -> Vec<String> {
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
        let (begin, end) = markers(name);
        if !doc.lines().any(|line| line == begin) && !doc.lines().any(|line| line == end) {
            missing.push(name.clone());
            continue;
        }
        let have = match current(&next, name) {
            Ok(have) => have,
            Err(error) => return Outcome::Invalid(error),
        };
        if &have == want {
            continue;
        }
        if check_only {
            diff.push(format!("stale: {name}"));
            diff.extend(sample(want, &have, "want"));
            diff.extend(sample(&have, want, "have"));
        } else if let Ok(s) = splice(&next, name, want) {
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
