# SPEC

## §G GOAL

crate `xenolith-lang-api`: contract ∀ language crate — `Host` & `Guest` traits, shared types, lens law harness. language plays host (file holding embed), guest (embedded code), or both; extract & inline = one lens, 2 directions.

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
self|languages/api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness
sib|languages/nix|nix parser, sinks, load idiom
sib|languages/pkl|pkl parser, hk step sinks, load idiom
sib|languages/shell|bash parser & host sinks, single-command classifier, shell linters
sib|languages/just|just parser, recipe sinks, load idiom
sib|languages/python|python grammar, guest rules
sib|languages/sql|sql grammar, guest rules
sib|languages/jq|jq grammar, guest rules
sib|languages/awk|awk grammar, guest rules

## §I INTERFACES

- trait `Host`: `id() -> LangId`; `sites(src: &str) -> Result<Vec<Site>>` (sinks holding guest code); `loads(src) -> Result<Vec<LoadRef>>` (for `src/graph`); `rewrite(src, &Site, &Invoke, path) -> Result<String>` (extract direction: body out, load in); `inline(src, &LoadRef, body) -> Result<String>` (inverse direction).
- trait `Guest`: `id() -> LangId`; `extension() -> &'static str`; `invoke(path) -> Invoke` (how to run a file of me: `bash x.sh`, `jq -f x.jq`); `trivial(body) -> Result<bool>` (may stay inline — shell = single simple command, `languages/shell:V3`); `linter() -> Option<LintCmd>`.
- fn `laws::check::<H: Host>(fixtures: &Path)` → panics w/ fixture path & broken law; called from each language crate's `cargo test`.
- crate impl: host-only | guest-only | both. guest-only language (python, sql, jq, …) ⊥ needs host grammar.
- type `LangId`: closed enum ∀ language in root host × sink matrix + guest-only (python, sql, js, css, perl, awk, jq, ruby); ⊥ feature-gated.
- type `Site { sink, guest: LangId, delim: Delim, holes: Vec<Span> }`. `Delim { kind, open: Span, body: Span, close: Span }` = the braces bounding foreign code, taken from AST string|block node, ⊥ scanned. `DelimKind` ∈ nix `''…''` \| `"…"`, heredoc `<<[-~]TAG` (quoted \| not), pkl `"""…"""` / `#"""…"""#`, yaml block `\|`/`>` + chomp, html `<script>`/`<style>` element, rust `r#"…"#`, ruby `<<~TAG`, argv string (`-c`/`-e`). `holes` = host interpolations inside body (nix `${…}`, pkl `\(…)`, shell `$VAR` in unquoted heredoc, yaml `${{ }}`).
- `Host::unescape(&Delim, raw) -> String` (strip common indent, host escapes like nix `''$`) & `Host::escape` inverse; `rewrite`/`inline` go through them.
- type `LoadRef { span, path, guest: LangId }`, `Invoke { argv }`, `LintCmd { argv, file_arg }`, `Error`.
- `Host::placement(&Site) -> Placement { name, dir }`: host's default extract name (from site syntax: nix attr path, hk step name, GH job/step id) & dir (pkl hk step → `scripts/hk`); layer D of extract resolution, lowest precedence.
- `Guest::prelude() -> Prelude` & `Guest::executable() -> bool`: default content & mode of extract file, overridable by config. type `Prelude { shebang: Option<Shebang>, strict: Option<&'static str> }` (strict: bash `set -euo pipefail`; ⊥ for python/sql/jq/awk) — one value per guest, consumed by mod `shebang`.
- `Host::hole_advice(&Site) -> Vec<String>`: host's proposed strategies for holes (nix `replaceVars`, pass as arg, env var); root wraps each as `Judgment` direction (V40).
- `Host::claims(path, head: &str) -> bool`: host claims file by filename, extension, path glob or shebang in `head` (first line); ∀ file is a candidate host.
- mod `shebang`: Rust equivalent of `github:pr0d1r2/nix-shebang` generalised ∀ guest — `has`, `get`, `strip`, `strip_strict(text, &Prelude)`, `parse -> Shebang { interpreter, args, is_env, resolved_interpreter }`, `resolves_to(LangId)`; bash/sh semantics & vectors shared w/ nix-shebang, per-language vectors ∀ other guest (`python3`, `awk -f`, `env -S jq -f`).
- `shebang::wrap(body, &Prelude) -> String`: extract file content = prelude + body; inverse of `strip_strict`.
- `Host::runtime_base(&Site) -> Base` ∈ `HostDir` (default) \| `RepoRoot` \| `Dir(path)`: directory the host's runtime resolves load paths from.
- `Guest::param(n) -> Option<String>`: guest's reference to n-th positional arg (shell `"$1"`, python `sys.argv[1]`); ⊥ → holes of that guest stay `Judgment`.

## §V INVARIANTS

V32: dependency shape: `xenolith-lang-api` ⊥ grammar dep, ⊥ feature; language crate deps ⊆ {`xenolith-lang-api`, own grammar, std-ish}; ⊥ root crate, ⊥ other language crate. checked from `cargo metadata`, ⊥ by review.
V33: `LangId` closed & ungated: ∃ variant ∀ known language regardless of enabled features ∴ host names guest compiled out; adding language = add variant here first.
V34: lens laws ∀ host, ∀ site `s` of fixture `x`, `y = rewrite(x, s, guest.invoke(p), p)`: (a) `inline(y, load, unescape(s.delim, s.delim.body))` ≡ `x` normalized whitespace (`src/extract:V4`); (b) `sites(y)` ∌ `s`; (c) `loads(y)` ∋ load of `p`; (d) `rewrite` on host w/ ⊥ sites = identity (`src/extract:V5`). enforced by `laws::check` ∀ language crate, ⊥ per-crate hand tests.
V35: load idiom split: guest owns `invoke` (how to run file of me); host owns wrapping `Invoke` in own syntax (`builtins.readFile`, hk step, `run:`). ⊥ host hardcodes guest command; ⊥ guest knows host syntax.
V36: trait fns pure: ⊥ fs, ⊥ process, ⊥ env, ⊥ clock; `&str` in, values out ∴ engines own IO (`--write`, lint runs) & C3 determinism holds per crate. `Vec` outputs sorted by span.
V37: missing capability = missing impl, ⊥ default method returning empty. ⊥ silent skip (`src/lint:V8` spirit).
V38: site = delimiter ∧ sink context. delimiter alone (same `''…''` under nix `description`) = inert data ⊥ site (`languages:V2` negative fixture). delimiter bounds from grammar node ∴ escapes, nesting, heredoc terminators, indent rules resolved by parser, ⊥ brace counting over raw bytes.
V39: body text for guest = `unescape(delim, raw)`; law V34(a) holds through `unescape`/`escape` round-trip; ∀ `DelimKind` ∃ fixture w/ indent + escape cases.
V40: holes → params: each distinct hole → `Guest::param(n)` in extract (same hole ⇒ same n); load = one-liner `<invoke> <hole₁> … <holeₙ>` in host syntax (holes stay host interpolations). MECHANICAL iff n ≤ `[threshold.load] max_params` & ∀ hole in expanding context (⊥ single-quoted, ⊥ quoted heredoc, ⊥ inside guest string literal) & `param` ≠ ⊥ & load one-liner trivial (`languages/shell:V3`); else `Judgment` w/ `hole_advice` & `rewrite` refuses (exit 2). ⊥ copying `${…}` into guest file verbatim.
V43: `placement` name deterministic & semantic: derived from site syntax (attr path, step name, job id), kebab-case, ⊥ line numbers, ⊥ random | hash-only names; no semantic name → `<host_stem>-<sink>`.
V63: extract file = header + strict + body; inline from disk = `shebang::strip_strict(file, &guest.prelude())` ∴ V34(a) holds over file ON DISK, ⊥ only in-memory body. ∀ guest property: `strip_strict(wrap(body, p), p) == body`; vectors shared w/ nix-shebang.
V66: load path in `rewrite` & `LoadRef.path` relative to site's runtime base — default HOST FILE dir (`./sub/x.sh`), else `Host::runtime_base` | rule `base` (`src/extract:V45`); ⊥ cwd relative. placement paths (`src/extract:V46`) stay repo-root relative; engine converts.

## §T TASKS

id|status|task|cites
T42|.|scaffold `languages/api` crate: `LangId`, `Site`, `LoadRef`, `Invoke`, `LintCmd`, `Error`, `Host`, `Guest`; workspace member, ⊥ features|V33,V36,V37,C1
T43|.|`laws::check` harness + fixture loader over calling crate's `tests/fixtures/<case>/`; RED on toy host in api tests|V34,`tests:V14`
T44|.|port per-language tasks onto traits: shell `Host`+`Guest` (`languages/shell:T11`, `languages/shell:T15`), nix `Host` (`languages/nix:T12`), pkl `Host` (`languages/pkl:T13`); each crate runs `laws::check`|V34,V35
T45|.|`Delim`/`DelimKind`/holes + `unescape`/`escape` round-trip property in harness; fixtures ∀ kind incl. indent, escapes, holes|V38,V39,V40,`tests:V15`
T48|.|`Placement`, `Host::placement`, `Host::hole_advice`, `Guest::prelude`, `Guest::executable` in api crate|V43,`languages/api:T42`
T63|.|`shebang` module port + shared vectors w/ nix-shebang; law harness inlines from disk|V63,V34
T76|.|holes → params rewrite; fixtures: `${pkgs.foo}` ×2 → one param, hole in single quotes → `Judgment`, 7 holes → `Judgment`|V40

## §B BUGS

id|date|cause|fix
