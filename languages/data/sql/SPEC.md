# SPEC

## §G GOAL

crate `xenolith-lang-sql` (feature `lang-sql`): tree-sitter sql grammar; guest (bash `psql <<`, ruby `<<~SQL`, rust query literal ?).

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
up|languages/data|hub: data & text guests -- python, sql, jq, awk, perl
self|languages/data/sql|sql grammar, guest rules
sib|languages/data/python|python grammar, guest rules
sib|languages/data/jq|jq grammar, guest rules
sib|languages/data/awk|awk grammar, guest rules
sib|languages/data/perl|perl grammar, guest rules

## §V INVARIANTS

V60: sql guest: `trivial` = single statement, ⊥ `;`-chained, within `[threshold.sql]`; `prelude` empty (no shebang, strict ⊥); ext `sql`; `invoke` = `psql -f {path}` (host may override); checks & fixers per `src/lint` §I defaults.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M5 | data languages -- python, sql, jq, awk | T60 | each guest lands w/ its trivial rule, checks & fixtures |

id|status|task|cites
T60|.|sql `Guest` + fixtures (single SELECT inline, multi-statement flagged)|V60

## §B BUGS

id|date|cause|fix
