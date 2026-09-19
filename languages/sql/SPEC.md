# SPEC

## §G GOAL

crate `xenolith-lang-sql` (feature `lang-sql`): tree-sitter sql grammar; guest (bash `psql <<`, ruby `<<~SQL`, rust query literal ?).

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
self|languages/sql|sql grammar, guest rules
sib|languages/api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness
sib|languages/nix|nix parser, sinks, load idiom
sib|languages/pkl|pkl parser, hk step sinks, load idiom
sib|languages/shell|bash parser & host sinks, single-command classifier, shell linters
sib|languages/just|just parser, recipe sinks, load idiom
sib|languages/python|python grammar, guest rules
sib|languages/jq|jq grammar, guest rules
sib|languages/awk|awk grammar, guest rules

## §V INVARIANTS

V60: sql guest: `trivial` = single statement, ⊥ `;`-chained, within `[threshold.sql]`; `header` ⊥ (no shebang); ext `sql`; `invoke` = `psql -f {path}` (host may override); linter `sqlfluff lint`.

## §T TASKS

id|status|task|cites
T60|.|sql `Guest` + fixtures (single SELECT inline, multi-statement flagged)|V60

## §B BUGS

id|date|cause|fix
