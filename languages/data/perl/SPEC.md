# SPEC

## §G GOAL

crate `xenolith-lang-perl` (feature `lang-perl`): tree-sitter-perl; guest (bash `perl -e`, `perl -ne` one-liners).

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
up|languages/data|hub: data & text guests -- python, sql, jq, awk, perl
self|languages/data/perl|perl grammar, guest rules
sib|languages/data/python|python grammar, guest rules
sib|languages/data/sql|sql grammar, guest rules
sib|languages/data/jq|jq grammar, guest rules
sib|languages/data/awk|awk grammar, guest rules

## §V INVARIANTS

V125: perl guest: `trivial` = single statement, ⊥ `sub`/`use`, within `[threshold.perl]`; `prelude` = shebang `#!/usr/bin/env perl`, strict ⊥ (`use strict; use warnings;` = the author's call, ⊥ ours); ext `pl`; `invoke` = `perl {path}`; checks per `src/lint` §I defaults (`perl -c`).

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M5 | data languages -- python, sql, jq, awk | T124 | each guest lands w/ its trivial rule, checks & fixtures |

id|status|task|cites
T124|.|perl `Guest` + fixtures (`perl -e` one-liner inline, multi-statement flagged)|V125

## §B BUGS

id|date|cause|fix
