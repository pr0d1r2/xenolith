# SPEC

## §G GOAL

language registry (`src/registry.rs`): the ONE file naming language crates & `lang-*` features; `hosts()`, `guests()`.

## §N NAV

rel|path|lens
up|.|-
up|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
self|src/registry|language registry: `hosts()`, `guests()`, feature gates, feature names
sib|src/config|`xenolith.toml` parse & validation
sib|src/extract|embed → own file, host rewrite, diff \| `--write`
sib|src/graph|host → extract load edges, dangling & orphan
sib|src/lint|per-language linter map & invocation
sib|src/cli|verbs, flags, exit codes, rule ids, output formats, hk wiring
sib|src/check|check engine: candidates → claims → sites → guests → violations; unclaimed & missing-guest policy, parallel scan
sib|src/discover|candidate discovery: `git ls-files` \| named paths, normalisation, symlink screen

## §V INVARIANTS
V41: registry = ONE file in root crate: `hosts() -> &'static [&'static dyn Host]`, `guests() -> &'static [&'static dyn Guest]`, each entry behind `#[cfg(feature = "lang-<lang>")]`, sorted by `LangId`. engines iterate registry; ⊥ `cfg(feature = "lang-*")` elsewhere (`src:V30` no-leak made checkable).

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end |  | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |

id|status|task|cites

## §B BUGS

id|date|cause|fix
B11|2026-09-26|`ShellHost` shipped (`languages/shells/shell:T15`) ⊥ in `HOSTS` ∴ ∀ `.sh` unclaimed (`src/check:V13`) ∴ ⊥ scanned, dogfood blind to scripts|register behind `lang-shell` (V41); `scripts/guard/tdd-order.sh` `test\(*` → `'test('*` (tree-sitter-bash ERROR)
