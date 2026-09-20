# SPEC

## §G GOAL

crate `xenolith-lang-just` (feature `lang-just`): tree-sitter-just; host: recipe bodies (shell by default, shebang recipe → shebang's guest), load idiom `bash <rel>/x.sh`.

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
self|languages/just|just parser, recipe sinks, load idiom
sib|languages/api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness
sib|languages/nix|nix parser, sinks, load idiom
sib|languages/pkl|pkl parser, hk step sinks, load idiom
sib|languages/shell|bash parser & host sinks, single-command classifier, shell linters
sib|languages/python|python grammar, guest rules
sib|languages/sql|sql grammar, guest rules
sib|languages/jq|jq grammar, guest rules
sib|languages/awk|awk grammar, guest rules
sib|languages/yaml|yaml parser, GH Actions sinks, placement
sib|languages/dockerfile|Dockerfile parser, `RUN` sinks, placement
sib|languages/shebang|shebang parse/strip/wrap ∀ guest
sib|languages/rust|rust parser, rust host sinks
sib|languages/ruby|ruby parser, ruby host sinks
sib|languages/html|html parser, inline script/style sinks

## §I INTERFACES

- sinks: recipe body > single simple command, shebang recipe → guest shell \| python \| …; load after extract: `bash scripts/x.sh`.
- placement prototype ? (T86 evaluates): just → `scripts/just/<recipe>.<ext>`, load `bash scripts/just/<recipe>.sh {{args}}` (recipe params → `languages/api/src/holes:V40` params).

## §V INVARIANTS

V58: `claims`: filename `justfile` (case-insensitive), `.justfile`, extension `.just`.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M4 | CI languages -- yaml, dockerfile, just | T16 | each host claims its files & extracts w/ fixtures (`languages:V56`) |

id|status|task|cites
T16|.|host just (tree-sitter-just): recipe sinks + fixtures|`languages/shell:V3`,`tests:V14`,`tests:V15`

## §B BUGS

id|date|cause|fix
