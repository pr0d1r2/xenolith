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

## §V INVARIANTS

V14: ∀ language crate owns its fixtures @ `languages/<lang>/tests/fixtures/<case>/`, shipped in crate package ∴ `cargo test` works from crates.io; root `tests/fixtures/` = cross-language & engine integration only.
V15: ∀ rule ∃ ≥1 positive (flagged) & ≥1 negative (clean) fixture.
V67: fixture case = `input.<ext>`, `expected.json` (check output), `expected/` tree = full post-`extract --write` state relative to case dir (rewritten host, ∀ extract, ∀ companion, ∀ nested level); compared byte-for-byte & exact file set.

## §T TASKS

id|status|task|cites
T68|.|fixture runner comparing `expected/` tree (bytes + file set)|V67,V14

## §B BUGS

id|date|cause|fix
