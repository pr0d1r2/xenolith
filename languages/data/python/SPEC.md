# SPEC

## §G GOAL

crate `xenolith-lang-python` (feature `lang-python`): tree-sitter-python; guest (bash heredoc & `-c`, just shebang recipe, nix `writers.writePython3`); host ? later (embedded sql & shell).

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
up|languages/data|hub: data & text guests -- python, sql, jq, awk, perl
self|languages/data/python|python grammar, guest rules
sib|languages/data/sql|sql grammar, guest rules
sib|languages/data/jq|jq grammar, guest rules
sib|languages/data/awk|awk grammar, guest rules
sib|languages/data/perl|perl grammar, guest rules

## §V INVARIANTS

V59: python guest: `trivial` = single expression statement, ⊥ `import`, ⊥ `def`/`class`, within `[threshold.python]`; `prelude` = shebang `#!/usr/bin/env python3`, strict ⊥; ext `py`; `invoke` = `python3 {path}`; checks & fixers per `src/lint` §I defaults.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M5 | data languages -- python, sql, jq, awk | T59 | each guest lands w/ its trivial rule, checks & fixtures |

id|status|task|cites
T59|.|python `Guest` + fixtures (trivial one-liner inline, multi-statement flagged)|V59

## §B BUGS

id|date|cause|fix
