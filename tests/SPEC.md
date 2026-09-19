# SPEC

## §G GOAL

fixtures per host & case, integration tests, bats under `tests/unit/` mirroring `scripts/`.

## §N NAV

rel|path|lens
up|.|-
self|tests|fixtures per host & case, integration tests, bats mirroring `scripts/`
sib|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
sib|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
sib|scripts|∀ shell in repo: dev shell hook, guardrail scripts

## §V INVARIANTS

V14: ∀ language crate owns its fixtures @ `languages/<lang>/tests/fixtures/<case>/`, shipped in crate package ∴ `cargo test` works from crates.io; root `tests/fixtures/` = cross-language & engine integration only.
V15: ∀ rule ∃ ≥1 positive (flagged) & ≥1 negative (clean) fixture.

## §T TASKS

id|status|task|cites

## §B BUGS

id|date|cause|fix
