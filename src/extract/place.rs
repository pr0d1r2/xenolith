//! Placement: where an extract goes, and which layer said so
//! (`src/extract:V45` to `src/extract:V48`, `src/extract:T50`).
//!
//! Three layers, highest first, resolved PER FIELD rather than per
//! site (`src/extract:V45`):
//!
//! * B -- the most specific `[[extract.rule]]` that sets the field;
//!   specificity is how many of `host`, `sink`, `guest` it matched, a
//!   tie goes to the nearer `xenolith.toml`, and a tie inside one file
//!   is refused naming both rules.
//! * C -- `[extract] layout`, which decides the path only.
//! * D -- the host's [`Placement`] and the guest's own defaults.
//!
//! Every field carries the [`Layer`] that decided it, so `xnl extract
//! --verbose` explains a placement instead of leaving it to be guessed
//! (`src/extract:V48`). Paths are templates over a closed set of
//! variables (`src/extract:V46`), rendered and then folded to one
//! repo-root relative spelling; a template that climbs out of the root
//! is refused, never clamped.

use std::fmt;

use xenolith_lang_api::{LangId, Placement};

use crate::config::tree::file_in;
use crate::config::{Base, ExtractRule, Layout, Prelude, Tree, defaults};

#[cfg(test)]
mod tests;

/// One `[[extract.rule]]`, with where it was declared.
#[derive(Debug, Clone)]
pub struct RuleAt<'a> {
    /// The `xenolith.toml` declaring it, root relative.
    pub file: String,
    /// How many directories below the root that file sits: the nearer
    /// file wins a tie (`src/extract:V45`).
    pub depth: usize,
    /// 1-based position among that file's rules, as `--verbose` names
    /// it.
    pub index: usize,
    /// The rule itself.
    pub rule: &'a ExtractRule,
}

/// Every rule a site in `host_path` can match: those of the root's
/// config and of each nested `xenolith.toml` on the way down to the
/// host's directory, root first.
#[must_use]
pub fn rules_for<'a>(tree: &'a Tree, host_path: &str) -> Vec<RuleAt<'a>> {
    let mut out = Vec::new();
    for (dir, config) in tree.layers() {
        let governs = dir.is_empty() || host_path.starts_with(&format!("{dir}/"));
        if !governs {
            continue;
        }
        let depth = if dir.is_empty() {
            0
        } else {
            dir.split('/').count()
        };
        for (at, rule) in config.extract.rules.iter().enumerate() {
            out.push(RuleAt {
                file: file_in(dir),
                depth,
                index: at + 1,
                rule,
            });
        }
    }
    out
}

/// Which layer decided a field (`src/extract:V48`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Layer {
    /// An `[[extract.rule]]`: its 1-based index and its file.
    Rule(usize, String),
    /// `[extract] layout`.
    Layout,
    /// The host's placement or runtime base.
    Host,
    /// The guest's defaults.
    Guest,
}

impl fmt::Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Layer::Rule(index, file) => write!(f, "rule #{index} in {file}"),
            Layer::Layout => f.write_str(leaf("extract.layout")),
            Layer::Host => f.write_str("host"),
            Layer::Guest => f.write_str("guest"),
        }
    }
}

/// A placement field (`src/extract:V45`), in the order `--verbose`
/// explains them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    /// The extract's path.
    Path,
    /// The extract's name.
    Name,
    /// What the load is relative to.
    Base,
    /// The argv the load runs.
    Invoke,
    /// The lines above the body.
    Prelude,
    /// The executable bit.
    Executable,
    /// The companion file.
    Companion,
}

impl fmt::Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Field::Path => "path",
            Field::Name => "name",
            Field::Base => leaf("extract.rule.base"),
            Field::Invoke => "invoke",
            Field::Prelude => "prelude",
            Field::Executable => "executable",
            Field::Companion => "companion",
        })
    }
}

/// The last segment of a key in the defaults table: how an engine names
/// a config key without spelling it (`src/config:V73`).
fn leaf(key: &str) -> &'static str {
    defaults::TABLE
        .iter()
        .find(|entry| entry.key == key)
        .and_then(|entry| entry.key.rsplit('.').next())
        .unwrap_or_default()
}

/// What is known about one site before it is placed.
#[derive(Debug, Clone)]
pub struct Ask<'a> {
    /// The host language.
    pub host: LangId,
    /// The host file, repo-root relative.
    pub host_path: &'a str,
    /// The sink, in the host's vocabulary.
    pub sink: &'a str,
    /// The guest language.
    pub guest: LangId,
    /// The extension the guest gives an extract here, without the dot.
    pub ext: &'a str,
    /// The host's own placement (layer D), or why it has none.
    pub placement: Result<Placement, String>,
    /// `[extract] layout` (layer C).
    pub layout: Layout,
    /// `[extract] root`, for the layouts that place under it.
    pub root: &'a str,
}

/// A placed site: every field of `src/extract:V45`, and who decided it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placed {
    /// The extract, repo-root relative and folded.
    pub path: String,
    /// The extract's name, as the host chose it.
    pub name: String,
    /// What the load is relative to at runtime.
    pub base: Base,
    /// A rule's argv template; `None` is the guest's `invoke`.
    pub invoke: Option<Vec<String>>,
    /// A rule's prelude; `None` is the guest's.
    pub prelude: Option<Prelude>,
    /// A rule's executable bit; `None` is the guest's.
    pub executable: Option<bool>,
    /// A rule's companion template; `None` is none.
    pub companion: Option<String>,
    /// Each field and the layer that decided it, in field order.
    pub why: Vec<(Field, Layer)>,
}

/// The template variables of `src/extract:V46`. `path` and `path_stem`
/// are the extract as loaded, known only once the path is placed, so
/// they are `None` while the path itself is rendered.
#[derive(Debug, Clone, Default)]
pub struct Vars {
    /// The host's name for the extract.
    pub name: String,
    /// The extract's extension.
    pub ext: String,
    /// The host file's directory, `""` at the root.
    pub host_dir: String,
    /// The host file's name without its extension.
    pub host_stem: String,
    /// The sink.
    pub sink: String,
    /// The guest language's name.
    pub guest: String,
    /// The extract path as loaded.
    pub path: Option<String>,
    /// The same without its extension.
    pub path_stem: Option<String>,
}

impl Vars {
    /// The variables for `ask`, `name` unset.
    #[must_use]
    pub fn of(ask: &Ask<'_>) -> Vars {
        let (host_dir, file) = ask
            .host_path
            .rsplit_once('/')
            .unwrap_or(("", ask.host_path));
        Vars {
            name: String::new(),
            ext: ask.ext.to_owned(),
            host_dir: host_dir.to_owned(),
            host_stem: stem(file).to_owned(),
            sink: ask.sink.to_owned(),
            guest: ask.guest.as_str().to_owned(),
            path: None,
            path_stem: None,
        }
    }

    fn get(&self, var: &str) -> Result<&str, String> {
        let unknown = || {
            format!(
                "`{{{var}}}` is the extract path itself, so it cannot place it \
                 (src/extract:V46)"
            )
        };
        match var {
            "name" => Ok(&self.name),
            "ext" => Ok(&self.ext),
            "host_dir" => Ok(&self.host_dir),
            "host_stem" => Ok(&self.host_stem),
            "sink" => Ok(&self.sink),
            "guest" => Ok(&self.guest),
            "path" => self.path.as_deref().ok_or_else(unknown),
            "path_stem" => self.path_stem.as_deref().ok_or_else(unknown),
            other => Err(format!(
                "unknown template variable `{{{other}}}`: the closed set is {{name}}, {{ext}}, \
                 {{host_dir}}, {{host_stem}}, {{sink}}, {{guest}}, {{path}}, {{path_stem}} \
                 (src/extract:V46)"
            )),
        }
    }
}

/// A file name without its last extension; a leading dot is part of
/// the name, not an extension.
fn stem(file: &str) -> &str {
    match file.rfind('.') {
        Some(at) if at > 0 => file.get(..at).unwrap_or(file),
        _ => file,
    }
}

/// `template` with every `{var}` replaced (`src/extract:V46`).
///
/// # Errors
///
/// An unknown variable, `{path}` before the path exists, or an
/// unclosed `{`.
pub fn render(template: &str, vars: &Vars) -> Result<String, String> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(rest.get(..open).unwrap_or_default());
        let after = rest.get(open + 1..).unwrap_or_default();
        let close = after
            .find('}')
            .ok_or_else(|| format!("template `{template}` has an unclosed `{{`"))?;
        out.push_str(vars.get(after.get(..close).unwrap_or_default())?);
        rest = after.get(close + 1..).unwrap_or_default();
    }
    out.push_str(rest);
    Ok(out)
}

/// `path` as one repo-root relative spelling: empty and `.` segments
/// dropped, `..` taken back.
///
/// # Errors
///
/// A path that is absolute, climbs above the root, or folds to nothing.
pub fn fold(path: &str) -> Result<String, String> {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err(format!("`{path}` climbs above the repository root"));
                }
            }
            other => parts.push(other),
        }
    }
    if parts.is_empty() {
        return Err(format!("`{path}` names no file"));
    }
    Ok(parts.join("/"))
}

/// Whether a rule's sink glob matches `sink`: dotted segments one for
/// one, `*` matching any run inside a segment, never a `.`.
#[must_use]
pub fn sink_matches(glob: &str, sink: &str) -> bool {
    let globs: Vec<&str> = glob.split('.').collect();
    let parts: Vec<&str> = sink.split('.').collect();
    globs.len() == parts.len()
        && globs
            .iter()
            .zip(&parts)
            .all(|(g, p)| segment_matches(g.as_bytes(), p.as_bytes()))
}

fn segment_matches(glob: &[u8], text: &[u8]) -> bool {
    match glob.split_first() {
        None => text.is_empty(),
        Some((b'*', rest)) => (0..=text.len()).any(|n| {
            text.get(n..)
                .is_some_and(|tail| segment_matches(rest, tail))
        }),
        Some((c, rest)) => text
            .split_first()
            .is_some_and(|(t, tail)| t == c && segment_matches(rest, tail)),
    }
}

/// How many of a rule's match keys hold for `ask`, or `None` when one
/// does not.
fn specificity(rule: &ExtractRule, ask: &Ask<'_>) -> Option<usize> {
    let host = rule.host.map(|h| h == ask.host);
    let sink = rule.sink.as_deref().map(|g| sink_matches(g, ask.sink));
    let guest = rule.guest.map(|g| g == ask.guest);
    let keys = [host, sink, guest];
    if keys.contains(&Some(false)) {
        return None;
    }
    Some(keys.iter().filter(|k| k.is_some()).count())
}

/// The rule deciding one field: the most specific matching rule that
/// sets it, the nearer file on a tie (`src/extract:V45`).
fn pick<'r, T>(
    field: Field,
    ask: &Ask<'_>,
    rules: &[RuleAt<'r>],
    get: impl Fn(&'r ExtractRule) -> Option<T>,
) -> Result<Option<(T, Layer)>, String> {
    let mut best: Option<((usize, usize), &RuleAt<'r>, T)> = None;
    for at in rules {
        let (Some(spec), Some(value)) = (specificity(at.rule, ask), get(at.rule)) else {
            continue;
        };
        let rank = (spec, at.depth);
        match &best {
            Some((top, other, _)) if *top == rank => {
                return Err(format!(
                    "rule #{} and rule #{} in {} both set `{field}` for sink `{}` at equal \
                     specificity: say which wins by making one more specific \
                     (src/extract:V45)",
                    other.index, at.index, at.file, ask.sink
                ));
            }
            Some((top, _, _)) if *top > rank => {}
            _ => best = Some((rank, at, value)),
        }
    }
    Ok(best.map(|(_, at, value)| (value, Layer::Rule(at.index, at.file.clone()))))
}

/// Place one site (`src/extract:V45`, `src/extract:V46`).
///
/// # Errors
///
/// Two rules of one file tied on a field, a template the closed set
/// does not cover, a path climbing out of the root, or no path at all:
/// the host has no placement and no rule sets one.
pub fn resolve(ask: &Ask<'_>, rules: &[RuleAt<'_>]) -> Result<Placed, String> {
    let mut vars = Vars::of(ask);
    let dir = match &ask.placement {
        Ok(placement) => {
            vars.name = render(&placement.name, &vars)?;
            Some(render(&placement.dir, &vars)?)
        }
        Err(_) => None,
    };
    let (path, layer) =
        if let Some((template, layer)) = pick(Field::Path, ask, rules, |r| r.path.clone())? {
            let named = dir.is_some();
            (rule_path(ask, &vars, named, &template, &layer)?, layer)
        } else {
            layout_path(ask, &vars, dir.as_deref())?
        };
    let base = pick(Field::Base, ask, rules, |r| r.base.clone())?;
    let invoke = pick(Field::Invoke, ask, rules, |r| r.invoke.clone())?;
    let prelude = pick(Field::Prelude, ask, rules, |r| r.prelude.clone())?;
    let executable = pick(Field::Executable, ask, rules, |r| r.executable)?;
    let companion = pick(Field::Companion, ask, rules, |r| r.companion.clone())?;
    let why = vec![
        (Field::Path, layer),
        (Field::Name, Layer::Host),
        (Field::Base, decided(base.as_ref(), Layer::Host)),
        (Field::Invoke, decided(invoke.as_ref(), Layer::Guest)),
        (Field::Prelude, decided(prelude.as_ref(), Layer::Guest)),
        (
            Field::Executable,
            decided(executable.as_ref(), Layer::Guest),
        ),
        (Field::Companion, decided(companion.as_ref(), Layer::Guest)),
    ];
    Ok(Placed {
        path,
        name: vars.name,
        base: base.map_or(Base::Host, |(b, _)| b),
        invoke: invoke.map(|(v, _)| v),
        prelude: prelude.map(|(v, _)| v),
        executable: executable.map(|(v, _)| v),
        companion: companion.map(|(v, _)| v),
        why,
    })
}

/// The layer a picked field names, or `default` when no rule set it.
fn decided<T>(picked: Option<&(T, Layer)>, default: Layer) -> Layer {
    picked.map_or(default, |(_, layer)| layer.clone())
}

/// A rule's `path` template, rendered and folded.
fn rule_path(
    ask: &Ask<'_>,
    vars: &Vars,
    named: bool,
    template: &str,
    layer: &Layer,
) -> Result<String, String> {
    if template.starts_with('/') {
        return Err(format!(
            "{layer}: path `{template}` is absolute; extracts live in the repository \
             (src/extract:V46)"
        ));
    }
    if !named && template.contains("{name}") {
        return Err(no_placement(ask));
    }
    fold(&render(template, vars)?)
}

/// The path `[extract] layout` gives, from the host's name and, for the
/// `host` layout, its directory.
fn layout_path(ask: &Ask<'_>, vars: &Vars, dir: Option<&str>) -> Result<(String, Layer), String> {
    let Some(dir) = dir else {
        return Err(no_placement(ask));
    };
    let file = format!("{}.{}", vars.name, ask.ext);
    let (path, layer) = match ask.layout {
        Layout::Host => (format!("{dir}/{file}"), Layer::Host),
        Layout::Mirror => (
            format!("{}/{}/{file}", ask.root, stem_path(ask.host_path)),
            Layer::Layout,
        ),
        Layout::Sibling => (
            format!("{}/{}.{file}", vars.host_dir, vars.host_stem),
            Layer::Layout,
        ),
        Layout::Central => (format!("{}/{}/{file}", ask.root, vars.guest), Layer::Layout),
    };
    Ok((fold(&path)?, layer))
}

fn no_placement(ask: &Ask<'_>) -> String {
    let detail = ask.placement.as_ref().err().map_or("", String::as_str);
    format!(
        "no place for sink `{}`: the {} host names no extract ({detail}) and no \
         [[extract.rule]] sets `path` (src/extract:V45)",
        ask.sink, ask.host
    )
}

/// A repo path without its extension: `nix/foo.nix` -> `nix/foo`.
fn stem_path(path: &str) -> String {
    match path.rsplit_once('/') {
        Some((dir, file)) => format!("{dir}/{}", stem(file)),
        None => stem(path).to_owned(),
    }
}

/// The path a host's load names for the extract at `extract` (repo-root
/// relative), relative to the site's runtime base
/// (`languages/api/src/lens:V66`): the host file's directory unless a
/// rule's `base` says otherwise. `./`-led unless it climbs, so a host
/// whose syntax needs the dot (a nix path) has it, and never relative
/// to the working directory.
#[must_use]
pub fn load_path(host_path: &str, extract: &str, base: &Base) -> String {
    let from: Vec<&str> = match base {
        Base::Host => host_path
            .rsplit_once('/')
            .map_or_else(Vec::new, |(dir, _)| segments(dir)),
        Base::Root => Vec::new(),
        Base::Dir(dir) => segments(dir),
    };
    let to = segments(extract);
    let shared = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let ups = from.len() - shared;
    let rest = to.get(shared..).unwrap_or_default().join("/");
    if ups == 0 {
        format!("./{rest}")
    } else {
        format!("{}{rest}", "../".repeat(ups))
    }
}

/// Refuse a path that could break out of the host syntax it is written
/// into -- a nix path, a pkl string, a yaml scalar (`src/extract:V83`):
/// only `[A-Za-z0-9._/-]`, and no segment led by `-`, which a command
/// would read as a flag.
///
/// # Errors
///
/// The path and the rule it breaks.
pub fn charset(path: &str) -> Result<(), String> {
    let plain = path
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "._/-".contains(c));
    let flag = path.split('/').any(|segment| segment.starts_with('-'));
    if plain && !flag {
        return Ok(());
    }
    Err(format!(
        "`{path}` holds a character outside [A-Za-z0-9._/-] or a segment led by `-`, which \
         the host's syntax could read as its own (src/extract:V83)"
    ))
}

/// The non-empty, non-`.` segments of a `/`-separated path.
fn segments(path: &str) -> Vec<&str> {
    path.split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .collect()
}

/// The suffix a colliding extract takes (`src/extract:V47`): the sink's
/// last dotted segment, lowercased, anything outside `[a-z0-9]` a `-`.
#[must_use]
pub fn suffix(sink: &str) -> String {
    let last = sink.rsplit('.').next().unwrap_or(sink);
    let mapped: String = last
        .chars()
        .map(|c| {
            let c = c.to_ascii_lowercase();
            if c.is_ascii_lowercase() || c.is_ascii_digit() {
                c
            } else {
                '-'
            }
        })
        .collect();
    mapped.trim_matches('-').to_owned()
}

/// `path` with `-<suffix>` before its extension.
fn suffixed(path: &str, suffix: &str) -> String {
    let (dir, file) = match path.rsplit_once('/') {
        Some((dir, file)) => (format!("{dir}/"), file),
        None => (String::new(), path),
    };
    let base = stem(file);
    let ext = file.get(base.len()..).unwrap_or_default();
    format!("{dir}{base}-{suffix}{ext}")
}

/// Disambiguate the placed paths of one run (`src/extract:V47`): each
/// `(path, sink)` sharing its path with another takes its sink's
/// suffix. Returns the indices whose paths are STILL shared, which
/// `src/extract:V6` refuses.
pub fn disambiguate(placed: &mut [(String, String)]) -> Vec<usize> {
    let shared = |placed: &[(String, String)], at: usize| {
        placed.get(at).is_some_and(|(path, _)| {
            placed
                .iter()
                .enumerate()
                .any(|(other, (p, _))| other != at && p == path)
        })
    };
    let colliding: Vec<usize> = (0..placed.len()).filter(|&i| shared(placed, i)).collect();
    for &at in &colliding {
        if let Some((path, sink)) = placed.get_mut(at) {
            *path = suffixed(path, &suffix(sink));
        }
    }
    (0..placed.len()).filter(|&i| shared(placed, i)).collect()
}
