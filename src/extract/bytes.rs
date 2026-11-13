//! Byte-level comparison used by the temporary V4 proof seam.

#[cfg(test)]
mod tests;

/// Words in a byte sequence under the temporary V4 normalization.
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
