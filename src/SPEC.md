# SPEC

## §G GOAL

root crate: lib + `xnl` bin: core model, CLI, verbs; cross-language engines extract/graph/lint/config as child nodes.

## §F FEDERATION

dir|owns|⊥owns|tokens
config|`xenolith.toml` parse & validation|applying config — each verb's node|-
extract|embed → own file, host rewrite, diff \| `--write`|detection (`languages`), load resolution (`graph`)|-
graph|host → extract load edges, dangling & orphan|writing files (`extract`)|-
lint|per-language linter map & invocation|deciding what is an extract (`graph`, `config`)|-
cli|verbs, flags, exit codes, rule ids, output formats, hk wiring|scanning & engines (`src/*`), config schema (`src/config`)|-
check|check engine: candidates → claims → sites → guests → violations; unclaimed & missing-guest policy, parallel scan|discovery (`src/discover`), rendering & exit codes (`src/cli`), config schema (`src/config`)|-
discover|candidate discovery: `git ls-files` \| named paths, normalisation, symlink screen|what is done w/ a candidate (`src/check`)|-
registry|language registry: `hosts()`, `guests()`, feature gates, feature names|language specifics (`languages`)|-

## §N NAV

rel|path|lens
up|.|-
self|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
sib|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
sib|scripts|∀ shell in repo: dev shell hook, guardrail scripts; gate config `hk.pkl`, vendored hk schema `pkl/Config.pkl`, `.github/workflows/`, `.github/zizmor.yml`
sib|tests|fixture format & runner, integration fixtures, bats mirroring `scripts/` & `.github/scripts/`
sib|nix|flake inputs, packaging, devShell, cachix, subset override, closure
sib|docs|public project docs & notices

## §C CONSTRAINTS

- C1: Rust cargo workspace; repo & root crate = `xenolith` (lib + bin `xnl` = `xenolith` consonant skeleton, `rg` shape; `xnl` free as crate, nixpkgs & brew binary — checked 2026-09-19); lib exposed for consumers. root crate depends on ∀ language crate, `default` = ∀ `lang-*` ∴ `xnl` works out of box, trim via `default-features = false`; feature = compile-time (grammar ∉ binary), `xenolith.toml` `[langs]` = runtime toggle over compiled-in set. ∀ workspace crate (api, ∀ `xenolith-lang-*`, root) published to crates.io, lockstep version. MIT.
- C2: edition `2024`, `rust-version = "1.95"` ≡ rustc in pinned nixpkgs. ⊥ rust-overlay, ⊥ fenix, ⊥ `rust-toolchain.toml`.
- C5: deps minimal & justified per crate in `Cargo.toml` comment. `cargo-deny` gate (licenses, advisories, duplicates). `clippy` `unwrap_used`/`expect_used` = deny.
- C139: 1-to-1 Rust unit tests ∀ tracked `.rs` w/ logic (∃ fn body ∴ pure-wiring `lib.rs` exempt; ∀ crate, as C5; twin of `scripts:C13`): `#[cfg(test)] mod tests;` → `<d>/<m>.rs` & `<d>/<m>/mod.rs` ↔ `<d>/<m>/tests.rs`, `<d>/lib.rs` ↔ `<d>/tests.rs`; inline `mod tests {}` ⊥ counts. `main.rs` ⊥ own mirror (= lib's) ∴ ! shim, `fn main` → lib only. `tests/` integration allowed, ⊥ satisfies. closed exemption list & checker: `scripts/guard:V140`.

## §I INTERFACES

- lib: `xenolith::check(&Path, &Config, &check::Options) -> Result<Report, CheckError>` (`Config` = root's; nested `xenolith.toml` read by the engine, per file `src/config` §I; `Options` = paths; `Report` = sorted violations + warnings; `CheckError` = the exit-2 refusals: discovery, nested config, missing guest), `xenolith::extract(...) -> Edit`, `xenolith::graph(...) -> Graph`.

## §V INVARIANTS

V1: ∀ violation carries `rule`, `file:line:col`, host lang, guest lang, sink, site delimiter kind, `why`, ≥1 direction. ⊥ bare "bad".
V11: deterministic: output order sorted (file, line, col); json byte-stable across runs & platforms.
V12: CPU only, offline: test runs w/ network disabled; ⊥ `reqwest`/`ureq`/`hyper` in dep tree (cargo-deny ban).
V18: `rust-version` ≡ pinned rustc minor; CI asserts.
V30: ∀ `lang-*` feature toggleable: build + test green w/ each feature alone & w/ none (`cargo hack --each-feature`). language compiled out → its files unclaimed per `src/check:V13`; strict → exit 2 message names missing feature `lang-<lang>`. ⊥ `cfg` leak: engine code ⊥ names a language outside its feature gate.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end | T3, T8, T40, T142 | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |

id|status|task|cites
T3|x|`Cargo.toml` (edition 2024, rust-version 1.95, MIT, lints), `clippy.toml`, `rustfmt.toml`, `deny.toml` w/ network crate bans|C2,C5,V12,V18
T8|x|core model: `Violation`, `Direction` over api `Site`/`LangId`; json schema v1; sorted output|V1,V11,`src/cli:V24`,`languages/api:T42`
T40|x|feature matrix: `lang-*` features in root `Cargo.toml`, `cargo-hack` in devShell, hk pre-push + CI `cargo hack --each-feature test`|V30,C1
T142|x|C139 backfill: `src/model/tests.rs`|C139,`scripts/guard:V140`

## §B BUGS

id|date|cause|fix
