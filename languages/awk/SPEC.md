# SPEC

## §G GOAL

crate `xenolith-lang-awk` (feature `lang-awk`): tree-sitter awk grammar; guest (bash `awk` program arg).

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
self|languages/awk|awk grammar, guest rules
sib|languages/api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness
sib|languages/nix|nix parser, sinks, load idiom
sib|languages/pkl|pkl parser, hk step sinks, load idiom
sib|languages/shell|bash parser & host sinks, single-command classifier, shell linters
sib|languages/just|just parser, recipe sinks, load idiom
sib|languages/python|python grammar, guest rules
sib|languages/sql|sql grammar, guest rules
sib|languages/jq|jq grammar, guest rules
sib|languages/yaml|yaml parser, GH Actions sinks, placement
sib|languages/dockerfile|Dockerfile parser, `RUN` sinks, placement

## §V INVARIANTS

V62: awk guest: `trivial` = single pattern-action, ⊥ `BEGIN`/`END`/`function`, within `[threshold.awk]`; `prelude` = shebang `#!/usr/bin/awk -f`, strict ⊥; ext `awk`; `invoke` = `awk -f {path}`; checks & fixers per `src/lint` §I defaults.

## §T TASKS

id|status|task|cites
T62|.|awk `Guest` + fixtures (`{print $1}` inline, BEGIN block flagged)|V62

## §B BUGS

id|date|cause|fix
