# SPEC

## §G GOAL

crate `xenolith-lang-jq` (feature `lang-jq`): tree-sitter jq grammar; guest (bash `jq` filter arg, nix `jq` calls in scripts).

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
up|languages/data|hub: data, text & markup -- python, sql, jq, awk, perl, xml
self|languages/data/jq|jq grammar, guest rules
sib|languages/data/python|python grammar, guest rules
sib|languages/data/sql|sql grammar, guest rules
sib|languages/data/awk|awk grammar, guest rules
sib|languages/data/perl|perl grammar, guest rules
sib|languages/data/xml|xml parser, launchd argv sinks, `xmllint` check

## §V INVARIANTS

V61: jq guest: `trivial` = filter ⊥ `def`, within `[threshold.jq]`; `prelude` = shebang `#!/usr/bin/env -S jq -f`, strict ⊥; ext `jq`; `invoke` = `jq -f {path}`; checks & fixers per `src/lint` §I defaults.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M5 | data languages -- python, sql, jq, awk | T61 | each guest lands w/ its trivial rule, checks & fixtures |

id|status|task|cites
T61|.|jq `Guest` + fixtures (`.foo` inline, filter w/ `def` flagged)|V61

## §B BUGS

id|date|cause|fix
