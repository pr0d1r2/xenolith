# SPEC

## §G GOAL

lib + `lyd` bin: core model, CLI, verbs; host/shell/extract/graph/lint/config as child nodes.

## §F FEDERATION

dir|owns|⊥owns|tokens
config|`lydite.toml` parse & validation|applying config — each verb's node|-
extract|embed → own file, host rewrite, diff \| `--write`|detection (`hosts`), load resolution (`graph`)|-
graph|host → extract load edges, dangling & orphan|writing files (`extract`)|-
hosts|host parsers & host × sink matrix, embed detection|shell classification (`shell`), rewriting (`extract`)|-
lint|per-language linter map & invocation|deciding what is an extract (`graph`, `config`)|-
shell|single-command classifier on bash AST|finding sinks (`hosts`)|-

## §N NAV

rel|path|lens
up|.|-
self|src|lib + `lyd` bin: core model, CLI, verbs, host/shell/extract/graph/lint/config nodes
sib|scripts|∀ shell in repo: dev shell hook, guardrail scripts
sib|tests|fixtures per host & case, integration tests, bats mirroring `scripts/`

## §V INVARIANTS


## §T TASKS

id|status|task|cites

## §B BUGS

id|date|cause|fix
