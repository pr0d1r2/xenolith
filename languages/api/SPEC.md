# SPEC

## §G GOAL

crate `lydite-lang-api`: contract ∀ language crate — `Host` & `Guest` traits, shared types, lens law harness. language plays host (file holding embed), guest (embedded code), or both; extract & inline = one lens, 2 directions.

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
self|languages/api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness
sib|languages/nix|nix parser, sinks, load idiom
sib|languages/pkl|pkl parser, hk step sinks, load idiom
sib|languages/shell|bash parser & host sinks, single-command classifier, shell linters

## §I INTERFACES

- trait `Host`: `id() -> LangId`; `sites(src: &str) -> Result<Vec<Site>>` (sinks holding guest code); `loads(src) -> Result<Vec<LoadRef>>` (for `src/graph`); `rewrite(src, &Site, &Invoke, path) -> Result<String>` (extract direction: body out, load in); `inline(src, &LoadRef, body) -> Result<String>` (inverse direction).
- trait `Guest`: `id() -> LangId`; `extension() -> &'static str`; `invoke(path) -> Invoke` (how to run a file of me: `bash x.sh`, `jq -f x.jq`); `trivial(body) -> Result<bool>` (may stay inline — shell = single simple command, `languages/shell:V3`); `linter() -> Option<LintCmd>`.
- fn `laws::check::<H: Host>(fixtures: &Path)` → panics w/ fixture path & broken law; called from each language crate's `cargo test`.
- crate impl: host-only | guest-only | both. guest-only language (python, sql, jq, …) ⊥ needs host grammar.
- type `LangId`: closed enum ∀ language in root host × sink matrix + guest-only (python, sql, js, css, perl, awk, jq, ruby); ⊥ feature-gated.
- type `Site { sink, guest: LangId, delim: Delim, holes: Vec<Span> }`. `Delim { kind, open: Span, body: Span, close: Span }` = the braces bounding foreign code, taken from AST string|block node, ⊥ scanned. `DelimKind` ∈ nix `''…''` \| `"…"`, heredoc `<<[-~]TAG` (quoted \| not), pkl `"""…"""` / `#"""…"""#`, yaml block `\|`/`>` + chomp, html `<script>`/`<style>` element, rust `r#"…"#`, ruby `<<~TAG`, argv string (`-c`/`-e`). `holes` = host interpolations inside body (nix `${…}`, pkl `\(…)`, shell `$VAR` in unquoted heredoc, yaml `${{ }}`).
- `Host::unescape(&Delim, raw) -> String` (strip common indent, host escapes like nix `''$`) & `Host::escape` inverse; `rewrite`/`inline` go through them.
- type `LoadRef { span, path, guest: LangId }`, `Invoke { argv }`, `LintCmd { argv, file_arg }`, `Error`.

## §V INVARIANTS

V32: dependency shape: `lydite-lang-api` ⊥ grammar dep, ⊥ feature; language crate deps ⊆ {`lydite-lang-api`, own grammar, std-ish}; ⊥ root crate, ⊥ other language crate. checked from `cargo metadata`, ⊥ by review.
V33: `LangId` closed & ungated: ∃ variant ∀ known language regardless of enabled features ∴ host names guest compiled out; adding language = add variant here first.
V34: lens laws ∀ host, ∀ site `s` of fixture `x`, `y = rewrite(x, s, guest.invoke(p), p)`: (a) `inline(y, load, unescape(s.delim, s.delim.body))` ≡ `x` normalized whitespace (`src/extract:V4`); (b) `sites(y)` ∌ `s`; (c) `loads(y)` ∋ load of `p`; (d) `rewrite` on host w/ ⊥ sites = identity (`src/extract:V5`). enforced by `laws::check` ∀ language crate, ⊥ per-crate hand tests.
V35: load idiom split: guest owns `invoke` (how to run file of me); host owns wrapping `Invoke` in own syntax (`builtins.readFile`, hk step, `run:`). ⊥ host hardcodes guest command; ⊥ guest knows host syntax.
V36: trait fns pure: ⊥ fs, ⊥ process, ⊥ env, ⊥ clock; `&str` in, values out ∴ engines own IO (`--write`, lint runs) & C3 determinism holds per crate. `Vec` outputs sorted by span.
V37: missing capability = missing impl, ⊥ default method returning empty. ⊥ silent skip (`src/lint:V8` spirit).
V38: site = delimiter ∧ sink context. delimiter alone (same `''…''` under nix `description`) = inert data ⊥ site (`languages:V2` negative fixture). delimiter bounds from grammar node ∴ escapes, nesting, heredoc terminators, indent rules resolved by parser, ⊥ brace counting over raw bytes.
V39: body text for guest = `unescape(delim, raw)`; law V34(a) holds through `unescape`/`escape` round-trip; ∀ `DelimKind` ∃ fixture w/ indent + escape cases.
V40: holes ⊥ silently extracted: site w/ ≥1 hole → violation carries `Judgment` direction (pass value as arg | env | `substituteAll`-style template), `rewrite` refuses (exit 2) unless hole-free. ⊥ copying `${…}` into guest file verbatim (would change meaning).

## §T TASKS

id|status|task|cites
T42|.|scaffold `languages/api` crate: `LangId`, `Site`, `LoadRef`, `Invoke`, `LintCmd`, `Error`, `Host`, `Guest`; workspace member, ⊥ features|V33,V36,V37,C1
T43|.|`laws::check` harness + fixture loader over `tests/fixtures/<host>/<case>/`; RED on toy host in api tests|V34,`tests:V14`

## §B BUGS

id|date|cause|fix
