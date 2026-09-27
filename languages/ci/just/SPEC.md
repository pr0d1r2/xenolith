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

- sinks: recipe body > single simple command, shebang recipe → guest shell \| python \| …; load after extract: `bash scripts/x.sh`.
- placement prototype ? (T86 evaluates): just → `scripts/just/<recipe>.<ext>`, load `bash scripts/just/<recipe>.sh {{args}}` (recipe params → `languages/api/src/holes:V40` params).

## §V INVARIANTS

V58: `claims`: filename `justfile` (case-insensitive), `.justfile`, extension `.just`.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M3 | publication -- just, xml, tcl | T16 | each host claims its files & extracts w/ fixtures (`languages:V56`) |

id|status|task|cites
T16|.|host just (tree-sitter-just): recipe sinks + fixtures|`languages/shells/shell:V3`,`tests:V14`,`tests:V15`

## §B BUGS

id|date|cause|fix
