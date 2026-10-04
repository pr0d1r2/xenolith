# SPEC

## §G GOAL

fixture FORMAT (V67) & runner; cross-language & engine integration fixtures (language fixtures live in each crate, V14); bats under `tests/unit/` mirroring `scripts/` & `.github/scripts/`.

## §N NAV

rel|path|lens
up|.|-
self|tests|fixture format & runner, integration fixtures, bats mirroring `scripts/` & `.github/scripts/`
sib|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
sib|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
sib|scripts|∀ shell in repo: dev shell hook, guardrail scripts; gate config `hk.pkl`, vendored hk schema `pkl/Config.pkl`, `.github/workflows/`, `.github/zizmor.yml`
sib|nix|flake inputs, packaging, devShell, cachix, subset override, closure
sib|docs|public project docs & notices, README & root doc files in the fleet's shape
sib|dev|repo-maintaining tooling, `publish = false`: README generated blocks, third-party notices

## §V INVARIANTS

V14: ∀ language crate owns its fixtures @ `<crate dir>/tests/fixtures/<case>/`, shipped in crate package ∴ `cargo test` works from crates.io; root `tests/fixtures/` = cross-language & engine integration only.
V15: ∀ rule ∃ ≥1 positive (flagged) & ≥1 negative (clean) fixture.
V67: fixture case = `input.<ext>`, `expected.json` (check output), `expected/` tree = full post-`extract --write` state relative to case dir (rewritten host, ∀ extract, ∀ companion, ∀ nested level); compared byte-for-byte & exact file set.
V118: ∀ corpus finding (crash, lens-law break, wrong guest, false positive \| negative) → minimal SYNTHETIC fixture in the owning crate (`tests:V14`) + `§B` row, in the fixing PR; corpus data leaves the machine only as counts & shapes (`scripts/guard` C17).
V119: lens laws fuzzed: `rewrite`/`inline` (`languages/api/src/lens:V34`), `escape`/`unescape` (`languages/api/src/lens:V39`), `shebang::wrap`/`strip_strict` (`languages/api/src/lens:V63`) -- property tests bounded in CI, cargo-fuzz targets per language crate run locally; crash or law break → fixture per V118.
V150: ∀ bats touching git = sandboxed: `setup` unsets ∀ `GIT_*` env (hook-exported `GIT_DIR`, `GIT_INDEX_FILE`, `GIT_WORK_TREE` ... override `git -C`), points `GIT_CONFIG_GLOBAL` at a tmp file, `GIT_CONFIG_NOSYSTEM=1`, `GIT_CEILING_DIRECTORIES` = test tmpdir ∴ suite run from a git hook (hk pre-commit \| pre-push) ⊥ reads \| writes the enclosing repo. ∀ such file ∃ regression test staging hook env over a sentinel repo, asserting sentinel untouched.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end | T68, T154 | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |
| M2 | language survey & corpus run | T117 | counts & failure shapes recorded, every finding filed as a fixture (`tests:V118`) |
| M3 | publication | T118 | public doc set, release machinery, history audit green, crates published (`.:T32`) |

id|status|task|cites
T68|.|fixture runner comparing `expected/` tree (bytes + file set)|V67,V14
T117|.|local corpus runner: `xnl check`/`extract --dry-run` over sibling repos, writes aggregate report only; finding → fixture checklist|V118
T118|.|proptest harness over laws + cargo-fuzz targets (nix, shell, pkl first)|V119
T154|.|fleet pilot: one sibling repo swaps its nix no-embedded-shell hook for `xnl check` + migrated `xenolith.toml`; same verdicts, ⊥ repo names here (`scripts/guard` C17)|V118,`.:T32`

## §B BUGS

id|date|cause|fix
B1|2026-09-26|bats fixtures ran `git -C <tmp> init` & `git -C <tmp> config user.*` under hook env: git exports `GIT_DIR` / `GIT_INDEX_FILE` to hooks & env beats `-C` ∴ writes hit the enclosing repo -- `.git/config` got `user.name=test`, `user.email=t@example.com`, `core.worktree` → an agent worktree; ∀ later git cmd in the checkout read the wrong tree & would commit as `test`|V150, T151
