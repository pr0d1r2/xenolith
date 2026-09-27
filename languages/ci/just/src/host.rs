//! just as a host: recipe bodies holding shell (`languages/ci/just:T16`).
//!
//! A site is a delimiter AND a sink context (`languages/api/src/site:V38`).
//! In a justfile the sink is the recipe and the delimiter its indented
//! body, both read from the tree; a comment, a string in an assignment or
//! a backtick in an expression is never a site.
//!
//! Two kinds of body (`languages/ci/just` §I):
//!
//! - LINES, `DelimKind::JustRecipe`: just runs each line in a fresh shell
//!   (`set shell`, `sh -cu` by default), so the site's env is that shell
//!   plus `errexit` (`languages/ci/just:V179`) and the body the guest
//!   judges is the lines as just runs them ([`crate::lines`]).
//! - a SCRIPT, `DelimKind::JustShebangRecipe`: a `#!` first line makes
//!   the whole body one file run by that interpreter, so the guest is
//!   what the shebang names and an interpreter the api does not know is
//!   no site at all, never "shell, probably".
//!
//! A `[script]` recipe is neither: its interpreter comes from `set
//! script-interpreter`, which the spec leaves open, so it is no site
//! until that is decided.
//!
//! The load is one recipe line, `<invoke argv>` (`languages/api:V35`),
//! `@`-prefixed when every line it replaces was; everything that would
//! make merging the lines into one script change what runs is refused
//! (`languages/ci/just:V180`) and stays a `Judgment`.

use std::path::{Path, PathBuf};

use tree_sitter::Node;
use xenolith_lang_api::{
    Delim, DelimKind, Error, FileArg, Format, GuestEnv, Host, Invoke, LangId, LintCmd, LoadRef,
    Placement, Result, Site, Span, holes, lens, shebang,
};

use crate::recipe::{self, Recipe, Settings, span, text};
use crate::shell::{self, LineShell};
use crate::{lines, placement, state};

#[cfg(test)]
mod tests;

/// Interpreters a load may name. Recognised, never emitted: the argv of a
/// load is the guest's `invoke` (`languages/api:V35`), and this list only
/// lets `loads` read one back.
const INTERPRETERS: &[&str] = &["bash", "sh", "zsh"];

/// Extensions a loaded script may carry.
const SCRIPT_EXTENSIONS: &[&str] = &["bash", "sh", "zsh"];

/// The shell dialects a shebang recipe's env may name
/// (`languages/shells/shell:V82`).
const DIALECTS: &[&str] = &["sh", "bash", "zsh"];

/// The just host.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct JustHost;

fn refuse(operation: &'static str) -> Error {
    Error::unsupported(LangId::Just, operation)
}

/// The site `recipe` makes, or `None` for a recipe that makes none.
fn site(recipe: &Recipe<'_>, src: &str, shell: Option<&LineShell>) -> Option<Site> {
    if recipe.script {
        return None;
    }
    let body = recipe.body(src)?;
    let (kind, guest, env) = if let Some(line) = recipe.shebang {
        let parsed = shebang::parse(text(line, src))?;
        let guest = shebang::guest_of(&parsed)?;
        let interpreter = parsed.resolved_interpreter();
        let name = interpreter.rsplit('/').next().unwrap_or(interpreter);
        let dialect = DIALECTS
            .iter()
            .find(|d| guest == LangId::Shell && **d == name)
            .map(|d| (*d).to_owned());
        let env = GuestEnv {
            dialect,
            options: Vec::new(),
        };
        (DelimKind::JustShebangRecipe, guest, env)
    } else {
        let env = shell.map(|s| s.env.clone()).unwrap_or_default();
        (DelimKind::JustRecipe, LangId::Shell, env)
    };
    Some(Site {
        sink: recipe.name.clone(),
        guest,
        env,
        delim: Delim {
            kind,
            open: span(recipe.header),
            body,
            close: Span::new(body.end, body.end),
        },
        holes: recipe
            .lines
            .iter()
            .flat_map(|line| recipe::interpolations(*line, src))
            .collect(),
    })
}

/// Every site of `src`, each with the recipe it came from.
fn located<'t>(
    tree: &'t tree_sitter::Tree,
    src: &str,
    settings: &Settings,
) -> Vec<(Site, Recipe<'t>)> {
    let shell = shell::line_shell(settings);
    recipe::recipes(tree, src)
        .into_iter()
        .filter_map(|recipe| site(&recipe, src, shell.as_ref()).map(|s| (s, recipe)))
        .collect()
}

/// The load a recipe line holds: `[@]<interpreter> <script> [args…]`, the
/// script a plain relative path with a shell extension.
fn load(line: Node<'_>, src: &str) -> Option<LoadRef> {
    let prefix = recipe::prefix(line, src);
    let command = recipe::line_text(line, src).get(prefix.len()..)?;
    let words: Vec<&str> = command.split_whitespace().collect();
    let [interpreter, script, ..] = words.as_slice() else {
        return None;
    };
    if !INTERPRETERS.contains(interpreter) || script.contains("{{") || script.starts_with('/') {
        return None;
    }
    let path = PathBuf::from(script);
    let extension = path.extension().and_then(|ext| ext.to_str())?;
    SCRIPT_EXTENSIONS.contains(&extension).then(|| LoadRef {
        span: recipe::line_span(line, src),
        path,
        guest: LangId::Shell,
    })
}

/// Whether an argv word can go into a load as it is: no shell quoting and
/// no just escaping needed, so `loads` reads back exactly what `rewrite`
/// wrote.
fn plain_word(word: &str) -> bool {
    !word.is_empty()
        && word
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || "/._-+,:=%".contains(ch))
}

/// The line break the host uses around `at`: CRLF when the line holding
/// it ends in one, else `\n`.
fn line_break(src: &str, at: Span) -> &'static str {
    let after = src.get(at.end..).and_then(|rest| rest.find('\n'));
    match after.map(|offset| at.end + offset) {
        Some(newline) if newline > 0 && src.as_bytes().get(newline - 1) == Some(&b'\r') => "\r\n",
        _ => "\n",
    }
}

/// `src` with `[at.start, at.end)` replaced by `with`.
fn splice(src: &str, at: Span, with: &str) -> Result<String> {
    let (Some(before), Some(after)) = (src.get(..at.start), src.get(at.end..)) else {
        return Err(Error::parse(LangId::Just, "span outside the source"));
    };
    Ok(format!("{before}{with}{after}"))
}

/// The extract direction for one line site: the host with the load in
/// place of the lines, and the merged script
/// (`languages/ci/just:V180`), or the refusal that keeps it a judgement.
fn extract(src: &str, site: &Site, invoke: &Invoke) -> Result<lens::Rewrite> {
    let tree = recipe::parse(src)?;
    let settings = recipe::settings(&tree, src);
    let found = located(&tree, src, &settings);
    let Some((_, recipe)) = found.iter().find(|(s, _)| s == site) else {
        return Err(Error::parse(
            LangId::Just,
            format!("no recipe `{}` at bytes {:?}", site.sink, site.delim.open),
        ));
    };
    if site.delim.kind == DelimKind::JustShebangRecipe {
        return Err(refuse("rewrite of a shebang recipe"));
    }
    if !site.holes.is_empty() {
        return Err(refuse(
            "rewrite of a recipe with {{…}} holes: pass each as an argument by hand",
        ));
    }
    let Some(shell) = shell::line_shell(&settings) else {
        return Err(refuse(
            "rewrite under a set shell that is not statically readable (languages/ci/just:V179)",
        ));
    };
    if recipe.params && (settings.positional || recipe.positional) {
        return Err(refuse("rewrite of a recipe passing positional arguments"));
    }
    let raw = site.delim.body.of(src).unwrap_or_default();
    let body = lines::logical(raw)?;
    let last = body.len().saturating_sub(1);
    for (i, line) in body.iter().enumerate() {
        if i < last && state::changes_state(&line.text) {
            return Err(refuse(
                "rewrite of a line that changes shell state a later line reads: cd, export, \
                 an assignment, set (languages/ci/just:V180)",
            ));
        }
        if i < last && state::escapes_errexit(&line.text) {
            return Err(refuse(
                "rewrite of an && or ! line that set -e would not stop at \
                 (languages/ci/just:B1)",
            ));
        }
        if !shell.errexit && state::sequences(&line.text) {
            return Err(refuse(
                "rewrite of an a; b line under a shell without -e \
                 (languages/ci/just:B1)",
            ));
        }
    }
    let (merged, quiet) = lines::merged(&body).map_err(refuse)?;
    if invoke.argv.is_empty() || !invoke.argv.iter().all(|word| plain_word(word)) {
        return Err(refuse("rewrite to a load that needs quoting"));
    }
    let first = recipe
        .lines
        .first()
        .ok_or_else(|| Error::parse(LangId::Just, "a line site with no line"))?;
    let load = format!("{}{}", if quiet { "@" } else { "" }, invoke.argv.join(" "));
    Ok(lens::Rewrite {
        src: splice(
            src,
            Span::new(first.start_byte(), site.delim.body.end),
            &load,
        )?,
        body: merged,
    })
}

impl Host for JustHost {
    fn id(&self) -> LangId {
        LangId::Just
    }

    /// `justfile` and `.justfile` in any case, and `*.just`
    /// (`languages/ci/just:V58`).
    fn claims(&self, path: &Path, _head: &str) -> bool {
        let named = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                name.eq_ignore_ascii_case("justfile") || name.eq_ignore_ascii_case(".justfile")
            });
        named || path.extension().is_some_and(|ext| ext == "just")
    }

    fn sites(&self, src: &str) -> Result<Vec<Site>> {
        let tree = recipe::parse(src)?;
        let settings = recipe::settings(&tree, src);
        Ok(located(&tree, src, &settings)
            .into_iter()
            .map(|(site, _)| site)
            .collect())
    }

    /// Every recipe line that runs a script: `[@]<sh|bash|zsh> <path>
    /// [args…]`, in a line body and not continued.
    fn loads(&self, src: &str) -> Result<Vec<LoadRef>> {
        let tree = recipe::parse(src)?;
        let mut out = Vec::new();
        for recipe in recipe::recipes(&tree, src) {
            if recipe.script || recipe.shebang.is_some() {
                continue;
            }
            let mut after_backslash = false;
            for line in &recipe.lines {
                let backslash = recipe::line_text(*line, src).ends_with('\\');
                if !after_backslash && !backslash {
                    out.extend(load(*line, src));
                }
                after_backslash = backslash;
            }
        }
        Ok(out)
    }

    fn rewrite(&self, src: &str, site: &Site, invoke: &Invoke, _path: &Path) -> Result<String> {
        extract(src, site, invoke).map(|rewrite| rewrite.src)
    }

    /// The lines become one load line, and the extract holds them merged
    /// (`languages/ci/just:V180`): a `-` line ending in `|| true`. Holes
    /// are refused, params and all: the load passing them is not decided
    /// yet (`languages/ci/just` §I).
    fn rewrite_bound(
        &self,
        src: &str,
        site: &Site,
        invoke: &Invoke,
        _path: &Path,
        _body: &str,
        params: &[holes::Param],
    ) -> Result<lens::Rewrite> {
        if !params.is_empty() {
            return Err(refuse(
                "rewrite of a recipe with {{…}} holes: pass each as an argument by hand",
            ));
        }
        extract(src, site, invoke)
    }

    /// The load line becomes the body's lines: `@` on each when the load
    /// had it, `-` on each that ends in `|| true`, indented as the load
    /// was, broken the way the host's lines are.
    fn inline(&self, src: &str, load: &LoadRef, body: &str) -> Result<String> {
        if !self.loads(src)?.contains(load) {
            return Err(Error::parse(
                LangId::Just,
                format!("no load of {} at {:?}", load.path.display(), load.span),
            ));
        }
        let line = load.span.of(src).unwrap_or_default();
        let prefix: String = line
            .chars()
            .take_while(|c| matches!(c, '@' | '-'))
            .collect();
        if prefix.contains('-') {
            return Err(refuse("inline over a - load"));
        }
        if line
            .get(prefix.len()..)
            .unwrap_or_default()
            .split_whitespace()
            .count()
            != 2
        {
            return Err(refuse("inline of a load passing arguments"));
        }
        let line_start = src
            .get(..load.span.start)
            .and_then(|before| before.rfind('\n'))
            .map_or(0, |newline| newline + 1);
        let indent = src.get(line_start..load.span.start).unwrap_or_default();
        let written = lines::written(body, indent, !prefix.is_empty(), line_break(src, load.span))?;
        splice(src, load.span, &written)
    }

    /// [`lines::unescape`]: the body as just runs it
    /// (`languages/api/src/lens:V39`).
    fn unescape(&self, delim: &Delim, raw: &str) -> Result<String> {
        lines::unescape(delim, raw)
    }

    /// [`lines::escape`]: the inverse, for a body `unescape` could give.
    fn escape(&self, delim: &Delim, body: &str) -> Result<String> {
        lines::escape(delim, body)
    }

    /// `just --fmt --check --unstable --justfile <file>`: the check the
    /// api names for just hosts (`languages/api` §I).
    fn checks(&self) -> Vec<LintCmd> {
        vec![LintCmd {
            argv: ["just", "--fmt", "--check", "--unstable", "--justfile"]
                .map(str::to_owned)
                .to_vec(),
            file_arg: FileArg::Append,
            format: Format::Raw,
        }]
    }

    /// `just --fmt --unstable --justfile <file>`, the check's fixer.
    fn fixers(&self) -> Vec<LintCmd> {
        vec![LintCmd {
            argv: ["just", "--fmt", "--unstable", "--justfile"]
                .map(str::to_owned)
                .to_vec(),
            file_arg: FileArg::Append,
            format: Format::Raw,
        }]
    }

    /// A shebang recipe's guest is what its `#!` line names.
    fn guest_by_shebang(&self, _src: &str, site: &Site) -> bool {
        site.delim.kind == DelimKind::JustShebangRecipe
    }

    /// `{host_dir}/scripts/just/<recipe>` (`languages/ci/just` §I).
    fn placement(&self, site: &Site) -> Result<Placement> {
        Ok(placement::placement(site))
    }
}
