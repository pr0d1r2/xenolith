//! The README badge block, rendered from the files that own each number
//! (`dev:V340`).
//!
//! Every function here is a pure function of `&str`: the caller reads the
//! repository and hands the text in, so the whole set is testable from a
//! string literal (`dev` §C). The readers are deliberately NOT a TOML parser
//! or a Pkl evaluator -- each reads the one shape its owner writes, and a
//! parser for two strings would be a dependency this crate does not want.
//!
//! Ported from sherd's `dev/src/badge.rs` (`dev:R341`), which paid for two
//! of the lessons below in its own bug log: a flake.lock prefix match that
//! read the wrong node, and a step count that also counted hooks.

use std::fmt::Write as _;

#[cfg(test)]
mod tests;

/// The value of the first `key = "value"` line, unquoted.
///
/// Reads the root manifest's `[workspace.package]` keys, which is where
/// `edition`, `rust-version` and `license` live here: a member's
/// `edition.workspace = true` does not begin `edition = `, so it is never
/// the match.
#[must_use]
pub fn manifest_value(manifest: &str, key: &str) -> Option<String> {
    manifest
        .lines()
        .map(str::trim_end)
        .find_map(|l| l.strip_prefix(key)?.strip_prefix(" = "))
        .map(|v| v.trim_matches('"').to_string())
}

/// The quoted strings of a `key = [ ... ]` array, which may span lines
/// and carry comments between its entries.
fn string_array(manifest: &str, key: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut inside = false;
    for line in manifest.lines() {
        let t = line.trim();
        let rest = if inside {
            t
        } else if let Some(r) = t.strip_prefix(key).and_then(|r| r.strip_prefix(" = [")) {
            inside = true;
            r
        } else {
            continue;
        };
        let body = rest.split('#').next().unwrap_or_default();
        let (items, closed) = match body.split_once(']') {
            Some((items, _)) => (items, true),
            None => (body, false),
        };
        out.extend(
            items
                .split(',')
                .map(|i| i.trim().trim_matches('"'))
                .filter(|i| !i.is_empty())
                .map(str::to_string),
        );
        if closed {
            break;
        }
    }
    out
}

/// The workspace members, in the order the root manifest lists them.
#[must_use]
pub fn members(manifest: &str) -> Vec<String> {
    string_array(manifest, "members")
}

/// The languages the default build compiles in: the `default` feature's
/// `lang-*` entries, without the prefix.
#[must_use]
pub fn default_languages(manifest: &str) -> Vec<String> {
    string_array(manifest, "default")
        .into_iter()
        .filter_map(|f| f.strip_prefix("lang-").map(str::to_string))
        .collect()
}

/// Classification shared by the badge and language table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageKind {
    /// Enabled by the default feature.
    Default,
    /// Declared as a non-default cargo feature.
    OptIn,
    /// No cargo language feature is declared.
    Planned,
}

/// Classify a language from the manifest's declared and default features.
#[must_use]
pub fn language_kind(id: &str, declared: &[String], default: &[String]) -> LanguageKind {
    let feature = format!("lang-{id}");
    if !declared.contains(&feature) {
        LanguageKind::Planned
    } else if default.iter().any(|d| d == id) {
        LanguageKind::Default
    } else {
        LanguageKind::OptIn
    }
}

/// The keys of one `[table]`, each once, with the text of its entry.
///
/// A key is counted where it STARTS, never per line: a formatter wrapping
/// one entry's `features = [` across lines must not turn one dependency into
/// three (sherd's `direct_dependencies`, measured on its own manifest).
fn table_entries(manifest: &str, table: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut inside = false;
    let mut depth: usize = 0;
    for line in manifest.lines() {
        let t = line.trim();
        if depth == 0 && t.starts_with('[') {
            inside = t == table;
            continue;
        }
        if !inside || t.starts_with('#') || t.is_empty() {
            continue;
        }
        if depth == 0 {
            if let Some((key, value)) = t.split_once('=') {
                out.push((key.trim().to_string(), value.to_string()));
            }
        } else if let Some(last) = out.last_mut() {
            last.1.push_str(t);
        }
        let opens = t.matches(['{', '[']).count();
        let closes = t.matches(['}', ']']).count();
        depth = depth.saturating_add(opens).saturating_sub(closes);
    }
    out
}

/// Every `[dependencies]` key, and whether it is a `path` dependency (a
/// crate of this workspace rather than a third party).
#[must_use]
pub fn dependencies(manifest: &str) -> Vec<(String, bool)> {
    table_entries(manifest, "[dependencies]")
        .into_iter()
        .map(|(k, v)| (k, v.contains("path =")))
        .collect()
}

/// Every feature the manifest declares.
#[must_use]
pub fn features(manifest: &str) -> Vec<String> {
    table_entries(manifest, "[features]")
        .into_iter()
        .map(|(k, _)| k)
        .collect()
}

/// Whether a package with this Cargo `publish` value ships.
#[must_use]
pub(crate) fn shipped(publish: Option<&str>) -> bool {
    let Some(value) = publish.map(str::trim) else {
        return true;
    };
    if value == "false" {
        return false;
    }
    // Cargo permits whitespace inside the empty restricted-registry list.
    // Metadata normally serializes this as `[]`, while a manifest may spell
    // it `[ ]`; both mean that the package is not publishable.
    !value
        .chars()
        .filter(|c| !c.is_ascii_whitespace())
        .eq("[]".chars())
}

/// Whether a manifest's package ships, using the same rule as metadata.
#[must_use]
pub fn published(manifest: &str) -> bool {
    let publish = manifest.lines().find_map(|line| {
        let line = line.trim();
        let (key, value) = line.split_once('=')?;
        (key.trim() == "publish").then_some(value)?;
        Some(value.split('#').next().unwrap_or_default().trim())
    });
    shipped(publish)
}

/// The `unsafe_code` level a manifest sets for itself, if it sets one.
#[must_use]
pub fn unsafe_level(manifest: &str) -> Option<String> {
    manifest_value(manifest, "unsafe_code")
}

/// A ratchet file's single number: `lines 97.96`, `density 0.0`.
#[must_use]
pub fn ratchet(text: &str, key: &str) -> Option<String> {
    text.lines()
        .find_map(|l| l.strip_prefix(key)?.strip_prefix(' '))
        .map(str::trim)
        .map(str::to_string)
}

/// A percentage TRUNCATED to one decimal (`dev:V344`): coverage differs by
/// a few hundredths between platforms on one tree, and truncation sends both
/// readings to the same tenth where rounding can split them.
#[must_use]
pub fn truncate_tenth(value: &str) -> Option<String> {
    let (whole, frac) = value.split_once('.').unwrap_or((value, "0"));
    let tenth = frac.chars().next()?;
    let valid = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    (valid(whole) && valid(frac)).then(|| format!("{whole}.{tenth}"))
}

/// What `flake.lock` records for one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Locked {
    /// The pinned rev, short.
    pub rev: String,
    /// The day that rev was last modified, UTC.
    pub date: String,
    /// The release branch it follows (`26.05`), if it follows one.
    pub release: Option<String>,
}

/// Read one NAMED node out of `flake.lock`.
///
/// The node HEADER is matched -- `"nixpkgs": {` -- never any line beginning
/// with the name: an input's own `inputs` block carries `"nixpkgs": [`, and a
/// prefix match enters that block and returns someone else's rev. The block
/// ends at the next header at the same indent.
#[must_use]
pub fn locked(lock: &str, node: &str) -> Option<Locked> {
    let header = format!("\"{node}\": {{");
    let mut indent = None;
    let (mut rev, mut stamp, mut release) = (None, None, None);
    for line in lock.lines() {
        let t = line.trim();
        let this = line.len().saturating_sub(t.len());
        match indent {
            None if t == header => indent = Some(this),
            None => {}
            Some(i) if this == i && t.ends_with('{') => break,
            Some(_) => {
                if let Some(v) = field(t, "rev") {
                    rev.get_or_insert(v.chars().take(7).collect::<String>());
                }
                if let Some(v) = field(t, "lastModified") {
                    stamp.get_or_insert(v);
                }
                if let Some(v) = field(t, "ref") {
                    release.get_or_insert(v);
                }
            }
        }
    }
    Some(Locked {
        rev: rev?,
        date: stamp.and_then(|s| s.parse().ok()).map(civil_date)?,
        release: release.map(|r: String| {
            r.trim_start_matches("nixos-")
                .trim_start_matches("release-")
                .to_string()
        }),
    })
}

fn field(line: &str, key: &str) -> Option<String> {
    let rest = line.strip_prefix(&format!("\"{key}\":"))?;
    Some(
        rest.trim()
            .trim_end_matches(',')
            .trim_matches('"')
            .to_string(),
    )
}

/// A unix timestamp as `YYYY-MM-DD`, UTC: Hinnant's civil-from-days, exact
/// over every date a lock file can hold. A date crate for one format string
/// would be a dependency for nothing.
#[must_use]
pub fn civil_date(epoch_secs: i64) -> String {
    let days = epoch_secs.div_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// How many steps the gate declares, counted INSIDE `local fast` and
/// `local all` only (`dev:V344`): `check` is both a hook and a step in
/// hk.pkl, so excluding hooks by name would drop a real step.
#[must_use]
pub fn gate_steps(pkl: &str) -> usize {
    let mut inside = false;
    let mut n: usize = 0;
    for line in pkl.lines() {
        if line.starts_with("local fast") || line.starts_with("local all") {
            inside = true;
        } else if line.starts_with('}') {
            inside = false;
        } else if inside && line.starts_with("  [\"") {
            n = n.saturating_add(1);
        }
    }
    n
}

/// The platforms CI gates, from the workflow's `os:` matrix (`dev:V342`) --
/// never from the flake's `systems`, which declares one CI never builds.
/// Each as `(vendor logo, os)`, sorted.
#[must_use]
pub fn ci_platforms(workflow: &str) -> Result<Vec<(String, String)>, String> {
    let lists: Vec<&str> = workflow
        .lines()
        .filter_map(|l| {
            let t = l.trim();
            t.strip_prefix("os: [")?.strip_suffix(']')
        })
        .collect();
    let Some(list) = lists.first() else {
        return Ok(Vec::new());
    };
    if lists.iter().skip(1).any(|other| other != list) {
        return Err("xenolith-dev: CI has disagreeing os matrices".to_string());
    }
    let mut out = Vec::new();
    for runner in list.split(',').map(str::trim) {
        let (os, vendors): (&str, &[&str]) = if runner == "macos-latest" {
            ("macos", &["arm"])
        } else if runner == "macos-13" {
            ("macos", &["intel"])
        } else if runner.contains("arm") {
            ("linux", &["arm"])
        } else if runner.starts_with("ubuntu") {
            ("linux", &["intel", "amd"])
        } else {
            return Err(format!("xenolith-dev: unknown CI runner `{runner}`"));
        };
        out.extend(vendors.iter().map(|v| ((*v).to_string(), os.to_string())));
    }
    out.sort();
    out.dedup();
    Ok(out)
}

/// Every file the block is rendered from, read by the CALLER (`dev` §C).
#[derive(Debug, Clone, Default)]
pub struct Sources {
    /// The root `Cargo.toml`.
    pub manifest: String,
    /// Each workspace member's path and `Cargo.toml`, in `members` order.
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
    /// `license` of the workspace package.
    pub license: String,
    /// `edition`.
    pub edition: String,
    /// `rust-version`.
    pub msrv: String,
    /// Distinct third-party `[dependencies]` over the shipped manifests.
    pub deps: usize,
    /// Shipped crates that `deny` rather than `forbid` unsafe code.
    pub unsafe_deny: usize,
    /// Languages the default build compiles in.
    pub languages: usize,
    /// Languages known to `LangId::ALL` and not in the default build.
    pub planned: usize,
    /// Steps in `local fast` and `local all`.
    pub gate_steps: usize,
    /// The coverage floor, truncated to a tenth.
    pub coverage_floor: String,
    /// Clippy warnings per thousand lines.
    pub lint_debt: String,
    /// `SPEC.md` nodes in the tree.
    pub nodes: usize,
    /// The nixpkgs node of `flake.lock`.
    pub nixpkgs: Locked,
    /// `(vendor, os)` per CI runner.
    pub platforms: Vec<(String, String)>,
}

/// Gather every fact, or name the owner that had nothing to say (`dev:V340`).
///
/// # Errors
/// An owning file missing the value it owns.
pub fn facts(s: &Sources, nodes: usize, known_languages: &[&str]) -> Result<Facts, String> {
    let missing = |what: &str| format!("xenolith-dev: no {what} to read (dev:V340)");
    let lines = ratchet(&s.coverage, "lines").ok_or_else(|| missing("`lines` row in .coverage"))?;
    let root_level =
        unsafe_level(&s.manifest).ok_or_else(|| missing("`unsafe_code` lint in Cargo.toml"))?;
    if root_level != "forbid" {
        return Err(format!(
            "xenolith-dev: Cargo.toml sets unsafe_code = \"{root_level}\"; the badge knows \
             `forbid` with named exceptions only (dev:V346)"
        ));
    }
    // SHIPPED = the root and every member not marked `publish = false`
    // (`dev:V346`): this crate's own dependencies are nobody's concern.
    let members: Vec<&str> = s
        .members
        .iter()
        .map(|(_, m)| m.as_str())
        .filter(|m| published(m))
        .collect();
    let mut third_party: Vec<String> = std::iter::once(s.manifest.as_str())
        .chain(members.iter().copied())
        .flat_map(dependencies)
        .filter(|(_, path)| !path)
        .map(|(name, _)| name)
        .collect();
    third_party.sort();
    third_party.dedup();
    let unsafe_deny = members
        .iter()
        .filter(|m| unsafe_level(m).as_deref() == Some("deny"))
        .count();
    let languages = default_languages(&s.manifest).len();
    let declared = features(&s.manifest);
    let default = default_languages(&s.manifest);
    let planned = known_languages
        .iter()
        .filter(|id| {
            matches!(
                language_kind(id, &declared, &default),
                LanguageKind::Planned
            )
        })
        .count();
    Ok(Facts {
        license: manifest_value(&s.manifest, "license")
            .ok_or_else(|| missing("`license` in Cargo.toml"))?,
        edition: manifest_value(&s.manifest, "edition")
            .ok_or_else(|| missing("`edition` in Cargo.toml"))?,
        msrv: manifest_value(&s.manifest, "rust-version")
            .ok_or_else(|| missing("`rust-version` in Cargo.toml"))?,
        deps: third_party.len(),
        unsafe_deny,
        languages,
        planned,
        gate_steps: gate_steps(&s.pkl),
        coverage_floor: truncate_tenth(&lines)
            .ok_or_else(|| missing("a number in .coverage's `lines` row"))?,
        lint_debt: ratchet(&s.debt, "density")
            .ok_or_else(|| missing("`density` row in .lint-debt"))?,
        nodes,
        nixpkgs: locked(&s.lock, "nixpkgs")
            .ok_or_else(|| missing("`nixpkgs` node in flake.lock"))?,
        platforms: ci_platforms(&s.workflow)?,
    })
}

const SHIELD: &str = "https://img.shields.io/badge";

/// One field of a shields.io static badge: a literal dash is a field
/// separator unless doubled, a literal underscore a space unless doubled,
/// and a space is written as an underscore.
#[must_use]
pub fn shield_text(raw: &str) -> String {
    raw.replace('_', "__").replace('-', "--").replace(' ', "_")
}

/// Render the block, in sherd's order and colours (`dev:R340`). No CI,
/// crates.io or docs.rs badge: nothing is published yet, and each would be
/// a broken image or a tick for a run nobody made (`dev:V341`).
#[must_use]
pub fn render(f: &Facts) -> String {
    let mut s = String::new();
    let (lic, ed, msrv, deps) = (&f.license, &f.edition, &f.msrv, f.deps);
    let lic_field = shield_text(lic);
    let (unsafe_alt, unsafe_msg, unsafe_colour) = if f.unsafe_deny == 0 {
        (
            "unsafe forbidden".to_string(),
            "forbidden".to_string(),
            "brightgreen",
        )
    } else {
        let n = f.unsafe_deny;
        (
            format!("unsafe forbidden, deny in {n} FFI crates"),
            format!("forbidden, deny in {n} FFI crates"),
            "yellowgreen",
        )
    };
    let unsafe_msg = shield_text(&unsafe_msg);
    let (built, planned) = (f.languages, f.planned);
    let _ = write!(
        s,
        "[![License: {lic}]({SHIELD}/license-{lic_field}-blue.svg)](LICENSE)\n\
         [![edition {ed}]({SHIELD}/edition-{ed}-000000?logo=rust&logoColor=white)](Cargo.toml)\n\
         [![MSRV {msrv}]({SHIELD}/MSRV-{msrv}-000000?logo=rust&logoColor=white)](Cargo.toml)\n\
         [![direct dependencies {deps}]({SHIELD}/direct_dependencies-{deps}-brightgreen)]\
         (docs/THIRD-PARTY-NOTICES.md)\n\
         [![{unsafe_alt}]({SHIELD}/unsafe-{unsafe_msg}-{unsafe_colour})](Cargo.toml)\n\
         [![languages {built} built, {planned} planned]({SHIELD}/languages-{built}_built,\
         _{planned}_planned-6E4AFF)](languages/SPEC.md)\n\n"
    );
    let (steps, cov, debt, nodes) = (f.gate_steps, &f.coverage_floor, &f.lint_debt, f.nodes);
    let _ = write!(
        s,
        "[![gate hk]({SHIELD}/gate-hk-6E4AFF)](hk.pkl)\n\
         [![gate steps {steps}]({SHIELD}/gate_steps-{steps}-6E4AFF)](hk.pkl)\n\
         [![coverage floor {cov}%]({SHIELD}/coverage_floor-%E2%89%A5{cov}%25-brightgreen)]\
         (.coverage)\n\
         [![lint debt {debt}/KLoC]({SHIELD}/lint_debt-%E2%89%A4{debt}%2FKLoC-orange)]\
         (.lint-debt)\n\
         [![federated nodes {nodes}]({SHIELD}/federated_nodes-{nodes}-6E4AFF)](SPEC.md)\n\n"
    );
    let np = &f.nixpkgs;
    let release = np.release.as_deref().unwrap_or("unpinned");
    let label = format!(
        "{}_({}_--_{})",
        shield_text(release),
        shield_text(&np.date),
        np.rev
    );
    let shown = format!("{release} ({} - {})", np.date, np.rev);
    let _ = write!(
        s,
        "[![nix flake]({SHIELD}/nix-flake-5277C3?logo=nixos&logoColor=white)](flake.nix)\n\
         [![nixpkgs {shown}]({SHIELD}/nixpkgs-{label}-5277C3?logo=nixos&logoColor=white)]\
         (flake.lock)\n"
    );
    for (vendor, os) in &f.platforms {
        let _ = writeln!(
            s,
            "[![{vendor} {os}]({SHIELD}/{os}-5277C3?logo={vendor}&logoColor=white)]\
             (.github/workflows/ci.yml)"
        );
    }
    s.push_str(
        "\n[![built with Claude Code](https://img.shields.io/badge/built_with-Claude_Code-D97757)]\
         (https://claude.com/claude-code)\n\
         [![built with Opus 5.5](https://img.shields.io/badge/built_with-Opus_5.5-D97757)]\
         (https://www.anthropic.com/claude)\n\
         [![built with SDD](https://img.shields.io/badge/built_with-spec--driven_\
         development-D97757)]\
         (SPEC.md)\n",
    );
    s
}
