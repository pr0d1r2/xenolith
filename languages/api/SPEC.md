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
- type `Site { sink, guest: LangId, delim: Delim }`. `Delim { kind, open: Span, body: Span, close: Span }` = the braces bounding foreign code, taken from AST string|block node, ⊥ scanned. `DelimKind` ∈ nix `''…''` \| `"…"`, heredoc `<<[-~]TAG` (quoted \| not), pkl `"""…"""` / `#"""…"""#`, yaml block `\|`/`>` + chomp, html `<script>`/`<style>` element, rust `r#"…"#`, ruby `<<~TAG`, argv string (`-c`/`-e`).
- `Host::unescape(&Delim, raw) -> String` (strip common indent, host escapes like nix `''$`) & `Host::escape` inverse; `rewrite`/`inline` go through them.
- type `LoadRef { span, path, guest: LangId }`, `Invoke { argv }`, `LintCmd { argv, file_arg }`, `Error`.

## §V INVARIANTS


## §T TASKS

id|status|task|cites

## §B BUGS

id|date|cause|fix
