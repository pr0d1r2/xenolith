# SPEC

## §G GOAL

crate `xenolith-lang-awk` (feature `lang-awk`): vendored `Beaglefoot/tree-sitter-awk` (MIT, `languages:V121`); guest (bash `awk` program arg).

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
up|languages/data|hub: data & text guests -- python, sql, jq, awk, perl
self|languages/data/awk|awk grammar, guest rules
sib|languages/data/python|python grammar, guest rules
sib|languages/data/sql|sql grammar, guest rules
sib|languages/data/jq|jq grammar, guest rules
sib|languages/data/perl|perl grammar, guest rules

## §V INVARIANTS

V62: awk guest: `trivial` = single pattern-action, ⊥ `BEGIN`/`END`/`function`, within `[threshold.awk]`; `prelude` = shebang `#!/usr/bin/awk -f`, strict ⊥; ext `awk`; `invoke` = `awk -f {path}`; checks & fixers per `src/lint` §I defaults.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M5 | data languages -- python, sql, jq, awk | T62 | each guest lands w/ its trivial rule, checks & fixtures |

id|status|task|cites
T62|.|awk `Guest` + fixtures (`{print $1}` inline, BEGIN block flagged)|V62

## §B BUGS

id|date|cause|fix
