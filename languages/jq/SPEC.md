# SPEC

## §G GOAL

crate `xenolith-lang-jq` (feature `lang-jq`): tree-sitter jq grammar; guest (bash `jq` filter arg, nix `jq` calls in scripts).

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
self|languages/jq|jq grammar, guest rules
sib|languages/api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness
sib|languages/nix|nix parser, sinks, load idiom
sib|languages/pkl|pkl parser, hk step sinks, load idiom
sib|languages/shell|bash parser & host sinks, single-command classifier, shell linters
sib|languages/just|just parser, recipe sinks, load idiom
sib|languages/python|python grammar, guest rules
sib|languages/sql|sql grammar, guest rules
sib|languages/awk|awk grammar, guest rules

## §V INVARIANTS

V61: jq guest: `trivial` = filter ⊥ `def`, within `[threshold.jq]`; `prelude` = shebang `#!/usr/bin/env -S jq -f`, strict ⊥; ext `jq`; `invoke` = `jq -f {path}`; linter ⊥ (grammar parse in `check` suffices).

## §T TASKS

id|status|task|cites
T61|.|jq `Guest` + fixtures (`.foo` inline, filter w/ `def` flagged)|V61

## §B BUGS

id|date|cause|fix
