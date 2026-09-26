//! `[[allow]]` against the sites a scan saw: which entry covers a site,
//! and which entries cover none (`src/config:V9`, `src/config:T25`).
//!
//! Parsing an allow is `super::parse`'s job; this module only MATCHES.
//! The engine (`src:V152`) calls [`Config::allowed`] for each site it
//! would flag, and [`Config::stale_allows`] once with every site it saw,
//! turning each returned entry into a `stale-allow` violation.
//!
//! Matching is exact equality on all three parts of the key
//! (`src/config:V10`). No prefix, no pattern, no path normalisation: a
//! loose match is a blanket allow by another name (V9), and a site the
//! engine names differently from the file is a stale entry the user is
//! told about rather than one silently kept alive.

use std::collections::BTreeSet;

use super::{Allow, Config};

#[cfg(test)]
mod tests;

/// A site as an allow keys it (`src/config:V10`): host path, sink path
/// and the body's content hash. Never a position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SiteKey<'a> {
    /// Host file, repo-root relative, as the engine names it.
    pub path: &'a str,
    /// Sink path inside the host, e.g. `systemd.services.foo.script`.
    pub sink: &'a str,
    /// Content hash of the body.
    pub hash: &'a str,
}

impl Allow {
    /// Whether this entry covers `site`: every key part equal.
    #[must_use]
    pub fn matches(&self, site: &SiteKey<'_>) -> bool {
        self.path == site.path && self.sink == site.sink && self.hash == site.hash
    }

    fn key(&self) -> SiteKey<'_> {
        SiteKey {
            path: &self.path,
            sink: &self.sink,
            hash: &self.hash,
        }
    }
}

impl Config {
    /// The entry covering `site`, the first in file order when several
    /// do, or `None` when the site stands unallowed.
    #[must_use]
    pub fn allowed(&self, site: &SiteKey<'_>) -> Option<&Allow> {
        self.allow.iter().find(|allow| allow.matches(site))
    }

    /// Every entry that no site in `seen` matches, with its index in
    /// `[[allow]]`, in file order (`src/config:V9`).
    ///
    /// `seen` may repeat sites and arrive in any order (the scan is
    /// parallel, `src:V95`); the answer depends on the set only, so it is
    /// the same on every run (`src:V11`).
    pub fn stale_allows<'s, I>(&self, seen: I) -> Vec<(usize, &Allow)>
    where
        I: IntoIterator<Item = SiteKey<'s>>,
    {
        let seen: BTreeSet<SiteKey<'s>> = seen.into_iter().collect();
        self.allow
            .iter()
            .enumerate()
            .filter(|(_, allow)| !seen.contains(&allow.key()))
            .collect()
    }
}
