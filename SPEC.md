# SPEC

## §G GOAL
purity of implementation: ∀ file holds ONE language. `xenolith` detects embeds of other language inside host file, extracts them to own file, rewrites host to cross-load extract, verifies ∀ load resolves & ∀ extract linted by its own linter. named for xenolith — foreign rock fragment enclosed in host rock (geology; `-lith` ∼ sibling `microlith`). embed = xenolith in host file; tool finds & removes them → host of one material.

## §F FEDERATION

dir|owns|⊥owns|tokens
languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter|engines (`src`)|-
src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config|language specifics (`languages`), repo tooling (`scripts`), fixtures (`tests`)|-
scripts|∀ shell in repo: dev shell hook, guardrail scripts|product rules (`src`), bats (`tests`)|-
tests|fixtures per host & case, integration tests, bats mirroring `scripts/`|implementation (`src`, `scripts`)|-

## §N NAV

rel|path|lens
up|-|-
self|.|-

## §C CONSTRAINTS
- C1: Rust. cargo workspace; repo & root crate = `xenolith` (lib + bin `xnl` = `xenolith` consonant skeleton, short invocation, `ripgrep`/`rg` shape; `xnl` free as crate, nixpkgs & brew binary — checked 2026-09-19); lib exposed for consumers. ∀ language = member crate `languages/<lang>` named `xenolith-lang-<lang>`: its grammar dep, parser, sinks as host, load idiom, default linter as embedded. root crate depends on language crates, each OPTIONAL behind cargo feature `lang-<lang>`; `default` = ∀ `lang-*` ∴ `xnl` works out of box, consumer trims via `default-features = false`. feature = compile-time (grammar ∉ binary); `xenolith.toml` `[langs]` = runtime toggle over compiled-in set. contract crate `languages/api` = `xenolith-lang-api` (⊥ feature-gated, ⊥ grammar dep): `Host` & `Guest` traits, shared types, law harness. ∀ workspace crate (api, ∀ `xenolith-lang-*`, root) published to crates.io, lockstep version. language crate depends only on api + own grammar; ⊥ on root crate, ⊥ on other language crate ∴ host names guest by `LangId`, ⊥ by type. MIT.
- C2: edition `2024`, `rust-version = "1.95"` ≡ rustc in pinned nixpkgs. ⊥ rust-overlay, ⊥ fenix, ⊥ `rust-toolchain.toml`.
- C3: CPU only, offline, deterministic. ⊥ network, ⊥ model, ⊥ heuristic scoring w/ randomness. same input → same bytes out.
- C4: real parsers, ⊥ regex over source. nix → `rnix`; others → `tree-sitter` + per-language grammar crates (bash, yaml, rust, ruby, html, javascript, css, python, sql, jq, awk, just, dockerfile ?, pkl ?). grammar missing for host → host unsupported, ⊥ regex fallback.
- C5: deps minimal & justified per crate in `Cargo.toml` comment. `cargo-deny` gate (licenses, advisories, duplicates). `clippy` `unwrap_used`/`expect_used` = deny.
- C6: nix flake. inputs ! `nixpkgs-lock` (`github:pr0d1r2/nixpkgs-lock`), `nix-hk` (`github:pr0d1r2/nix-hk`), `itok` (`github:pr0d1r2/itok`), `microlith` (`github:pr0d1r2/microlith`), `sherd` (`github:pr0d1r2/sherd`). `nixpkgs.follows = "nixpkgs-lock/nixpkgs"`; ∀ other input follows root `nixpkgs-lock` & `nix-hk`; tool→tool edges (`microlith`→`itok`, `itok`→`microlith`) follow root. ⊥ other nixpkgs edge. `flake.lock` committed, pure eval, ⊥ IFD. tool inputs = dev/guardrail only, ⊥ in `packages.default` closure.
- C7: systems: declared 4 (`aarch64-darwin`, `x86_64-darwin`, `x86_64-linux`, `aarch64-linux`); tier-1 3 (⊥ `x86_64-darwin`) built + pushed cachix `pr0d1r2` from `main` only (mirror `nix-hk`).
- C8: consumers take `xenolith` as flake input w/ `inputs.nixpkgs-lock.follows = "nixpkgs-lock"` ∴ same rev, cache hit. consumer picks language subset → `lang-*` features ∴ binary carries only those grammars & sinks. subset ≠ default → built locally (cachix holds default = all only), trade: smaller & faster binary vs cache miss.
- C9: guardrails = `hk` (from `nix-hk`), `hk.pkl`. ∀ hk step = one plain command a human can paste (`cargo fmt --check`, `xnl check {{files}}`); ⊥ inline shell logic (dogfood `languages/shell:V3`).
- C10: `nix develop` shellHook runs `scripts/dev/shell-hook.sh` → `hk install` idempotent ∴ hooks current ∀ shell enter. shellHook wired via `builtins.readFile`, ⊥ inline.
- C11: TDD. RED commit (`test:` failing `#[test]` | fixture | bats) → GREEN commit (`feat:`|`fix:`) → REFACTOR commit (`refactor:`) ?. test commit ! precede impl commit.
- C12: atomic commits. 1 logical change / commit. Conventional Commits. body ! carry agent reasoning (`Why:` + cite `§V.n`|`§T.n`) → audit trail.
- C13: any shell in repo ∈ `scripts/` \| `.github/scripts/` w/ 1-to-1 bats at mirrored path (`scripts/a/b.sh` ↔ `tests/unit/scripts/a/b.bats`, `.github/scripts/ci/x.sh` ↔ `tests/unit/.github/scripts/ci/x.bats`); `set -euo pipefail`, shellcheck, shfmt clean.
- C14: Rust coverage via `cargo llvm-cov`, floor in `.coverage`, gated by `sherd coverage --check`, ratchets up only (`--record` refuses drop). lint debt ratchet via `sherd debt --check` vs `.lint-debt`.
- C15: report-only by default. `xnl extract` writes only w/ explicit `--write`; default prints diff.
- C16: single config `xenolith.toml` at consumer repo root. replaces per-language allowlist files (`.nix-embedded-shell-allowlist`, `.pkl-embedded-shell-allowlist`, …).
- C17: ⊥ private repo named in source, fixtures, docs, commit msgs. public repos (`nix-hk`, `nixpkgs-lock`, `itok`, `microlith`, `sherd` — verified PUBLIC 2026-09-18; `nix-shebang` — verified PUBLIC 2026-09-19) may be named. unknown = private. fixtures synthetic | anonymised.
- C18: markdown fenced code = documentation, ⊥ embed. out of scope by default ?.
- C19: dogfood: xenolith repo passes `xnl check` on itself in CI.
- C20: cycle risk: itok, microlith, sherd may later adopt xenolith as guard → flake input cycle. ∴ those edges ! be devShell-only & `follows` root; ⊥ lib (cargo) dep on each other ?. decide before first consumer adopts.
- C21: spec toolchain in guardrails: `microlith` (`mth fmt --check`, `mth check` ∀ `SPEC.md`), `itok` (`itok check` vs `.context-limits`), `sherd` (`sherd validate`, `sherd sync --check`, `sherd check`, `sherd budget`, `sherd coverage --check` vs `.coverage`, `sherd debt --check` vs `.lint-debt`; `sherd review` advisory ?). ∀ hk step one plain command (C9) ∴ remediation text in tool output | `scripts/hk/*.sh`, ⊥ inline `\|\| { echo …; }`.
- C22: spec = sherd DAG federation. root `SPEC.md` holds §G, §C, §I, §F; ∀ node dir own `SPEC.md` w/ §V, §T, §B; §N generated by `sherd sync`. citations namespaced (`` `NODE:Vn` ``). nodes: `languages` hub → `languages/api` (contract crate) + 1 node per language crate (`languages/shell` incl. single-command classifier, `languages/nix`, `languages/pkl`, `languages/just`, `languages/python`, `languages/sql`, `languages/jq`, `languages/awk`; others added w/ their crate, host tasks wait @ hub until then); `src` hub → `src/extract`, `src/graph`, `src/lint`, `src/config` (cross-language engines calling into language crates); `scripts` hub → `scripts/guard` (repo guardrails); `tests` (fixtures). spec node = dir = crate for languages. federate FIRST, before code: node dir may hold only `SPEC.md` until its code lands ∴ ∀ task cites node-local ids from day one & ⊥ big-bang migration later.

## §I INTERFACES
- cmd: `xnl check [--format human|json] [paths…]` → detect embeds; exit 1 ∃ violation.
- cmd: `xnl extract [--write] <path>[:line]…` → move embed to extract file, rewrite host to load it; ≥1 path, each ∀ its sites or one `:line`; default = print unified diff (all paths, sorted); exit 1 if diff non-empty.
- cmd: `xnl graph [--format human|json] [paths…]` → host → extract load edges; flags orphan extracts & dangling loads.
- cmd: `xnl lint [--fix] [paths…]` → run ∀ configured check ∀ extract by language; `--fix` runs fixers then re-checks, extracts only; missing binary = error, ⊥ skip.
- cmd: `xnl langs [--format human|json]` → ∀ known language (`LangId`): role host \| guest \| both, compiled in (feature `lang-<lang>` on) y/n, sinks, delimiter kinds, load idiom, default linter.
- flag: `--verbose` ∀ verb; silence = success otherwise.
- flag: `--strict-hosts` ∀ verb: unclaimed file → exit 2 (≡ `[langs] unclaimed = "error"`).
- warnings: human → stderr, json → `warnings[]`; ⊥ change exit code.
- exit: 0 ok · 1 violation | drift · 2 usage | config error | refused. several apply → highest wins (2 > 1 > 0).
- file: `xenolith.toml` — schema & defaults in `src/config` §I: `version`, `[extract]`, `[[extract.rule]]`, `[extract.shell]`, `[[allow]] path, sink, hash, reason`, `[[exclude]]` & per-verb `exclude`, `[[detect]]`, `[lint.<guest>]`, `[langs]`, `[parse]`, `[threshold.*]`.
- rules: `xenolith` (non-trivial guest in sink; holes → same rule w/ `Judgment` direction, `languages/api:V40`), `dangling-load` & `orphan-extract` (`src/graph:V7`), `stale-allow` (`src/config:V9`), `stale-rule` (`src/config:V44`), `host-parse-error` (`languages:V78`), `stale-exclude` (`src/config:V79`). kebab-case, stable ∀ schema version.
- human output: `file:line:col xenolith: <guest> in <host> <sink> (<why>)`; `xnl extract` diff header `removing xenolith → <extract path>`. metaphor lives in rule id & wording, verbs stay conventional (`check`, `extract`, `graph`, `lint`, `langs`) ∼ fleet `mth check`, `sherd check`.
- json: violation = `{rule, file, line, col, host, guest, sink, site, why, directions[]}`; `site` = `DelimKind` (`languages/api` §I); each direction `Mechanical`|`Judgment` (mirror microlith shape).
- lib: `xenolith::check(&Path, &Config) -> Vec<Violation>`, `xenolith::extract(...) -> Edit`, `xenolith::graph(...) -> Graph`.
- nix: `packages.<sys>.default` = xenolith; `overlays.default` ?; `checks` run cargo test + clippy + dogfood.
- nix: `packages.<sys>.default.override { languages = [ "nix" "pkl" ]; }` → `buildNoDefaultFeatures` + `buildFeatures = lang-<l>` ∀ l; default `languages` = ∀ supported.
- hk: consumer step `check = "xnl check {{files}}"`, `check_diff = "xnl extract {{files}}"` (hk shows proposed extraction), `fix = "xnl extract --write {{files}}"` (explicit `hk fix` only, C15); `xnl graph`, `xnl lint` as own steps.
- file (this repo): `.context-limits` (itok ceilings), `.coverage` (floor), `.lint-debt` (sherd debt baseline).

### host × sink matrix (initial)
host|sink detected|embedded|load idiom after extract
nix|`script`, `preStart`, `postStart`, `shellHook`, `ExecStart*`, `text` of `writeShellScript*`/`writeShellApplication`, `runCommand` body, `buildPhase`/`installPhase`/`*Phase`|shell|per `languages/nix:V53`
pkl|hk step `check`, `fix`, `shell`, `check_diff`, `check_list_files`|shell|`bash scripts/hk/x.sh {{files}}`
just|recipe body > single simple command, shebang recipe|shell \| python \| …|`bash scripts/x.sh`
yaml (GH Actions)|`run:` block|shell|per `languages:V75`
Dockerfile|`RUN` > single simple command|shell|`COPY` + `RUN bash /x.sh`
bash|heredoc fed to interpreter (`python <<`, `ruby <<`, `psql <<`), `-c`/`-e` args (`python -c`, `ruby -e`, `node -e`, `perl -e`, `sh -c`, `bash -c`), `awk` program > threshold ?, `jq` filter > threshold ?|python \| ruby \| sql \| js \| perl \| awk \| jq|`python scripts/x.py`, `jq -f x.jq`, `awk -f x.awk`
rust|`Command::new("sh"\|"bash").arg("-c")`, SQL string literal passed to query fn ?|shell \| sql|`include_str!("x.sql")`
ruby|squiggly heredoc tagged `SQL`/`SH`/`JS`, backticks, `system("…")` w/ control syntax|sql \| shell \| js|`File.read(…)` / `Rails.root.join` ?
html|inline `<script>` body, inline `<style>` body, `on*=` attrs ?|js \| css|`<script src>`, `<link rel=stylesheet>`

## §V INVARIANTS
V12: CPU only, offline: test runs w/ network disabled; ⊥ `reqwest`/`ureq`/`hyper` in dep tree (cargo-deny ban).
V16: rule & its checker & its fixtures land in ONE commit; RED test commit precedes (C11).
V17: flake inputs ! exactly `nixpkgs-lock`, `nix-hk`, `itok`, `microlith`, `sherd` (+ follows per C6); `flake.lock` holds exactly 1 nixpkgs node, rev ≡ nixpkgs-lock rev; check fails otherwise.
V18: `rust-version` ≡ pinned rustc minor; CI asserts.
V19: xenolith repo passes own `xnl check`, `xnl graph`, `xnl lint` (dogfood, C19).
V22: `cargo fmt --check`, `clippy -D warnings`, `cargo deny check`, `cargo test` green before push; hk `pre-push` enforces.
V25: ∀ `SPEC.md` (root & nodes) pass `mth fmt --check` & `mth check`.
V26: ∀ path ∈ `.context-limits` ≤ ceiling via `itok check`; ceiling raise only in own commit w/ `Why:`.
V27: federation consistent: `sherd validate`, `sherd sync --check`, `sherd check`, `sherd budget` green; §N ⊥ hand-edited.
V28: coverage ≥ `.coverage` floor & lint debt ≤ `.lint-debt` (`sherd coverage --check`, `sherd debt --check`); both ratchet one way.
V29: `packages.default` closure ∌ itok, microlith, sherd, hk (dev-only inputs, C6).
V30: ∀ `lang-*` feature toggleable: build + test green w/ each feature alone & w/ none (`cargo hack --each-feature`). language compiled out → its files unclaimed per `src:V13`; strict → exit 2 message names missing feature `lang-<lang>`. ⊥ `cfg` leak: engine code ⊥ names a language outside its feature gate.
V31: nix `languages` subset exact: `xnl langs` of subset build lists exactly subset as compiled in; unknown name → eval error listing supported names, ⊥ silent drop; empty list = eval error.

## §T TASKS
id|status|task|cites
T1|.|scaffold flake: inputs nixpkgs-lock + nix-hk w/ follows, devShell (rustc, cargo, clippy, rustfmt, cargo-deny, cargo-llvm-cov ?, hk, bats, shellcheck, shfmt, nixfmt, statix, deadnix), `.gitignore`, `flake.lock`|V17,C6
T3|.|`Cargo.toml` (edition 2024, rust-version 1.95, MIT, lints), `clippy.toml`, `rustfmt.toml`, `deny.toml` w/ network crate bans|C2,C5,V12,V18
T4|.|`hk.pkl`: fmt, clippy, deny, test, shellcheck, shfmt, nixfmt, statix, deadnix; commit-msg & pre-push hooks|C9,V22
T26|.|nix package `packages.default`, `checks` (test, clippy, dogfood); cachix push from CI `main`|C7,C19,V19
T27|.|CI workflow: tier-1 matrix, `hk check --all`, bats, `nix flake check`, cachix|C7,V22
T28|.|dogfood: `xnl check`/`graph`/`lint` on own repo green|V19,C19
T30|.|README: purpose, name origin, host matrix, `xenolith.toml` ref, consumer flake snippet w/ follows, `trusted-users` note for cachix|I.file,C8
T31|.|consumer migration doc: replacing `.nix-embedded-shell-allowlist` / `.pkl-embedded-shell-allowlist` w/ `xenolith.toml`|C16
T32|.|release: tag, CHANGELOG, crates.io publish ∀ workspace crate in dependency order api → languages → root, lockstep version|C1
T33|.|hk steps `mth fmt --check` & `mth check` ∀ `SPEC.md`; `mth fmt` as fix|V25,C21
T34|.|`.context-limits` ceilings + hk `itok check`|V26,C21
T35|.|hk steps `sherd validate`, `sherd sync --check`, `sherd check`, `sherd budget`; `sherd review` advisory ?|V27,C21
T36|.|`.coverage` floor + `.lint-debt` baseline; hk pre-push `sherd coverage --check`, `sherd debt --check` (supersedes C14 llvm-cov wiring in T1)|V28,C14
T37|.|federate spec before code: node dirs w/ `SPEC.md` + `§F` → `sherd adopt .` proposal → map file → `sherd adopt . --map` → `sherd sync`|V27,C22
T38|.|closure check: `nix path-info -r` of `packages.default` ∌ dev tools|V29,C6
T39|.|resolve C20 cycle policy before itok/microlith/sherd adopt xenolith|C20
T40|.|feature matrix: `lang-*` features in root `Cargo.toml`, `cargo-hack` in devShell, hk pre-push + CI `cargo hack --each-feature test`|V30,C1
T41|.|nix `languages` override arg → cargo features; flake check builds subset `[ "nix" ]` & asserts `xnl langs`; README consumer snippet w/ subset|V31,C8

## §B BUGS
id|date|cause|fix
