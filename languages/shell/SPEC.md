# SPEC

## §G GOAL

crate `xenolith-lang-shell` (feature `lang-shell`): tree-sitter-bash; bash as host (heredoc to interpreter, `-c`/`-e` args), single-command classifier shared by ∀ shell sink, shell extract load idiom, default linters shellcheck + shfmt.

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
self|languages/shell|bash parser & host sinks, single-command classifier, shell linters
sib|languages/api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness
sib|languages/nix|nix parser, sinks, load idiom
sib|languages/pkl|pkl parser, hk step sinks, load idiom
sib|languages/just|just parser, recipe sinks, load idiom
sib|languages/python|python grammar, guest rules
sib|languages/sql|sql grammar, guest rules
sib|languages/jq|jq grammar, guest rules
sib|languages/awk|awk grammar, guest rules
sib|languages/yaml|yaml parser, GH Actions sinks, placement
sib|languages/dockerfile|Dockerfile parser, `RUN` sinks, placement
sib|languages/shebang|shebang parse/strip/wrap ∀ guest
sib|languages/rust|rust parser, rust host sinks
sib|languages/ruby|ruby parser, ruby host sinks

## §I INTERFACES

- sinks: heredoc fed to interpreter (`python <<`, `ruby <<`, `psql <<`), `-c`/`-e` args (`python -c`, `ruby -e`, `node -e`, `perl -e`, `sh -c`, `bash -c`), `awk` program > threshold ?, `jq` filter > threshold ? → guest python \| ruby \| sql \| js \| perl \| awk \| jq; load after extract: `python scripts/x.py`, `jq -f x.jq`, `awk -f x.awk`.
- `[extract.shell] strict` ∈ `preserve` (default) \| `enforce`: `enforce` → prelude `set -euo pipefail` regardless of context, diff marks it `Judgment` (semantic change).
- placement prototype ? (T86 evaluates): bash host → `<host_dir>/<host_stem>.<name>.<ext>`, load via `"$(dirname "${BASH_SOURCE[0]}")/…"` (whitelisted in V3).

## §V INVARIANTS

V3: sink w/ single simple command (argv, optional leading `NAME=value` assignments; generated load may use exactly `"$(dirname "${BASH_SOURCE[0]}")"` as path prefix; otherwise ⊥ `|`, `&&`, `\|\|`, `;`, `$(`, backtick, redirect, `if`/`for`/`while`/`case`, heredoc, subshell, function def) = allowed. ≥1 control construct = violation, unless construct ∈ `[threshold.shell] allow` (`src/config` §I). classification via shell AST (tree-sitter-bash), ⊥ substring grep.
V51: shell guest defaults: `prelude(env)` = shebang `#!/usr/bin/env <dialect>` + `set`/`setopt` line reproducing `env.options` (V82); ⊥ context → bash + `set -euo pipefail`; `executable` = true; ext `sh` (zsh → `zsh`); `invoke` = `<dialect> {path}`.
V82: dialects `sh`, `bash`, `zsh` (`dash`/`ksh` ? as sh-family): `env.dialect` from context (`sh -c`, `bash -c`, `zsh -c`, shebang, GH `shell:`, nix systemd `script`; host declares) & `env.options` = effective `set -o`/`setopt` state; prelude reproduces both exactly ∴ semantics preserved. grammar: tree-sitter-bash ∀ sh & bash, zsh best-effort ? (unsupported construct → `Judgment`).

## §T TASKS

id|status|task|cites
T11|.|shell single-command classifier on tree-sitter-bash AST (shared by all shell sinks)|V3,`languages:V2`
T15|.|host bash: heredoc-to-interpreter, `-c`/`-e` args, awk/jq threshold ? + fixtures|`languages:V2`,`tests:V14`,`tests:V15`
T53|.|shell `Guest::prelude`/`executable`/`invoke` defaults + fixture proving extract passes shellcheck|V51
T83|.|dialect & option capture per shell host context; fixtures: `sh -c`, `bash -c` under `set -e`, `zsh -c` w/ `setopt`|V82

## §B BUGS

id|date|cause|fix
