# SPEC

## §G GOAL

per-language linter map & invocation over extracts; missing linter binary = error.

## §N NAV

rel|path|lens
up|.|-
up|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
self|src/lint|per-language linter map & invocation
sib|src/config|`xenolith.toml` parse & validation
sib|src/extract|embed → own file, host rewrite, diff | `--write`
sib|src/graph|host → extract load edges, dangling & orphan

## §V INVARIANTS

V8: `lint`: ∀ extract linted by linter for its language from config | built-in default; linter absent from PATH = exit 2, ⊥ silent skip.

## §T TASKS

id|status|task|cites
T24|.|`lint`: per-language linter map w/ defaults (shellcheck+shfmt, ruff ?, sqlfluff ?, eslint ?, stylelint ?), missing binary = exit 2|V8

## §B BUGS

id|date|cause|fix
