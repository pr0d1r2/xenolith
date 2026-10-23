# SPEC

## §G GOAL

crate `xenolith-lang-just` (feature `lang-just`): own pure-Rust justfile parser (V355), ⊥ grammar, ⊥ C, ⊥ `unsafe`; host: recipe bodies (shell by default, shebang recipe → shebang's guest), load idiom `bash <rel>/x.sh`.

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

- sinks: recipe body (∀ lines of 1 recipe, line prefixes `@` & `-` stripped) → guest shell, dialect per V179; shebang recipe (`#!` 1st body line) → guest by shebang (`languages/shebang`), whole body 1 site; `[script]` recipe → guest by `set script-interpreter` ? (until then ⊥ site). `{{...}}` interpolations = holes (`languages/api/src/holes:V40`); recipe params pass as args.
- load after extract: recipe body → ONE line `<guest invoke> <path> <args>` (`languages/api:V35`), e.g. `bash scripts/x.sh {{args}}`; path relative to justfile dir (just's default cwd, `languages:V74` `HostDir`); `@` on ∀ lines → `@` load. until decided → refused (`Judgment`): holes, shebang recipe, params under `positional-arguments`, B1.
- dialect ⊥ readable (V179) also: `import` w/o own `set shell`, argv[0] ∉ sh\|bash\|zsh, unknown flag, escaped or prefixed (`x"..."`, `f"..."`) string.
- parser (V355): `src/lexer.rs` (just's tokens) + `src/parse.rs` (items, settings, headers) + `src/parse/body.rs` (lines, prefixes, `{{...}}`) + `src/parse/expr.rs` (expressions). REPLACED 2026-10-23 the vendored tree-sitter-just (crate 0.2.0's `links` clash w/ tree-sitter 0.27 forced vendoring per `languages:V121`, plus a local patch for B3); grammar, patch & `build.rs` deleted.
- placement (T184): `{host_dir}/scripts/just/<recipe>.<ext>` -- under the fleet's `scripts/` (R178), `just/` keeps each host's extracts apart.
- host checks: `just --fmt --check --justfile <file>`, fixer `just --fmt --justfile <file>` (`languages/api` §I); ⊥ `--unstable`: pinned just 1.51.0 gives the same verdict w/ & w/o it (T183).

## §R RESEARCH

id|topic|finding|src
R178|fleet 2026-09-27|34 repos / 188 justfiles; 19 w/ shebang recipes; 1 w/ `set shell`. fleet hook (xnl replaces it): files named exactly `justfile`, EVERY body line ! match an allowlist -- `just --list`, `bash scripts/...`, `bats tests/...`, `expect tests/...`, `ssh -t u@h ...` ∴ fleet extracts already load as `bash scripts/<path>`|read-only fleet survey, counts only (`scripts/guard` C17)
R207|V181 delta 2026-09-27|xnl vs the fleet hook, measured w/ `xnl check`: LOOSER -- 1 simple command off the allowlist passes (`cargo build`, `rm -rf dist`), so does `#!` + 1 command (shebang ⊥ allowlisted there); STRICTER -- ≥2 lines, even ALL allowlisted (`bash scripts/lint.sh` + `bats tests/unit`) flagged `sequence`, hook passes; WIDER -- claims `Justfile`, `.justfile`, `*.just` (V58), hook `justfile` only. T185 keeps it as the DEFAULT, ⊥ parity: `[threshold.just] max_lines = 2` passes `check` (each line 1 simple command), 3 lines still flagged; `[threshold.shell] allow` relaxes every shell site alike|fixture `pos-fleet-delta` (sites, rewrite), e2e `xnl check`

## §V INVARIANTS

V58: `claims`: filename `justfile` (case-insensitive), `.justfile`, extension `.just`.
V355: parser = recursive descent over just's GRAMMAR.md + just 1.51 additions (`eager`, `unexport`, `x`/`f` strings, attributes on any item, key-value attribute args); ∀ recipe: header span (`@`? name params `:` deps, ⊥ trailing comment), `#!` first line apart, lines = text after the body's indent ⊥ `\r`, `@`/`-` prefix span, `{{...}}` holes lexed as just lexes them (`{{{{` literal, `{{` opens an EXPRESSION to its `}}`). settings typed per GRAMMAR (list \| string \| boolean; unknown name ⇒ value read, ⊥ refused). ⊥ what just rejects (unterminated string \| backtick, unknown escape, unknown char, bad setting value, indented item, inconsistent body indent, broken expression) ⇒ `Error::Parse` line:col (`languages:V78`). dogfood 2026-10-23: 204 fleet justfiles, 410 violations identical to the grammar's.
V179: dialect: `set shell := [...]` STATICALLY readable (string list literal) → argv[0] basename = dialect & its flags = `env.options` (`languages/shells/shell:V82`); absent → just's default `sh -cu`; ⊥ readable (expr, `set windows-shell` only) → `Judgment`.
V180: recipe w/o shebang runs LINE BY LINE, each line a fresh shell ∴ body inline iff ≤ `[threshold.just] max_lines` lines (default 1, T185) & EACH line alone a single simple command (`languages/shells/shell:V3`); over it = violation, even ∀ line on the fleet allowlist; `cd` inside the ceiling passes (its state ⊥ reaches the next line: just's semantics, ⊥ a size rule). extract ⊥ merges lines blindly: `-` line → `... \|\| true`, prelude `set -eu` (just stops at 1st failing line, `-u` from `sh -cu`); line changing shell state a later line reads (`cd`, `export`, assignment, `set`) → `Judgment` ⊥ `Mechanical`.
V181: V180 alone replaces the fleet allowlist (R178): 1 knob, `[threshold.just] max_lines` (`src/config:V240`), just-specific, per repo \| subtree; ⊥ command allowlist (T185). delta (R207) at the default: STRICTER on ≥2 lines, LOOSER on 1 simple command off the allowlist; ⊥ silent: `xnl` docs name it.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M3 | publication -- just, xml, tcl |  | each host claims its files & extracts w/ fixtures (`languages:V56`) |

id|status|task|cites

## §B BUGS

id|date|cause|fix
B1|2026-09-27|V180's merge under `set -e` ⊥ exact: `a && b` \| `! a` on a non-last line (errexit skips both; just stops on the line's status) & `a; b` when the argv lacks `-e` (just ignores `a`)|refuse → `Judgment` (`state::escapes_errexit`, `state::sequences`); unit tests
B2|2026-09-27|grammar 0.2.0 lexes `{{{{` by context: `x{{{{y}}` → hole `{{y}}` just never evaluates|holes = nodes just's own lexer opens; `{{` ⊥ node → hole anyway (refused ⊥ merged); fixture `pos-holes`. 2026-10-23 V355: the parser lexes `{{{{` & `{{` as just does, ⊥ grammar to second-guess
B3|2026-09-27|grammar 0.2.0 rejects just 1.51 syntax: `f"..."`, `x"..."`, `[arg(..., pattern="\d+")]` ∴ file → `host-parse-error` (`languages:V78`); also `&&`/`\|\|`, `assert(...)`, `eager`, `[attr]` on an assignment, `unexport`|upstream main = 0.2.0's rev (2026-09-27) ∴ LOCAL PATCH of `grammar.js`, regenerated (`languages:V121` record); `pattern="\d+"` ⊥ valid just either (`\d` escape), `'\d+'` parsed already; fixture `pos-just-1-51`; closed 2026-10-23 by V355: grammar & patch deleted, the parser reads just 1.51 natively
