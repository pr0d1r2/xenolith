# SPEC

## §G GOAL

crate `xenolith-lang-tcl` (feature `lang-tcl`): vendored tree-sitter-tcl; tcl (& expect dialect) as host (`exec`/`spawn` argv, `exec … <<`) & as guest (shell heredoc to `tclsh`, `expect -c`).

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
up|languages/shells|hub: shell family -- shell, bats, tcl
self|languages/shells/tcl|tcl grammar (expect dialect), `exec`/`spawn` sinks, guest rules
sib|languages/shells/shell|bash parser & host sinks, single-command classifier, shell linters
sib|languages/shells/bats|bats grammar (based-on shell), `@test` sinks, test-host rules

## §I INTERFACES

- host sinks (V196): `exec sh|bash|zsh|dash -c <word>` → shell; `exec <interp> << <word>` (Tcl's stdin-from-value redirection) → guest per interp; expect `spawn sh|bash -c <word>` → shell. `spawn` + `send` (interactive) ⊥ site; `open "\|sh -c …"` ?.
- body: braced `{…}` word verbatim; `"…"` word w/ `$var` \| `[cmd]` → holes (`languages/api/src/holes:V40`) ?, until fixture.
- guest: per V197; shell side = `languages/shells/shell:V139`, added by T200 ⊥ here.
- host checks ? (V198).

## §R RESEARCH

id|topic|finding|src
R193|fleet 2026-09-27|5 repos / 23 files (14 `.tcl`, 9 `.exp`); 13 use `exec`, 5 run `sh\|bash -c`, 8 use expect `spawn`. fleet hook (`*.tcl`, `*.exp`): `tclsh` reads the file, `info complete` (unclosed braces, brackets, quotes) + lint "`#` inside `set x { }` = literal, ⊥ comment" — itself a Tcl program fed to `tclsh` by a shell heredoc, i.e. a V197 site|read-only fleet survey, counts only (`scripts/guard` C17)
R194|grammar 2026-09-27|`tree-sitter-tcl` (tree-sitter-grammars, MIT) ⊥ on crates.io (404) ∴ VENDOR per `languages:V121`|crates.io API, upstream repo

## §V INVARIANTS

V195: `claims`: `*.tcl`, `*.tk`, `*.exp`, shebang resolving to `tclsh` \| `wish` \| `expect`. expect = DIALECT of tcl (same grammar, extra commands `spawn` `expect` `send` `interact`; `languages/shells:V132`) ∴ ⊥ own `LangId`, `env.dialect = expect` iff `.exp` \| expect shebang.
V196: host site ⇐ plain interp name as `exec`/`spawn` arg 1 (⊥ `$var`, ⊥ `[cmd]`); `-c` word → shell, dialect = interp, env from ITS argv (argv form of `languages/shells/shell:V139`); `<<` word → guest per interp; ⊥ other shape = site.
V197: guest tcl ⇐ shell heredoc to `tclsh` \| `wish` \| `expect` & `expect -c '…'` (`tclsh` has ⊥ `-c`). defaults: `invoke` = `tclsh {path}` \| `expect {path}`; ext `tcl` \| `exp`; prelude shebang `#!/usr/bin/env tclsh` \| `expect`, strict ⊥; `trivial` ? (default `max_lines`/`max_bytes`, `languages/api:V37`).
V198: checks ? (unconfirmed until measured, as `languages/shells/bats:V136`): fleet check (R193) vs `tclint` vs `nagelfar` — decide by MEASURING on the fleet's 23 shapes (counts only). fleet check needs a Tcl checker FILE shipped w/ `xnl` (an extract itself, dogfood `.:V19`). fixers ?.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M3 | publication -- just, xml, tcl | T199-T202 | host & guest tcl w/ fixtures, checks decided (`languages:V56`) |

id|status|task|cites
T199|.|scaffold `languages/shells/tcl` crate (vendored `tree-sitter-tcl`, `LangId` variant 1st per `languages/api:V33`): `claims` + V196 sinks; fixtures: `exec sh -c {a \| b}` flagged, `exec bash -c {ls}` ⊥ flagged, `exec python3 << $src` → python, `spawn sh -c` flagged, `spawn ssh` + `send` ⊥ site, `.exp` → dialect expect|V195,V196,R194,`tests:V14`,`tests:V15`
T200|.|shell side of V197: `tclsh`/`wish`/`expect` heredoc & `expect -c` as shell sinks → guest tcl; fixture = R193's shape (`tclsh /dev/stdin "$f" <<'TCL'`)|V197,`languages/shells/shell:V139`,`languages:V81`
T201|.|MEASURE the 3 check candidates (V198), record counts & shapes, promote the winner & drop the `?`|V198,`src/lint:V8`
T202|.|lookalike tcl~shell ? (`puts hi`, `set x 1` read as bash commands): measure; if confirmed add kinship edge + fixture pair|`languages:V131`,`languages/api:V33`

## §B BUGS

id|date|cause|fix
