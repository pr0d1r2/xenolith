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

## §V INVARIANTS

V3: sink w/ single simple command (argv only: ⊥ `|`, `&&`, `\|\|`, `;`, `$(`, backtick, redirect, `if`/`for`/`while`/`case`, heredoc, subshell, function def) = allowed. ≥1 control construct = violation, unless construct ∈ `[threshold.shell] allow` (`src/config` §I). classification via shell AST (tree-sitter-bash), ⊥ substring grep.
V51: shell guest defaults: `header` = `#!/usr/bin/env bash` + `set -euo pipefail`, `executable` = true, `ext` = `sh`, `invoke` = `bash {path}`.

## §T TASKS

id|status|task|cites
T11|.|shell single-command classifier on tree-sitter-bash AST (shared by all shell sinks)|V3,`languages:V2`
T15|.|host bash: heredoc-to-interpreter, `-c`/`-e` args, awk/jq threshold ? + fixtures|`languages:V2`,`tests:V14`,`tests:V15`
T53|.|shell `Guest::header`/`executable`/`invoke` defaults + fixture proving extract passes shellcheck|V51

## §B BUGS

id|date|cause|fix
