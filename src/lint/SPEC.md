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

## §I INTERFACES

- default checks / fixers (all overridable): sh & bash → `shellcheck -s <dialect>`, `shfmt -d` / `shfmt -w`; zsh → `zsh -n`; python → `ruff check`, `ruff format --check`, `mypy` ? / `ruff check --fix`, `ruff format`; sql → `sqlfluff lint` / `sqlfluff fix`; jq → `jq -n -f {file}`; awk → `gawk --lint -f {file} /dev/null` ?.

## §V INVARIANTS

V8: `lint`: ∀ extract run ∀ check of its guest (defaults ∪ | replaced by `[lint.<guest>]`), each reported separately, ⊥ stop at first; binary absent from PATH = exit 2, ⊥ silent skip; `--fix` runs fixers then re-checks, touches extracts only.

## §T TASKS

id|status|task|cites
T24|.|`lint`: per-language linter map w/ defaults (shellcheck+shfmt, ruff ?, sqlfluff ?, eslint ?, stylelint ?), missing binary = exit 2|V8
T87|.|multi-check runner, per-check results, `--fix`; defaults table; fixture w/ 2 checks both failing|V8

## §B BUGS

id|date|cause|fix
