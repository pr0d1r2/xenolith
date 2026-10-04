# SPEC

## §G GOAL

crate `xenolith-lang-shell` (`lang-shell`): tree-sitter-bash host (heredoc, `-c`/`-e`), single-command classifier ∀ shell sink, load idiom, linters shellcheck + shfmt, zsh `zsh -n`.

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
up|languages/shells|hub: shell family -- shell, bats, tcl
self|languages/shells/shell|bash parser & host sinks, single-command classifier, shell linters
sib|languages/shells/bats|bats grammar (based-on shell), `@test` sinks, test-host rules
sib|languages/shells/tcl|tcl grammar (expect dialect), `exec`/`spawn` sinks, guest rules

## §I INTERFACES

- `claims`: `*.sh`, `*.bash`, `.envrc`, shell shebang (`shebang::resolves_to`); ⊥ `*.bats` (V137), ⊥ zsh shebang (V310).
- host `checks` shellcheck + shfmt, `fixers` shfmt: sh-family only (V310).
- sinks: heredoc to interpreter (`python`, `ruby`, `psql`, `tclsh`\|`wish`\|`expect`), `-c`/`-e` args (`python -c`, `ruby -e`, `node -e`, `perl -e`, `sh -c`, `bash -c`, `expect -c`), awk/jq program > threshold ? → guest python \| ruby \| sql \| js \| perl \| tcl (`languages/shells/tcl:V197`) \| awk \| jq; load: `python scripts/x.py`, `jq -f x.jq`, `awk -f x.awk`.
- `[extract.shell] strict` ∈ `preserve` (default) \| `enforce`: `enforce` → prelude `set -euo pipefail` always, diff marks it `Judgment`.
- placement ? (T86): bash host → `<host_dir>/<host_stem>.<name>.<ext>`, load via `"$(dirname "${BASH_SOURCE[0]}")/…"` (V3).

## §V INVARIANTS

V3: sink w/ single simple command (argv, optional leading `NAME=value`; generated load may use exactly `"$(dirname "${BASH_SOURCE[0]}")"` as path prefix; else ⊥ `|`, `&&`, `\|\|`, `;`, `$(`, backtick, redirect, `if`/`for`/`while`/`case`, heredoc, subshell, function def) = allowed. ≥1 control construct = violation unless ∈ `[threshold.shell] allow` (`src/config` §I). via tree-sitter-bash AST, ⊥ substring grep.
V51: shell guest defaults: `prelude(env)` = `#!/usr/bin/env <dialect>` + `set`/`setopt` reproducing `env.options` (V82); ⊥ context → bash + `set -euo pipefail`; executable; ext `sh` (zsh → `zsh`); `invoke` = `<dialect> {path}`.
V82: dialects `sh`, `bash`, `zsh` (`dash`/`ksh` ? as sh-family): `env.dialect` from context (`sh -c`, `bash -c`, `zsh -c`, shebang, GH `shell:`, nix systemd `script`) & `env.options` = effective `set -o`/`setopt` state; prelude reproduces both. zsh-only construct → `Judgment` (V138).
V137: shell `claims` ⊥ `*.bats` (`languages:V130`, `languages/shells/bats:V134`). measured 2026-09-21: tree-sitter-bash PARSES `@test "x" { run echo hi }` as command + brace group ∴ claimed `*.bats` = script offered for extraction: confident & wrong.
V138: zsh-only syntax (`${(f)x}`, `() { print hi }`, `*(.)`) ⊥ parsed by tree-sitter-bash ∴ classifier → `Judgment` `zsh construct unsupported` (`languages/shells:V132`), ⊥ `Err`, ⊥ `host-parse-error`: valid zsh, the gap is OURS.
V139: site ⇐ plain interpreter name; heredoc iff ⊥ program arg (`jq`/`awk` stdin = data; tcl: `-`\|`/dev/stdin` = stdin); `-c`/`-e` arg `'…'` \| `"…"` w/o `\`; env ← ITS argv, ⊥ enclosing `set`.
V310: `claims` ⊥ shebang interpreter `zsh`, before ext: tree-sitter-bash ⊥ reads zsh & `Host::checks` ⊥ sees the file ∴ zsh = extract only → `zsh -n` (`src/lint` §I). 2026-09-27: shellcheck 0.11.0 SC1071; shfmt 3.13.1 `-ln zsh` rejects 424/1229 zsh 5.9.1 `functions/*` (`zsh -n` ok), breaks 6/805; nixpkgs ∌ zsh linter.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end | , , | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |

id|status|task|cites

## §B BUGS

id|date|cause|fix
B1|2026-09-21|`classify` → `Err` ∀ zsh-only syntax ∵ 2-state `Classification` (`c103f5b`) ⊥ 3rd state; V82 says `Judgment`|V138,T136
B2|2026-09-26|`classify("cat <<< hi")` → simple ∵ `construct_of` ⊥ row for `herestring_redirect`; fix: → `redirect`|V3,T149
B3|2026-09-26|sh site w/ only `pipefail` → prelude `set -` (bash: turns `-v`/`-x` off) ∵ `set_line` drops `pipefail` for sh yet `strict_line` → `Some`; fix: ⊥ strict line|V51,V82,T149
B4|2026-09-26|zsh site w/ valid `${(f)x}` → `unparseable shell` ∵ classifier ⊥ saw `env.dialect`; fix: `classify_in`, zsh & grammar rejects → `unsupported`|V138,T136
B5|2026-09-26|dogfood `xnl lint`: 10 `.sh` fail shfmt (tabs) ∵ ANY parser/printer flag → shfmt 3.13.1 ignores `.editorconfig`; fix: bare `shfmt --diff`/`--write`|V51,`.:V19`
B6|2026-09-27|`xnl lint` fails ∀ zsh-shebang file (SC1071) & `check` → `host-parse-error` ∵ `claims` took zsh; fix: ⊥ claim|V310,T310
