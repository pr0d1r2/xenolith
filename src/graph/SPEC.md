# SPEC

## §G GOAL

host → extract load edges; dangling load & orphan extract detection.

## §N NAV

rel|path|lens
up|.|-
up|src|root crate: lib + `lyd` bin, core model, CLI, cross-language engines extract/graph/lint/config
self|src/graph|host → extract load edges, dangling & orphan
sib|src/config|`lydite.toml` parse & validation
sib|src/extract|embed → own file, host rewrite, diff | `--write`
sib|src/lint|per-language linter map & invocation

## §V INVARIANTS


## §T TASKS

id|status|task|cites

## §B BUGS

id|date|cause|fix
