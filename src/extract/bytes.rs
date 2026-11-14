//! Private byte-fidelity seam for the temporary V4 proof.
//!
//! The extract engine still parses hosts as UTF-8, so this module is not a
//! public byte-preserving API yet. Keeping the comparison here gives the
//! future fidelity crate one small, explicit boundary: the proof consumes
//! bytes, and its transitional rule is normalized words rather than a
//! claim that the rewrite preserved every byte.

#[cfg(test)]
mod tests;

/// Words in a byte sequence under the temporary V4 normalization.
///
/// `from_utf8_lossy` is intentional until the dedicated fidelity crate owns
/// decoding. It makes malformed input deterministic while preserving the
/// current V4 contract (`src/extract:V4`).
pub(super) fn words(bytes: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(bytes)
        .trim_start_matches('\u{feff}')
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

/// Whether two byte sequences are equal under the temporary V4 proof rule.
pub(super) fn equivalent(left: &[u8], right: &[u8]) -> bool {
    words(left) == words(right)
}
