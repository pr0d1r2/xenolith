# SPEC

## §G GOAL

fixtures per host & case, integration tests, bats under `tests/unit/` mirroring `scripts/`.

## §N NAV

rel|path|lens
up|.|-
self|tests|fixtures per host & case, integration tests, bats mirroring `scripts/`
sib|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
sib|src|root crate: lib + `lyd` bin, core model, CLI, cross-language engines extract/graph/lint/config
sib|scripts|∀ shell in repo: dev shell hook, guardrail scripts

## §V INVARIANTS


## §T TASKS

id|status|task|cites

## §B BUGS

id|date|cause|fix
