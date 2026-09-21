# SPEC

## §G GOAL

crate `xenolith-lang-python` (feature `lang-python`): tree-sitter-python; guest (bash heredoc & `-c`, just shebang recipe, nix `writers.writePython3`); host ? later (embedded sql & shell).

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
self|languages/python|python grammar, guest rules
sib|languages/api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness
sib|languages/nix|nix parser, sinks, load idiom
sib|languages/pkl|pkl parser, hk step sinks, load idiom
sib|languages/shell|bash parser & host sinks, single-command classifier, shell linters
sib|languages/just|just parser, recipe sinks, load idiom
sib|languages/sql|sql grammar, guest rules
sib|languages/jq|jq grammar, guest rules
sib|languages/awk|awk grammar, guest rules
sib|languages/bats|bats grammar (based-on shell), `@test` sinks, test-host rules
sib|languages/yaml|yaml parser, GH Actions sinks, placement
sib|languages/dockerfile|Dockerfile parser, `RUN` sinks, placement
sib|languages/shebang|shebang parse/strip/wrap ∀ guest
sib|languages/rust|rust parser, rust host sinks
sib|languages/ruby|ruby parser, ruby host sinks
sib|languages/html|html parser, inline script/style sinks
sib|languages/js|javascript grammar, guest rules
sib|languages/css|css grammar, guest rules
sib|languages/perl|perl grammar, guest rules

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
