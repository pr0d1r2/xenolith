# SPEC

## §G GOAL

∀ shell in repo: dev shell hook & guardrail scripts, each w/ mirrored bats.

## §F FEDERATION

dir|owns|⊥owns|tokens
guard|repo guardrail scripts hk calls|product rules (`src`), bats (`tests`)|-

## §N NAV

rel|path|lens
up|.|-
self|scripts|∀ shell in repo: dev shell hook, guardrail scripts
sib|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
sib|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
sib|tests|fixture format & runner, integration fixtures, bats mirroring `scripts/` & `.github/scripts/`

## §V INVARIANTS


## §T TASKS

id|status|task|cites
T2|.|`scripts/dev/shell-hook.sh` + bats (RED→GREEN): idempotent `hk install`, wired via `builtins.readFile`|C10,`scripts/guard:V21`

## §B BUGS

id|date|cause|fix
