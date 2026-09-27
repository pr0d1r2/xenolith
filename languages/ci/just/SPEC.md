# SPEC

## §G GOAL

crate `xenolith-lang-just` (feature `lang-just`): tree-sitter-just; host: recipe bodies (shell by default, shebang recipe → shebang's guest), load idiom `bash <rel>/x.sh`.

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
up|languages/ci|hub: build, CI & config hosts -- nix, pkl, just, yaml, dockerfile
self|languages/ci/just|just parser, recipe sinks, load idiom
sib|languages/ci/nix|nix parser, sinks, load idiom
sib|languages/ci/pkl|pkl parser, hk step sinks, load idiom
sib|languages/ci/yaml|yaml parser, GH Actions sinks, placement
sib|languages/ci/dockerfile|Dockerfile parser, `RUN` sinks, placement

## §I INTERFACES

- sinks: recipe body (∀ lines of 1 recipe, line prefixes `@` & `-` stripped) → guest shell, dialect per V179; shebang recipe (`#!` 1st body line) → guest by shebang (`languages/shebang`), whole body 1 site; `[script]` recipe → guest by `set script-interpreter` ?. `{{…}}` interpolations = holes (`languages/api/src/holes:V40`); recipe params pass as args.
- load after extract: recipe body → ONE line `<guest invoke> <path> <args>` (`languages/api:V35`), e.g. `bash scripts/x.sh {{args}}`; path relative to justfile dir (just's default cwd, `languages:V74` `HostDir`).
- placement ? (T184): `scripts/<recipe>.<ext>` (fleet shape, R178) vs prototype `scripts/just/<recipe>.<ext>`.
- host checks: `just --fmt --check --unstable` (`languages/api` §I; `--unstable` still needed on the pinned just ?, T183).

## §R RESEARCH

id|topic|finding|src
R178|fleet 2026-09-27|34 repos / 188 justfiles; 19 w/ shebang recipes; 1 w/ `set shell`. fleet hook (xnl replaces it): files named exactly `justfile`, EVERY body line ! match an allowlist — `just --list`, `bash scripts/…`, `bats tests/…`, `expect tests/…`, `ssh -t u@h …` ∴ fleet extracts already load as `bash scripts/<path>`|read-only fleet survey, counts only (`scripts/guard` C17)

## §V INVARIANTS

V58: `claims`: filename `justfile` (case-insensitive), `.justfile`, extension `.just`.
V179: dialect: `set shell := [...]` STATICALLY readable (string list literal) → argv[0] basename = dialect & its flags = `env.options` (`languages/shells/shell:V82`); absent → just's default `sh -cu`; ⊥ readable (expr, `set windows-shell` only) → `Judgment`.
V180: recipe w/o shebang runs LINE BY LINE, each line a fresh shell ∴ body trivial iff 1 line & that line single simple command (`languages/shells/shell:V3`); ≥2 lines = violation. extract ⊥ merges lines blindly: `-` line → `… \|\| true`, prelude `set -eu` (just stops at 1st failing line, `-u` from `sh -cu`); line changing shell state a later line reads (`cd`, `export`, assignment, `set`) → `Judgment` ⊥ `Mechanical`.
V181: fleet allowlist (R178) is STRICTER than V180 — any single simple command passes here. replacement = V180 + `[threshold.just]` ? (T185); ⊥ silent loosening: `xnl` docs name the delta.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M3 | publication -- just, xml, tcl | T16, T183-T185 | each host claims its files & extracts w/ fixtures (`languages:V56`) |

id|status|task|cites
T16|.|host just (`tree-sitter-just` crates.io 0.2.0, casey, MIT; `languages:V121`): recipe sinks + fixtures: 1-line simple ⊥ flagged, 2 lines flagged, `-`/`@` prefixes, shebang recipe → python guest, `set shell := ["bash", "-uc"]` → bash, `{{param}}` hole, `cd` then cmd → `Judgment`|V58,V179,V180,`languages/shells/shell:V3`,`tests:V14`,`tests:V15`
T183|.|host checks: measure `just --fmt --check` w/ & w/o `--unstable` on the pinned just; record & drop the `?`|`languages/api` §I,`src/lint:V8`
T184|.|DECIDE by 2026-10-15 (before M3 cut): placement dir — count fleet `bash scripts/…` load paths (R178, counts only) → promote winner to a V; else keep prototype|`languages:T86`,R178
T185|.|DECIDE by 2026-10-15: `[threshold.just] commands` allowlist (fleet parity) or document V180 as the replacement; fixture per choice|V181,R178,`src/config:V55`

## §B BUGS

id|date|cause|fix
