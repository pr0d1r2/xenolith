# SPEC

## §G GOAL

single-command classifier on tree-sitter-bash AST, shared by ∀ shell sink.

## §N NAV

rel|path|lens
up|.|-
up|src|lib + `lyd` bin: core model, CLI, verbs, host/shell/extract/graph/lint/config nodes
self|src/shell|single-command classifier on bash AST
sib|src/config|`lydite.toml` parse & validation
sib|src/extract|embed → own file, host rewrite, diff | `--write`
sib|src/graph|host → extract load edges, dangling & orphan
sib|src/hosts|host parsers & host × sink matrix, embed detection
sib|src/lint|per-language linter map & invocation

## §V INVARIANTS


## §T TASKS

id|status|task|cites

## §B BUGS

id|date|cause|fix
