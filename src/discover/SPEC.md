# SPEC

## §G GOAL

candidate discovery (`src/discover.rs`): which files a verb looks at -- tracked set or named paths, symlinks screened.

## §N NAV

rel|path|lens
up|.|-
up|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
self|src/discover|candidate discovery: `git ls-files` \| named paths, normalisation, symlink screen
sib|src/config|`xenolith.toml` parse & validation
sib|src/extract|embed → own file, host rewrite, diff \| `--write`
sib|src/graph|host → extract load edges, dangling & orphan
sib|src/lint|per-language linter map & invocation
sib|src/cli|verbs, flags, exit codes, rule ids, output formats, hk wiring
sib|src/check|check engine: candidates → claims → sites → guests → violations; unclaimed & missing-guest policy, parallel scan
sib|src/registry|language registry: `hosts()`, `guests()`, feature gates, feature names

## §V INVARIANTS
V57: `xnl check|graph|lint` w/ ⊥ paths → candidates = `git ls-files` (tracked only ∴ `.gitignore` honoured); ⊥ git repo & ⊥ paths → exit 2 usage. named dir w/ 0 candidates → exit 2; candidate = regular file \| symlink (FIFO, socket, device ⊥ listed; named → exit 2); named paths normalised (root-relative, ⊥ `.`, `x/..` folded only past a real dir) before dedup.
V128: candidate that IS a symlink ⊥ scanned (warning `symlink-skipped`), & discovery ⊥ follows symlinked dirs — a tracked symlink may point outside the repo, & the same bytes would be reported twice under 2 paths. named explicitly → exit 2 saying so (`src/extract:V71`, `src/graph:V72` are the write & graph halves).

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end |  | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |

id|status|task|cites

## §B BUGS

id|date|cause|fix
B9|2026-09-26|named dir w/ 0 tracked files → empty scan, exit 0; walk kept FIFO ∴ engine `open` hung; `./a` & `a` deduped raw ∴ reported twice|V57: `EmptyDir` / `NotAFile` exit 2, regular files \| symlinks only, `normalise` before dedup
