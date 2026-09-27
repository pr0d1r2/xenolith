# SPEC

## §G GOAL

crate `xenolith-lang-api`: contract ∀ language crate — `Host` & `Guest` traits, shared types, lens law harness. language plays host (file holding embed), guest (embedded code), or both; extract & inline = one lens, 2 directions.

## §F FEDERATION

dir|owns|⊥owns|tokens
src|api modules: site, lens, holes; hub root `lib.rs` & `shebang.rs` re-export|traits & LangId (this node)|-

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
self|languages/api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness
sib|languages/shebang|shebang parse/strip/wrap ∀ guest
sib|languages/shells|hub: shell family -- shell, bats
sib|languages/ci|hub: build, CI & config hosts -- nix, pkl, just, yaml, dockerfile
sib|languages/data|hub: data & text guests -- python, sql, jq, awk, perl
sib|languages/web|hub: web host & its guests -- html, js, css
sib|languages/rust|rust parser, rust host sinks
sib|languages/ruby|ruby parser, ruby host sinks

## §I INTERFACES

- trait `Host`: `id() -> LangId`; `sites(src: &str) -> Result<Vec<Site>>` (sinks holding guest code); `loads(src) -> Result<Vec<LoadRef>>` (for `src/graph`); `rewrite(src, &Site, &Invoke, path) -> Result<String>` (extract direction: body out, load in); `inline(src, &LoadRef, body) -> Result<String>` (inverse direction); `unescape(&Delim, raw) -> Result<String>` (body as the guest reads it, `languages/api/src/lens:V39`).
- trait `Guest`: `id() -> LangId`; `extension(&GuestEnv) -> &'static str` (dialect decides: zsh → `zsh`, `languages/shells/shell:V51`); `invoke(path) -> Invoke` (how to run a file of me: `bash x.sh`, `jq -f x.jq`); `trivial(body) -> Result<bool>` (may stay inline — shell = single simple command, `languages/shells/shell:V3`); `constructs(body) -> Result<Vec<&'static str>>` (sorted, empty iff `trivial`; `[threshold.<guest>] allow` names; default `Unsupported` (V37) → `max_lines`/`max_bytes`); `unsupported(body, &GuestEnv) -> Option<&'static str>` (dialect syntax the guest cannot judge → that reason, a Judgment, `languages/shells/shell:V138`; default `None`); `checks(&GuestEnv) -> Vec<LintCmd>`, `fixers(&GuestEnv) -> Vec<LintCmd>`.
- `Host::guest_by_shebang(src, &Site) -> bool`: site's guest named by a shebang in its body (`languages/ci/nix:T157`), ⊥ by its sink (`src/check:V42`); default `false` = host never reads one (a fact, ⊥ missing capability, V37).
- crate impl: host-only | guest-only | both. guest-only language (python, sql, jq, …) ⊥ needs host grammar.
- type `LangId`: closed enum ∀ language in root host × sink matrix + guest-only (python, sql, js, css, perl, awk, jq, ruby); ⊥ feature-gated.
- type `LoadRef { span, path, guest: LangId }`, `Invoke { argv }`, `LintCmd { argv, file_arg, format: Json(parser) | Sarif | Raw }`, `Error`.
- `Guest::prelude(&GuestEnv) -> Prelude` & `Guest::executable() -> bool`: default content & mode of extract file, overridable by config. type `Prelude { shebang: Option<Shebang>, strict: Option<String> }` (owned: `languages/shells/shell:V82` wants the site's exact `set -o` state, ⊥ a fixed literal set) (strict: bash `set -euo pipefail`; ⊥ for python/sql/jq/awk) — one value per guest, consumed by mod `shebang`.
- `Host::checks() -> Vec<LintCmd>` & `Host::fixers()`: checks for host files themselves (nix `statix`, `deadnix`, `nixfmt --check`; GH `actionlint`, `zizmor`; Dockerfile `hadolint`; just `just --fmt --check --unstable`; pkl `pkl format --diff` ?).
- kinship, beside `LangId` & ungated like it (V33): `base_of(LangId) -> Option<LangId>` (bats → shell) & `lookalikes(LangId) -> &'static [LangId]` (shell ~ awk, perl, jq) ∴ relation stated even when neither crate compiled in (`languages:V130`, `languages:V131`).
- `Host::claims` ⊥ true ∀ file of a language based-on it (`.bats` ⊥ claimed by shell): parent grammar either MISREADS child syntax confidently or errors on it, & both are wrong answers (`languages:V130`).

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
| M2 | kinship & test hosts | T131 | parent ⊥ claims a based-on child, ∀ lookalike edge symmetric (`languages:V130`, `languages:V131`) |

id|status|task|cites
T42|x|scaffold `languages/api` crate: `LangId`, `Site`, `LoadRef`, `Invoke`, `LintCmd`, `Error`, `Host`, `Guest`; workspace member, ⊥ features|V33,V36,V37,C24
T44|.|port per-language tasks onto traits: shell `Host`+`Guest` (`languages/shells/shell:T11`, `languages/shells/shell:T15`), nix `Host` (`languages/ci/nix:T12`), pkl `Host` (`languages/ci/pkl:T13`); each crate runs `laws::check`|`languages/api/src/lens:V34`,V35
T131|.|kinship table (`base_of`, `lookalikes`) + `claims` refusal ∀ based-on child; test: ∀ base pair parent ⊥ claims child's ext, ∀ lookalike pair both directions present|V33,`languages:V130`,`languages:V131`

## §B BUGS

id|date|cause|fix
