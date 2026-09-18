# SPEC

## §G GOAL

move embed → own file & rewrite host to load it: diff default, `--write`, lossless, idempotent, collision guard.

## §N NAV

rel|path|lens
up|.|-
up|src|root crate: lib + `lyd` bin, core model, CLI, cross-language engines extract/graph/lint/config
self|src/extract|embed → own file, host rewrite, diff | `--write`
sib|src/config|`lydite.toml` parse & validation
sib|src/graph|host → extract load edges, dangling & orphan
sib|src/lint|per-language linter map & invocation

## §V INVARIANTS


## §T TASKS

id|status|task|cites

## §B BUGS

id|date|cause|fix
