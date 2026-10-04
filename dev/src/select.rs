//! Which generated outputs a change can have invalidated (`dev:V345`).
//!
//! hk hands a step the changed files. Each output declares the inputs it is
//! rendered FROM, and a change selects the outputs whose inputs it touches.
//! A HEURISTIC in one direction only: a pattern too broad costs a re-render,
//! a pattern too narrow lets a stale output through the commit hook -- so
//! the push and CI layer runs with no paths, where every output is compared.

#[cfg(test)]
mod tests;

/// One generated output and the files it is rendered from.
#[derive(Debug)]
pub struct Generated {
    /// The README marker name, or `notices` for the notices file.
    pub name: &'static str,
    /// Literal paths, `dir/**` prefixes and `**/name` suffixes.
    pub inputs: &'static [&'static str],
}

/// Every output `xenolith-dev` maintains. The output's own file is an input
/// too: a hand edit inside a generated block is a change the narrow layer
/// must compare, not wait for the push to find.
pub const OUTPUTS: &[Generated] = &[
    Generated {
        name: "badges",
        inputs: &[
            "README.md",
            "**/Cargo.toml",
            "hk.pkl",
            ".coverage",
            ".lint-debt",
            "flake.lock",
            ".github/workflows/ci.yml",
            "**/SPEC.md",
            "languages/api/src/**",
            "dev/src/**",
        ],
    },
    Generated {
        name: "langs",
        inputs: &[
            "README.md",
            "Cargo.toml",
            "**/SPEC.md",
            "languages/api/src/**",
            "dev/src/**",
        ],
    },
    Generated {
        name: "notices",
        inputs: &[
            "docs/THIRD-PARTY-NOTICES.md",
            "Cargo.lock",
            "**/Cargo.toml",
            "flake.lock",
            "nix/package.nix",
            "nix/tools.nix",
            "**/UPSTREAM",
            "**/LICENSE",
            "**/LICENSE.txt",
            "**/NOTICE.txt",
            "dev/src/**",
        ],
    },
];

/// Does one changed path match one declared input? Three shapes and no
/// more: a literal path, a `dir/**` prefix and a `**/name` suffix.
#[must_use]
pub fn matches(pattern: &str, path: &str) -> bool {
    if let Some(prefix) = pattern.strip_suffix("/**") {
        return path == prefix || path.starts_with(&format!("{prefix}/"));
    }
    if let Some(name) = pattern.strip_prefix("**/") {
        return path == name || path.ends_with(&format!("/{name}"));
    }
    pattern == path
}

/// The outputs worth comparing, given what changed. An EMPTY change set is
/// "no scope given" and selects every output -- the shape `hk check --all`,
/// pre-push and CI use.
#[must_use]
pub fn selected(changed: &[String]) -> Vec<&'static str> {
    OUTPUTS
        .iter()
        .filter(|o| {
            changed.is_empty()
                || o.inputs
                    .iter()
                    .any(|p| changed.iter().any(|c| matches(p, c)))
        })
        .map(|o| o.name)
        .collect()
}
