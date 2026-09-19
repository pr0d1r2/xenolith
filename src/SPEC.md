# SPEC

## §G GOAL

root crate: lib + `xnl` bin: core model, CLI, verbs; cross-language engines extract/graph/lint/config as child nodes.

## §F FEDERATION

dir|owns|⊥owns|tokens
config|`xenolith.toml` parse & validation|applying config — each verb's node|-
extract|embed → own file, host rewrite, diff \| `--write`|detection (`languages`), load resolution (`graph`)|-
graph|host → extract load edges, dangling & orphan|writing files (`extract`)|-
lint|per-language linter map & invocation|deciding what is an extract (`graph`, `config`)|-

## §N NAV

rel|path|lens
up|.|-
self|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
sib|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
sib|scripts|∀ shell in repo: dev shell hook, guardrail scripts
sib|tests|fixture format & runner, integration fixtures, bats mirroring `scripts/` & `.github/scripts/`
sib|nix|flake inputs, packaging, devShell, cachix, subset override, closure

## §C CONSTRAINTS

- C1: Rust cargo workspace; repo & root crate = `xenolith` (lib + bin `xnl` = `xenolith` consonant skeleton, `rg` shape; `xnl` free as crate, nixpkgs & brew binary — checked 2026-09-19); lib exposed for consumers. root crate depends on ∀ language crate, `default` = ∀ `lang-*` ∴ `xnl` works out of box, trim via `default-features = false`; feature = compile-time (grammar ∉ binary), `xenolith.toml` `[langs]` = runtime toggle over compiled-in set. ∀ workspace crate (api, ∀ `xenolith-lang-*`, root) published to crates.io, lockstep version. MIT.
- C2: edition `2024`, `rust-version = "1.95"` ≡ rustc in pinned nixpkgs. ⊥ rust-overlay, ⊥ fenix, ⊥ `rust-toolchain.toml`.
- C5: deps minimal & justified per crate in `Cargo.toml` comment. `cargo-deny` gate (licenses, advisories, duplicates). `clippy` `unwrap_used`/`expect_used` = deny.

## §I INTERFACES

- cmd: `xnl check [--format human|json|sarif] [paths…]` → detect embeds; exit 1 ∃ violation.
- cmd: `xnl extract [--write] [--relocate] <path>[:line]…` → move embed to extract file, rewrite host to load it; ≥1 path, each ∀ its sites or one `:line`; default = print unified diff (all paths, sorted); exit 1 if diff non-empty.
- cmd: `xnl graph [--format human|json|sarif] [paths…]` → host → extract load edges; flags orphan extracts & dangling loads.
- cmd: `xnl lint [--fix] [paths…]` → run ∀ configured check ∀ extract by language; `--fix` runs fixers then re-checks, extracts only; missing binary = error, ⊥ skip.
- cmd: `xnl langs [--format human|json]` → ∀ known language (`LangId`): role host \| guest \| both, compiled in (feature `lang-<lang>` on) y/n, sinks, delimiter kinds, load idiom, default linter.
- flag: `--verbose` ∀ verb; silence = success otherwise.
- flag: `--strict-hosts` ∀ verb: unclaimed file → exit 2 (≡ `[langs] unclaimed = "error"`).
- warnings: human → stderr, json → `warnings[]`; ⊥ change exit code.
- exit: 0 ok · 1 violation | drift · 2 usage | config error | refused. several apply → highest wins (2 > 1 > 0).
- rules: `xenolith` (non-trivial guest in sink; holes → same rule w/ `Judgment` direction, `languages/api:V40`), `dangling-load` & `orphan-extract` (`src/graph:V7`), `stale-allow` (`src/config:V9`), `stale-rule` (`src/config:V44`), `host-parse-error` (`languages:V78`), `stale-exclude` (`src/config:V79`). kebab-case, stable ∀ schema version.
- human output: `file:line:col xenolith: <guest> in <host> <sink> (<why>)`; `xnl extract` diff header `removing xenolith → <extract path>`. metaphor lives in rule id & wording, verbs stay conventional (`check`, `extract`, `graph`, `lint`, `langs`) ∼ fleet `mth check`, `sherd check`.
- json: violation = `{rule, file, line, col, host, guest, sink, site, why, directions[]}`; `site` = `DelimKind` (`languages/api` §I); each direction `Mechanical`|`Judgment` (mirror microlith shape).
- json envelope: `{"schema": 1, "violations": [...], "warnings": [{"code", "file"?, "message"}]}`; keys sorted, arrays sorted per `src:V11`.
- lib: `xenolith::check(&Path, &Config) -> Vec<Violation>`, `xenolith::extract(...) -> Edit`, `xenolith::graph(...) -> Graph`.
- hk: consumer step `check = "xnl check {{files}}"`, `check_diff = "xnl extract {{files}}"` (hk shows proposed extraction), `fix = "xnl extract --write {{files}}"` (explicit `hk fix` only, C15); `xnl graph`, `xnl lint` as own steps.
- flag: `--trust-config`: permit commands defined in any `xenolith.toml` (`[lint.<guest>]` checks/fixers, `[lint] all`); ⊥ config key can grant it.
- cmd: `xnl lint` gains `--format human|json|sarif` (see `src/lint` §I).
- cmd: `xnl init` → write minimal `xenolith.toml` (`version = 1`) in cwd; ⊥ overwrite; prints detected languages & suggested deviations, writes none (convention over configuration).
- cmd: `xnl migrate [--write]` → read `.nix-embedded-shell-allowlist`, `.pkl-embedded-shell-allowlist` (& siblings) → `[[allow]]` entries keyed per `src/config:V10`, `reason` = original comment \| `migrated from <file>`; default prints diff.
- cmd: `xnl inline [--write] <extract>…` → put extract body back into host via lens inverse (`languages/api:V34`); default prints diff; refuses (exit 2) if extract loaded by >1 host.

## §V INVARIANTS

V1: ∀ violation carries `rule`, `file:line:col`, host lang, guest lang, sink, site delimiter kind, `why`, ≥1 direction. ⊥ bare "bad".
V11: deterministic: output order sorted (file, line, col); json byte-stable across runs & platforms.
V12: CPU only, offline: test runs w/ network disabled; ⊥ `reqwest`/`ureq`/`hyper` in dep tree (cargo-deny ban).
V13: unclaimed file (0 hosts claim, named or not) ⊥ scanned & ⊥ reported by default; `--strict-hosts` \| `[langs] unclaimed = "error"` → exit 2 "host unsupported"; `warn` → warning.
V18: `rust-version` ≡ pinned rustc minor; CI asserts.
V24: exit codes stable: 0 ok, 1 violation, 2 usage/config/unsupported. json schema versioned (`"schema": 1`).
V30: ∀ `lang-*` feature toggleable: build + test green w/ each feature alone & w/ none (`cargo hack --each-feature`). language compiled out → its files unclaimed per `src:V13`; strict → exit 2 message names missing feature `lang-<lang>`. ⊥ `cfg` leak: engine code ⊥ names a language outside its feature gate.
V41: registry = ONE file in root crate: `hosts() -> &'static [&'static dyn Host]`, `guests() -> &'static [&'static dyn Guest]`, each entry behind `#[cfg(feature = "lang-<lang>")]`, sorted by `LangId`. engines iterate registry; ⊥ `cfg(feature = "lang-*")` elsewhere (V30 no-leak made checkable).
V42: guest compiled out: site whose `guest` ∉ `guests()` → per `[langs] missing_guest` (default `error`: exit 2 naming feature `lang-<guest>`; `warn` → warning; `ignore`); ⊥ guessing trivial/non-trivial.
V57: `xnl check|graph|lint` w/ ⊥ paths → candidates = `git ls-files` (tracked only ∴ `.gitignore` honoured); ⊥ git repo & ⊥ paths → exit 2 usage.
V95: scan parallel per file, results merged then sorted (V11) ∴ output byte-identical to serial run; `--jobs N` (default cores).
V102: `--format sarif` = SARIF 2.1.0: rule id → `ruleId`, `file:line:col` → `physicalLocation`, `why` → `message`, `Mechanical` direction → `fixes` when diff known; lint findings keep tool `code` as `ruleId` under tool's own `run`; deterministic like V11.

## §T TASKS

id|status|task|cites
T3|.|`Cargo.toml` (edition 2024, rust-version 1.95, MIT, lints), `clippy.toml`, `rustfmt.toml`, `deny.toml` w/ network crate bans|C2,C5,V12,V18
T8|.|core model: `Violation`, `Direction` over api `Site`/`LangId`; json schema v1; sorted output|V1,V11,V24,`languages/api:T42`
T9|.|CLI skeleton `xnl` (check, extract, graph, lint, langs; `--format`, `--verbose`); exit codes|I.cmd,V24,V13
T40|.|feature matrix: `lang-*` features in root `Cargo.toml`, `cargo-hack` in devShell, hk pre-push + CI `cargo hack --each-feature test`|V30,C1
T46|.|registry file + compiled-out guest exit 2; test builds w/ `lang-nix` only & asserts nix→shell site exits 2 naming `lang-shell`|V41,V42,V30
T58|.|candidate discovery via `git ls-files`; fixture: untracked & ignored files ⊥ scanned; outside git → exit 2|V57
T75|.|unclaimed handling: default ignore, `--strict-hosts` & config error/warn; fixture: hk-style file list w/ `.png`, `.md`|V13
T88|.|`missing_guest` error/warn/ignore; fixture on `lang-nix`-only build|V42
T96|.|`xnl init` + fixture: empty repo, existing config refused|`src/config:V89`
T97|.|`xnl migrate` + fixtures per legacy allowlist format|`src/config:V10`
T98|.|parallel scan + determinism test (serial vs `--jobs 8` byte-equal)|V95,V11
T103|.|SARIF writer ∀ verb + schema validation test|V102

## §B BUGS

id|date|cause|fix
