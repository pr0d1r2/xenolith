# SPEC

## §G GOAL

`lydite.toml` parse: extract dirs, `[[allow]]` (reason, hash|span, staleness), lint map, hosts toggle, thresholds.

## §N NAV

rel|path|lens
up|.|-
up|src|root crate: lib + `lyd` bin, core model, CLI, cross-language engines extract/graph/lint/config
self|src/config|`lydite.toml` parse & validation
sib|src/extract|embed → own file, host rewrite, diff | `--write`
sib|src/graph|host → extract load edges, dangling & orphan
sib|src/lint|per-language linter map & invocation

## §V INVARIANTS


## §T TASKS

id|status|task|cites

## §B BUGS

id|date|cause|fix
