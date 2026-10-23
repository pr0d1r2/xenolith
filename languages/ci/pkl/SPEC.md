# SPEC

## §G GOAL

crate `xenolith-lang-pkl` (feature `lang-pkl`): `pklr` (pure Rust, V356), ⊥ grammar, ⊥ C, ⊥ `unsafe`; hk step sinks, `sh scripts/hk/x.sh {{files}}` load idiom (V52).

## §I INTERFACES

- sinks: hk step `check`, `fix`, `shell`, `check_diff`, `check_list_files` → guest shell; load after extract: `<interpreter> scripts/hk/x.sh {{files}}` (V52).
- site `env` = shell hk runs the step under (V172).

## §V INVARIANTS

V356: parser = `pklr` =5.2.1, default features OFF (lexer + parser only, deps `libm` `thiserror`): its parser decides validity (≡ Pkl 0.31.1 ≡ hk 2.x; measured 2026-10-23 on 77 fleet files, 14 rejected by all three) ⇒ ⊥ valid ⇒ `Error::Parse`, ⊥ partial sites (`languages:V78`). pklr's AST has ⊥ positions ∴ `src/syntax.rs` builds an object tree from pklr's TOKENS (byte offsets): properties (`local` flag), `[key]` entries (plain key), `{...}` vs `new T {...}` (only an entry's in-place `{...}` is a step body, as before), string literals (delimiters, `#` count, plain content, `\(...)` holes from the token groups); other expressions skipped (`src/syntax/expr.rs`), their objects kept. `string::unescape` re-lexes the body w/ pklr. dogfood 2026-10-23: 63 valid fleet files, reports identical to the tree-sitter host (43 violations, 8 files).
V52: pkl host placement: hk step site → name = step key (kebab), dir = `scripts/hk`, load = `<interpreter> ./scripts/hk/<name>.sh {{files}}` (hk passes files through), interpreter = the extract prelude's (V172; default `sh`, ⊥ `bash`: ⊥ a shell the step never ran under), set by the engine's `invoke` (`src/extract` §I); `loads` reads `sh`\|`bash`\|`zsh`.
V171: write side: `escape` = inverse of `unescape` under the delim's own `#` count (`languages/api/src/lens:V39`); `inline` writes the host's line break (CRLF host → CRLF, B2); holes: `rewrite_bound` ⊥ (default `Unsupported`, `languages/api/src/lens` §I) ∴ refused, ⊥ `\(...)` landing in the script as text.
V172: hk step site `env` = what hk runs it under (`languages/shells/shell:V82`), ⊥ guest default: step `shell`, else enclosing `Group` `shell` (`Hook` ⊥ has one), iff plain string `<sh\|bash\|zsh> [-<set letters>\|-o <name>]... -c` → its dialect & options; ⊥ set \| unreadable → hk default `sh -o errexit -c` (`pkl/Config.pkl` `Step.shell`; hk 1.58.1 argv) = `sh` + [`errexit`] ∴ prelude `#!/usr/bin/env sh` + `set -e`, ⊥ `set -euo pipefail` (B3).

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end | , | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |

id|status|task|cites

## §B BUGS

id|date|cause|fix
B1|2026-09-26|`unescape` required `\n` after opening `"""` ∴ CRLF hk.pkl → every step `unparseable pkl string`|`string::line_breaks`: `\r\n`\|`\r`\|`\n` = 1 break → `\n`, as Pkl; spans on host bytes; LF≡CRLF fixture
B2|2026-09-26|`inline` wrote `multiline`'s LF lines into a CRLF hk.pkl ∴ mixed line endings in the host|T171: line break read off the host, used ∀ line written
B3|2026-09-27|site `env` = `GuestEnv::default()` ∴ extract prelude `set -euo pipefail` (`languages/shells/shell:V51`) ∀ hk step, which hk ran as `sh -o errexit -c`: `nounset`/`pipefail` added, dialect bash|V172,T172
B4|2026-10-23|the vendored tree-sitter-pkl accepted what Pkl rejects: a `///` doc comment inside an object (before an entry or a step property), a whitespace line shorter than a multi-line string's indent ∴ `xnl` read sites through files hk 2.x and Pkl 0.31.1 refuse to load; 14 of 77 fleet files, 1 own fixture|V356: validity is pklr's; fixture `inert-strings` fixed (doc comment attached to a member); string tests now expect the short-whitespace line refused
