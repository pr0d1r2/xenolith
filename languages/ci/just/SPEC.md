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

- sinks: recipe body (∀ lines of 1 recipe, line prefixes `@` & `-` stripped) → guest shell, dialect per V179; shebang recipe (`#!` 1st body line) → guest by shebang (`languages/shebang`), whole body 1 site; `[script]` recipe → guest by `set script-interpreter` ? (until then ⊥ site). `{{…}}` interpolations = holes (`languages/api/src/holes:V40`); recipe params pass as args.
- load after extract: recipe body → ONE line `<guest invoke> <path> <args>` (`languages/api:V35`), e.g. `bash scripts/x.sh {{args}}`; path relative to justfile dir (just's default cwd, `languages:V74` `HostDir`); `@` on ∀ lines → `@` load. until decided → refused (`Judgment`): holes, shebang recipe, params under `positional-arguments`, B1.
- dialect ⊥ readable (V179) also: `import` w/o own `set shell`, argv[0] ∉ sh\|bash\|zsh, unknown flag, escaped or prefixed (`x"…"`, `f"…"`) string.
- grammar: crate 0.2.0 pins tree-sitter ~0.25, `links` clash w/ 0.27 ∴ C VENDORED per `languages:V121`: upstream rev of 0.2.0 (newest, 2026-09-27) + LOCAL PATCH of `grammar.js` for just 1.51 (B3), regenerated; patch, CLI & ABI in `vendor/tree-sitter-just/UPSTREAM`.
- placement (T184): `{host_dir}/scripts/just/<recipe>.<ext>` — under the fleet's `scripts/` (R178), `just/` keeps each host's extracts apart.
- host checks: `just --fmt --check --justfile <file>`, fixer `just --fmt --justfile <file>` (`languages/api` §I); ⊥ `--unstable`: pinned just 1.51.0 gives the same verdict w/ & w/o it (T183).

## §R RESEARCH

id|topic|finding|src
R178|fleet 2026-09-27|34 repos / 188 justfiles; 19 w/ shebang recipes; 1 w/ `set shell`. fleet hook (xnl replaces it): files named exactly `justfile`, EVERY body line ! match an allowlist — `just --list`, `bash scripts/…`, `bats tests/…`, `expect tests/…`, `ssh -t u@h …` ∴ fleet extracts already load as `bash scripts/<path>`|read-only fleet survey, counts only (`scripts/guard` C17)
R207|V181 delta 2026-09-27|xnl vs the fleet hook, measured w/ `xnl check`: LOOSER — 1 simple command off the allowlist passes (`cargo build`, `rm -rf dist`), so does `#!` + 1 command (shebang ⊥ allowlisted there); STRICTER — ≥2 lines, even ALL allowlisted (`bash scripts/lint.sh` + `bats tests/unit`) flagged `sequence`, hook passes; WIDER — claims `Justfile`, `.justfile`, `*.just` (V58), hook `justfile` only. T185 keeps the delta, ⊥ parity; `[threshold.shell] allow` relaxes every shell site alike|fixture `pos-fleet-delta` (sites, rewrite), e2e `xnl check`

## §V INVARIANTS

V58: `claims`: filename `justfile` (case-insensitive), `.justfile`, extension `.just`.
V179: dialect: `set shell := [...]` STATICALLY readable (string list literal) → argv[0] basename = dialect & its flags = `env.options` (`languages/shells/shell:V82`); absent → just's default `sh -cu`; ⊥ readable (expr, `set windows-shell` only) → `Judgment`.
V180: recipe w/o shebang runs LINE BY LINE, each line a fresh shell ∴ body trivial iff 1 line & that line single simple command (`languages/shells/shell:V3`); ≥2 lines = violation, even ∀ line on the fleet allowlist (T185: justfiles = one-liners, the load after extract). extract ⊥ merges lines blindly: `-` line → `… \|\| true`, prelude `set -eu` (just stops at 1st failing line, `-u` from `sh -cu`); line changing shell state a later line reads (`cd`, `export`, assignment, `set`) → `Judgment` ⊥ `Mechanical`.
V181: V180 alone replaces the fleet allowlist (R178): ⊥ `[threshold.just]`, ⊥ command allowlist (T185). delta (R207): STRICTER on ≥2 lines, LOOSER on 1 simple command off the allowlist; ⊥ silent: `xnl` docs name it.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M3 | publication -- just, xml, tcl | T16, T183-T185 | each host claims its files & extracts w/ fixtures (`languages:V56`) |

id|status|task|cites
T16|x|host just (`tree-sitter-just` crates.io 0.2.0, casey, MIT; `languages:V121`): recipe sinks + fixtures: 1-line simple ⊥ flagged, 2 lines flagged, `-`/`@` prefixes, shebang recipe → python guest, `set shell := ["bash", "-uc"]` → bash, `{{param}}` hole, `cd` then cmd → `Judgment`|V58,V179,V180,`languages/shells/shell:V3`,`tests:V14`,`tests:V15`
T183|x|host checks: measure `just --fmt --check` w/ & w/o `--unstable` on the pinned just; record & drop the `?`|`languages/api` §I,`src/lint:V8`
T184|x|DECIDED 2026-09-27: placement dir = prototype `scripts/just/<recipe>.<ext>` (§I); ⊥ conflict w/ fleet `bash scripts/…` loads (R178)|`languages:T86`,R178
T185|x|DECIDED 2026-09-27: ⊥ `[threshold.just]` allowlist; V180 is the replacement, ≥2 lines always a violation (V181); fixture `pos-fleet-delta`|V181,R178,`src/config:V55`

## §B BUGS

id|date|cause|fix
B1|2026-09-27|V180's merge under `set -e` ⊥ exact: `a && b` \| `! a` on a non-last line (errexit skips both; just stops on the line's status) & `a; b` when the argv lacks `-e` (just ignores `a`)|refuse → `Judgment` (`state::escapes_errexit`, `state::sequences`); unit tests
B2|2026-09-27|grammar 0.2.0 lexes `{{{{` by context: `x{{{{y}}` → hole `{{y}}` just never evaluates|holes = nodes just's own lexer opens; `{{` ⊥ node → hole anyway (refused ⊥ merged); fixture `pos-holes`
B3|2026-09-27|grammar 0.2.0 rejects just 1.51 syntax: `f"…"`, `x"…"`, `[arg(…, pattern="\d+")]` ∴ file → `host-parse-error` (`languages:V78`); also `&&`/`\|\|`, `assert(…)`, `eager`, `[attr]` on an assignment, `unexport`|upstream main = 0.2.0's rev (2026-09-27) ∴ LOCAL PATCH of `grammar.js`, regenerated (`languages:V121` record); `pattern="\d+"` ⊥ valid just either (`\d` escape), `'\d+'` parsed already; fixture `pos-just-1-51`
