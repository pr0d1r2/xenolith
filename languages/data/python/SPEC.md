# SPEC

## §G GOAL

crate `xenolith-lang-python` (feature `lang-python`): tree-sitter-python; guest (bash heredoc & `-c`, just shebang recipe, nix `writers.writePython3`); host ? later (embedded sql & shell).

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
up|languages/data|hub: data, text & markup -- python, sql, jq, awk, perl, xml
self|languages/data/python|python grammar, guest rules
sib|languages/data/sql|sql grammar, guest rules
sib|languages/data/jq|jq grammar, guest rules
sib|languages/data/awk|awk grammar, guest rules
sib|languages/data/perl|perl grammar, guest rules
sib|languages/data/xml|xml parser, launchd argv sinks, `xmllint` check

## §V INVARIANTS

V59: python guest: `trivial` = single expression statement (an `assignment` the grammar files as one is ⊥ trivial), ⊥ `import`, ⊥ `def`/`class`; else ⊥ constructs named (`languages/api:V37`) ∴ judged by `[threshold.python]` `max_lines`/`max_bytes`; parse error → `Error::Parse` (`languages:V77`); `prelude` = shebang `#!/usr/bin/env python3`, strict ⊥; ext `py`; `invoke` = `python3 {path}`; checks & fixers per `src/lint` §I defaults, their tools in `nix/tools.nix` `python`. guest ONLY: a `.py` file stays unclaimed, and an unclaimed-file message ⊥ names `lang-python` (`src/registry` `GUEST_ONLY`).

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
id|status|task|cites

## §B BUGS

id|date|cause|fix
