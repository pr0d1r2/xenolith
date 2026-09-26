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
sib|languages/bats|bats grammar (based-on shell), `@test` sinks, test-host rules
sib|languages/yaml|yaml parser, GH Actions sinks, placement
sib|languages/dockerfile|Dockerfile parser, `RUN` sinks, placement
sib|languages/shebang|shebang parse/strip/wrap ∀ guest
sib|languages/rust|rust parser, rust host sinks
sib|languages/ruby|ruby parser, ruby host sinks
sib|languages/html|html parser, inline script/style sinks
sib|languages/js|javascript grammar, guest rules
sib|languages/css|css grammar, guest rules
sib|languages/perl|perl grammar, guest rules

## §I INTERFACES

- `claims`: `*.sh`, `*.bash`, `.envrc`, shebang resolving to a shell dialect (`shebang::resolves_to`) — ⊥ `*.bats` (`languages/bats:V134`, V137).
- sinks: heredoc fed to interpreter (`python <<`, `ruby <<`, `psql <<`), `-c`/`-e` args (`python -c`, `ruby -e`, `node -e`, `perl -e`, `sh -c`, `bash -c`), `awk` program > threshold ?, `jq` filter > threshold ? → guest python \| ruby \| sql \| js \| perl \| awk \| jq; load after extract: `python scripts/x.py`, `jq -f x.jq`, `awk -f x.awk`.
- `[extract.shell] strict` ∈ `preserve` (default) \| `enforce`: `enforce` → prelude `set -euo pipefail` regardless of context, diff marks it `Judgment` (semantic change).
- placement prototype ? (T86 evaluates): bash host → `<host_dir>/<host_stem>.<name>.<ext>`, load via `"$(dirname "${BASH_SOURCE[0]}")/…"` (whitelisted in V3).

## §V INVARIANTS

V3: sink w/ single simple command (argv, optional leading `NAME=value` assignments; generated load may use exactly `"$(dirname "${BASH_SOURCE[0]}")"` as path prefix; otherwise ⊥ `|`, `&&`, `\|\|`, `;`, `$(`, backtick, redirect, `if`/`for`/`while`/`case`, heredoc, subshell, function def) = allowed. ≥1 control construct = violation, unless construct ∈ `[threshold.shell] allow` (`src/config` §I). classification via shell AST (tree-sitter-bash), ⊥ substring grep.
V51: shell guest defaults: `prelude(env)` = shebang `#!/usr/bin/env <dialect>` + `set`/`setopt` line reproducing `env.options` (V82); ⊥ context → bash + `set -euo pipefail`; `executable` = true; ext `sh` (zsh → `zsh`); `invoke` = `<dialect> {path}`.
V82: dialects `sh`, `bash`, `zsh` (`dash`/`ksh` ? as sh-family): `env.dialect` from context (`sh -c`, `bash -c`, `zsh -c`, shebang, GH `shell:`, nix systemd `script`; host declares) & `env.options` = effective `set -o`/`setopt` state; prelude reproduces both exactly ∴ semantics preserved. grammar: tree-sitter-bash ∀ sh & bash, zsh best-effort ? (unsupported construct → `Judgment`).
V137: shell `claims` ⊥ `*.bats` (`languages:V130`, `languages/bats:V134`). measured 2026-09-21: tree-sitter-bash PARSES `@test "x" { run echo hi }` as command + brace group & the classifier calls it `sequence` ∴ a claimed `*.bats` file reads as a script w/ control flow & gets offered for extraction — confident & wrong, which no error message would have been.
V138: zsh-only syntax (flags `${(f)x}`, anon fn `() { print hi }`, glob qualifier `*(.)`) ⊥ parsed by tree-sitter-bash ∴ classifier ! return `Judgment` (`languages:V132`), ⊥ `Err`, ⊥ `host-parse-error`: the body is valid zsh & the gap is OURS. `Classification` carries a 3rd state ∴ engine reports `Judgment` w/ why `zsh construct unsupported`.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end | T11, T15, T53, T83, T135, T136, T149 | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |

id|status|task|cites
T11|x|shell single-command classifier on tree-sitter-bash AST (shared by all shell sinks)|V3,`languages:V2`
T15|.|host bash: heredoc-to-interpreter, `-c`/`-e` args, awk/jq threshold ? + fixtures|`languages:V2`,`tests:V14`,`tests:V15`
T53|x|shell `Guest::prelude`/`executable`/`invoke` defaults + fixture proving extract passes shellcheck|V51
T83|.|dialect & option capture per shell host context; fixtures: `sh -c`, `bash -c` under `set -e`, `zsh -c` w/ `setopt`|V82
T135|.|`claims` ∀ shell excl. `*.bats`; fixtures: `.bats` file ⊥ claimed, `.sh` & shebang-only file claimed|V137,`languages:V130`
T136|.|`Judgment` state in `Classification` ∀ unsupported zsh construct (fixes B1); fixtures: `setopt` stays simple, flags, anon fn, glob qualifier|V138,V82,`languages:V132`
T149|x|`src:C139` backfill: `languages/shell/src/classify/tests.rs`, `languages/shell/src/guest/tests.rs`|`src:C139`,`scripts/guard:V140`

## §B BUGS

id|date|cause|fix
B1|2026-09-21|`classify` returns `Err` ∀ zsh-only syntax (`setopt err_exit`, `() { print hi }`) ∵ tree-sitter-bash ⊥ parse it, while V82 says unsupported zsh construct → `Judgment`. shipped in `c103f5b`: 2-state `Classification` (simple \| constructs) had ⊥ 3rd state to return ∴ Err was the only exit|V138,T136
B2|2026-09-26|`classify("cat <<< hi")` → simple ∵ tree-sitter-bash parses `<<<` as `herestring_redirect`, ⊥ `file_redirect`, & `construct_of` had ⊥ row for it ∴ V3 "⊥ redirect" missed herestrings. found by the `src:C139` backfill (`classify/tests.rs`); fix: `herestring_redirect` → `redirect`|V3,T149
B3|2026-09-26|sh site w/ options = [`pipefail`] → prelude `set -` ∵ `set_line` drops `pipefail` for sh (V51, ⊥ POSIX) & nothing is left, yet `strict_line` still returned `Some`; `set -` ⊥ no-op (bash: turns `-v`/`-x` off). found by the `src:C139` backfill (`guest/tests.rs`); fix: sh w/ only `pipefail` → ⊥ strict line|V51,V82,T149
