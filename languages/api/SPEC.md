# SPEC

## §G GOAL

crate `xenolith-lang-api`: contract ∀ language crate — `Host` & `Guest` traits, shared types, lens law harness. language plays host (file holding embed), guest (embedded code), or both; extract & inline = one lens, 2 directions.

## §F FEDERATION

dir|owns|⊥owns|tokens
src|api modules: site, lens, holes|traits & LangId (this node)|-

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
sib|languages/yaml|yaml parser, GH Actions sinks, placement
sib|languages/dockerfile|Dockerfile parser, `RUN` sinks, placement
sib|languages/shebang|shebang parse/strip/wrap ∀ guest
sib|languages/rust|rust parser, rust host sinks
sib|languages/ruby|ruby parser, ruby host sinks
sib|languages/html|html parser, inline script/style sinks
sib|languages/js|javascript grammar, guest rules
sib|languages/css|css grammar, guest rules

## §I INTERFACES

- trait `Host`: `id() -> LangId`; `sites(src: &str) -> Result<Vec<Site>>` (sinks holding guest code); `loads(src) -> Result<Vec<LoadRef>>` (for `src/graph`); `rewrite(src, &Site, &Invoke, path) -> Result<String>` (extract direction: body out, load in); `inline(src, &LoadRef, body) -> Result<String>` (inverse direction).
- trait `Guest`: `id() -> LangId`; `extension() -> &'static str`; `invoke(path) -> Invoke` (how to run a file of me: `bash x.sh`, `jq -f x.jq`); `trivial(body) -> Result<bool>` (may stay inline — shell = single simple command, `languages/shell:V3`); `checks(&GuestEnv) -> Vec<LintCmd>`, `fixers(&GuestEnv) -> Vec<LintCmd>`.
- crate impl: host-only | guest-only | both. guest-only language (python, sql, jq, …) ⊥ needs host grammar.
- type `LangId`: closed enum ∀ language in root host × sink matrix + guest-only (python, sql, js, css, perl, awk, jq, ruby); ⊥ feature-gated.
- type `LoadRef { span, path, guest: LangId }`, `Invoke { argv }`, `LintCmd { argv, file_arg, format: Json(parser) | Sarif | Raw }`, `Error`.
- `Guest::prelude(&GuestEnv) -> Prelude` & `Guest::executable() -> bool`: default content & mode of extract file, overridable by config. type `Prelude { shebang: Option<Shebang>, strict: Option<&'static str> }` (strict: bash `set -euo pipefail`; ⊥ for python/sql/jq/awk) — one value per guest, consumed by mod `shebang`.
- `Host::checks() -> Vec<LintCmd>` & `Host::fixers()`: checks for host files themselves (nix `statix`, `deadnix`, `nixfmt --check`; GH `actionlint`, `zizmor`; Dockerfile `hadolint`; just `just --fmt --check --unstable`; pkl `pkl format --diff` ?).

## §V INVARIANTS

V32: dependency shape: `xenolith-lang-api` deps ⊆ {`xenolith-shebang`}, ⊥ grammar dep, ⊥ feature; language crate deps ⊆ {`xenolith-lang-api`, own grammar, std-ish}; ⊥ root crate, ⊥ other language crate. checked from `cargo metadata`, ⊥ by review.
V33: `LangId` closed & ungated: ∃ variant ∀ known language regardless of enabled features ∴ host names guest compiled out; adding language = add variant here first.
V35: load idiom split: guest owns `invoke` (how to run file of me); host owns wrapping `Invoke` in own syntax (`builtins.readFile`, hk step, `run:`). ⊥ host hardcodes guest command; ⊥ guest knows host syntax.
V36: trait fns pure: ⊥ fs, ⊥ process, ⊥ env, ⊥ clock; `&str` in, values out ∴ engines own IO (`--write`, lint runs) & C3 determinism holds per crate. `Vec` outputs sorted by span.
V37: missing capability = missing impl, ⊥ default method returning empty. ⊥ silent skip (`src/lint:V8` spirit).

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end | T42, T44 | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |

id|status|task|cites
T42|.|scaffold `languages/api` crate: `LangId`, `Site`, `LoadRef`, `Invoke`, `LintCmd`, `Error`, `Host`, `Guest`; workspace member, ⊥ features|V33,V36,V37,C24
T44|.|port per-language tasks onto traits: shell `Host`+`Guest` (`languages/shell:T11`, `languages/shell:T15`), nix `Host` (`languages/nix:T12`), pkl `Host` (`languages/pkl:T13`); each crate runs `laws::check`|`languages/api/src/lens:V34`,V35

## §B BUGS

id|date|cause|fix
