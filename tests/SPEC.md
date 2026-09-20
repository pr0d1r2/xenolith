# SPEC

## §G GOAL

fixture FORMAT (V67) & runner; cross-language & engine integration fixtures (language fixtures live in each crate, V14); bats under `tests/unit/` mirroring `scripts/` & `.github/scripts/`.

## §N NAV

rel|path|lens
up|.|-
self|tests|fixture format & runner, integration fixtures, bats mirroring `scripts/` & `.github/scripts/`
sib|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
sib|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
sib|scripts|∀ shell in repo: dev shell hook, guardrail scripts
sib|nix|flake inputs, packaging, devShell, cachix, subset override, closure
sib|docs|public project docs & notices

## §V INVARIANTS

V14: ∀ language crate owns its fixtures @ `languages/<lang>/tests/fixtures/<case>/`, shipped in crate package ∴ `cargo test` works from crates.io; root `tests/fixtures/` = cross-language & engine integration only.
V15: ∀ rule ∃ ≥1 positive (flagged) & ≥1 negative (clean) fixture.
V67: fixture case = `input.<ext>`, `expected.json` (check output), `expected/` tree = full post-`extract --write` state relative to case dir (rewritten host, ∀ extract, ∀ companion, ∀ nested level); compared byte-for-byte & exact file set.
V118: ∀ corpus finding (crash, lens-law break, wrong guest, false positive \| negative) → minimal SYNTHETIC fixture in the owning crate (`tests:V14`) + `§B` row, in the fixing PR; corpus data leaves the machine only as counts & shapes (`scripts/guard` C17).
V119: lens laws fuzzed: `rewrite`/`inline` (`languages/api/src/lens:V34`), `escape`/`unescape` (`languages/api/src/lens:V39`), `shebang::wrap`/`strip_strict` (`languages/api/src/lens:V63`) — property tests bounded in CI, cargo-fuzz targets per language crate run locally; crash or law break → fixture per V118.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end | T68 | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |
| M2 | language survey & corpus run | T117 | counts & failure shapes recorded, every finding filed as a fixture (`tests:V118`) |
| M3 | publication | T118 | public doc set, release machinery, history audit green, crates published (`.:T32`) |

id|status|task|cites
T68|.|fixture runner comparing `expected/` tree (bytes + file set)|V67,V14
T117|.|local corpus runner: `xnl check`/`extract --dry-run` over sibling repos, writes aggregate report only; finding → fixture checklist|V118
T118|.|proptest harness over laws + cargo-fuzz targets (nix, shell, pkl first)|V119

## §B BUGS

id|date|cause|fix
