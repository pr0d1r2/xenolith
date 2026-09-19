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

V14: ∀ host in §I matrix ∃ fixture dir `tests/fixtures/<host>/<case>/` w/ `input.*`, `expected.json`, & for extract `expected.host`, `expected.extract.*`.
V15: ∀ rule ∃ ≥1 positive (flagged) & ≥1 negative (clean) fixture.

## §T TASKS

id|status|task|cites

## §B BUGS

id|date|cause|fix
