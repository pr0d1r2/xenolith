# SPEC

## §G GOAL

host parsers (`rnix`, tree-sitter grammars) & host × sink matrix: find sinks in host AST, emit embeds.

## §N NAV

rel|path|lens
up|.|-
up|src|lib + `lyd` bin: core model, CLI, verbs, host/shell/extract/graph/lint/config nodes
self|src/hosts|host parsers & host × sink matrix, embed detection
sib|src/config|`lydite.toml` parse & validation
sib|src/extract|embed → own file, host rewrite, diff | `--write`
sib|src/graph|host → extract load edges, dangling & orphan
sib|src/lint|per-language linter map & invocation
sib|src/shell|single-command classifier on bash AST

## §V INVARIANTS


## §T TASKS

id|status|task|cites

## §B BUGS

id|date|cause|fix
