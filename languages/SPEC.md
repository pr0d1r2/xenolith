# SPEC

## §G GOAL

∀ language = 1 workspace crate `lydite-lang-<lang>` behind feature `lang-<lang>`, 1 spec node; shared language contract (parse, sinks, load idiom, default linter). host tasks for languages w/o own crate yet wait here.

## §F FEDERATION

dir|owns|⊥owns|tokens
nix|nix parser, sinks, load idiom|shell classification (`languages/shell`)|-
pkl|pkl parser, hk step sinks, load idiom|shell classification (`languages/shell`)|-
shell|bash parser & host sinks, single-command classifier, shell linters|sinks in other hosts (their node)|-

## §N NAV

rel|path|lens
up|.|-
self|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
sib|src|root crate: lib + `lyd` bin, core model, CLI, cross-language engines extract/graph/lint/config
sib|scripts|∀ shell in repo: dev shell hook, guardrail scripts
sib|tests|fixtures per host & case, integration tests, bats mirroring `scripts/`

## §V INVARIANTS


## §T TASKS

id|status|task|cites

## §B BUGS

id|date|cause|fix
