# SPEC

## §G GOAL

crate `xenolith-lang-pkl` (feature `lang-pkl`): pkl grammar; hk step sinks, `bash scripts/hk/x.sh {{files}}` load idiom.

## §I INTERFACES

- sinks: hk step `check`, `fix`, `shell`, `check_diff`, `check_list_files` → guest shell; load after extract: `bash scripts/hk/x.sh {{files}}`.
- site `env` = shell hk runs the step under (V172).

## §V INVARIANTS

V52: pkl host placement: hk step site → name = step key (kebab), dir = `scripts/hk`, load = `bash scripts/hk/<name>.sh {{files}}` (hk passes files through).
V171: write side: `escape` = inverse of `unescape` under the delim's own `#` count (`languages/api/src/lens:V39`); `inline` writes the host's line break (CRLF host → CRLF, B2); `rewrite` `Unsupported` ∀ holes (`languages/api/src/holes:V40` params ⊥ built ∴ `\(…)` would land in the script as text).
V172: hk step site `env` = what hk runs it under (`languages/shell:V82`), ⊥ guest default: step `shell`, else enclosing `Group` `shell` (`Hook` ⊥ has one), iff plain string `<sh\|bash\|zsh> [-<set letters>\|-o <name>]… -c` → its dialect & options; ⊥ set \| unreadable → hk default `sh -o errexit -c` (`pkl/Config.pkl` `Step.shell`; hk 1.58.1 argv) = `sh` + [`errexit`] ∴ prelude `#!/usr/bin/env sh` + `set -e`, ⊥ `set -euo pipefail` (B3).

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end | T13, T54, T147, T171, T172 | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |

id|status|task|cites
T13|x|host pkl (vendored `apple/tree-sitter-pkl`, Apache-2.0, `languages:V121`): hk step sinks, fixtures|`languages:V2`,`languages/shell:V3`,`tests:V14`,`tests:V15`
T54|x|pkl `Host::placement` for hk steps + fixture (`{{files}}` forwarded)|V52
T147|x|`src:C139` backfill: `languages/pkl/src/host/tests.rs`, `languages/pkl/src/string/tests.rs`|`src:C139`,`scripts/guard:V140`
T171|x|`escape` + `escape_law` ∀ fixture site; `inline` keeps CRLF; `rewrite` refuses holes|V171,B2,`languages/api/src/lens:V39`
T172|.|site `env` from hk's shell: default, step, `Group`, unreadable; fixture|V172,B3,`languages/shell:V82`

## §B BUGS

id|date|cause|fix
B1|2026-09-26|`unescape` required `\n` after opening `"""` ∴ CRLF hk.pkl → every step `unparseable pkl string`|`string::line_breaks`: `\r\n`\|`\r`\|`\n` = 1 break → `\n`, as Pkl; spans on host bytes; LF≡CRLF fixture
B2|2026-09-26|`inline` wrote `multiline`'s LF lines into a CRLF hk.pkl ∴ mixed line endings in the host|T171: line break read off the host, used ∀ line written
B3|2026-09-27|site `env` = `GuestEnv::default()` ∴ extract prelude `set -euo pipefail` (`languages/shell:V51`) ∀ hk step, which hk ran as `sh -o errexit -c`: `nounset`/`pipefail` added, dialect bash|V172,T172
