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

- lib: `xenolith::check(&Path, &Config) -> Vec<Violation>`, `xenolith::extract(...) -> Edit`, `xenolith::graph(...) -> Graph`.

## §V INVARIANTS

V1: ∀ violation carries `rule`, `file:line:col`, host lang, guest lang, sink, site delimiter kind, `why`, ≥1 direction. ⊥ bare "bad".
V11: deterministic: output order sorted (file, line, col); json byte-stable across runs & platforms.
V12: CPU only, offline: test runs w/ network disabled; ⊥ `reqwest`/`ureq`/`hyper` in dep tree (cargo-deny ban).
V13: unclaimed file (0 hosts claim, named or not) ⊥ scanned & ⊥ reported by default; `--strict-hosts` \| `[langs] unclaimed = "error"` → exit 2 "host unsupported"; `warn` → warning.
V18: `rust-version` ≡ pinned rustc minor; CI asserts.
V30: ∀ `lang-*` feature toggleable: build + test green w/ each feature alone & w/ none (`cargo hack --each-feature`). language compiled out → its files unclaimed per `src:V13`; strict → exit 2 message names missing feature `lang-<lang>`. ⊥ `cfg` leak: engine code ⊥ names a language outside its feature gate.
V41: registry = ONE file in root crate: `hosts() -> &'static [&'static dyn Host]`, `guests() -> &'static [&'static dyn Guest]`, each entry behind `#[cfg(feature = "lang-<lang>")]`, sorted by `LangId`. engines iterate registry; ⊥ `cfg(feature = "lang-*")` elsewhere (V30 no-leak made checkable).
V42: guest compiled out: site whose `guest` ∉ `guests()` → per `[langs] missing_guest` (default `error`: exit 2 naming feature `lang-<guest>`; `warn` → warning; `ignore`); ⊥ guessing trivial/non-trivial.
V57: `xnl check|graph|lint` w/ ⊥ paths → candidates = `git ls-files` (tracked only ∴ `.gitignore` honoured); ⊥ git repo & ⊥ paths → exit 2 usage.
V95: scan parallel per file, results merged then sorted (V11) ∴ output byte-identical to serial run; `--jobs N` (default cores).
V120: scan throughput recorded (files/s over the M2 corpus & a synthetic large tree); regression beyond recorded budget = warning `slow-scan`, ⊥ gate failure (timing noise); budget raise only w/ Why.
V128: candidate that IS a symlink ⊥ scanned (warning `symlink-skipped`), & discovery ⊥ follows symlinked dirs — a tracked symlink may point outside the repo, & the same bytes would be reported twice under 2 paths. named explicitly → exit 2 saying so (`src/extract:V71`, `src/graph:V72` are the write & graph halves).
V152: `xenolith::check` = ONE engine: candidates (V57, V128, `src/config:V79`) → registry hosts claim (V41, V13) → `Host::sites` → guest ∈ `guests()` (V42) → `Guest::trivial` relaxed by `[threshold]` (`src/config:V55`) → non-trivial ∧ ⊥ `[[allow]]` → `Violation` (V1); unmatched allow → `stale-allow` (`src/config:V9`). `src/cli` renders & maps exit codes only.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end | T3, T8, T40, T46, T58, T75, T88, T127, T142, T153 | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |
| M3 | publication | T98, T119 | public doc set, release machinery, history audit green, crates published (`.:T32`) |

id|status|task|cites
T3|x|`Cargo.toml` (edition 2024, rust-version 1.95, MIT, lints), `clippy.toml`, `rustfmt.toml`, `deny.toml` w/ network crate bans|C2,C5,V12,V18
T8|x|core model: `Violation`, `Direction` over api `Site`/`LangId`; json schema v1; sorted output|V1,V11,`src/cli:V24`,`languages/api:T42`
T40|x|feature matrix: `lang-*` features in root `Cargo.toml`, `cargo-hack` in devShell, hk pre-push + CI `cargo hack --each-feature test`|V30,C1
T46|.|registry file + compiled-out guest exit 2; test builds w/ `lang-nix` only & asserts nix→shell site exits 2 naming `lang-shell`|V41,V42,V30
T58|x|candidate discovery via `git ls-files`; fixture: untracked & ignored files ⊥ scanned; outside git → exit 2|V57
T75|.|unclaimed handling: default ignore, `--strict-hosts` & config error/warn; fixture: hk-style file list w/ `.png`, `.md`|V13
T88|.|`missing_guest` error/warn/ignore; fixture on `lang-nix`-only build|V42
T98|.|parallel scan + determinism test (serial vs `--jobs 8` byte-equal)|V95,V11
T119|.|benchmark harness + recorded budget file|V120,V95
T127|x|symlink handling in discovery + fixtures: symlinked file, symlinked dir, explicit symlink path|V128,V57
T142|x|C139 backfill: `src/model/tests.rs`|C139,`scripts/guard:V140`
T153|.|`xenolith::check` per V152; fixtures: nix `&&` script flagged, single command clean, allowed clean, stale allow flagged, pkl hk step flagged|V152,T46,T58

## §B BUGS

id|date|cause|fix
B1|2026-09-26|`Report::push` keyed (file,line,col,rule), `warn` (code,file): ties w/ differing rendered fields kept arrival order ∴ parallel scan (V95) → bytes vary run to run|key extended to ∀ rendered field (`report_order`, + message); V11 ⊇ ties
