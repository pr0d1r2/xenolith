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
sib|languages/yaml|yaml parser, GH Actions sinks, placement
sib|languages/dockerfile|Dockerfile parser, `RUN` sinks, placement
sib|languages/shebang|shebang parse/strip/wrap ∀ guest
sib|languages/rust|rust parser, rust host sinks
sib|languages/ruby|ruby parser, ruby host sinks
sib|languages/html|html parser, inline script/style sinks

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
