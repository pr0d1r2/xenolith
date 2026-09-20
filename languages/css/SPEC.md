# SPEC

## §G GOAL

crate `xenolith-lang-css` (feature `lang-css`): tree-sitter-css; guest (html inline `<style>`, `style=` attrs ?).

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
self|languages/css|css grammar, guest rules
sib|languages/api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness
sib|languages/nix|nix parser, sinks, load idiom
sib|languages/pkl|pkl parser, hk step sinks, load idiom
sib|languages/shell|bash parser & host sinks, single-command classifier, shell linters
sib|languages/just|just parser, recipe sinks, load idiom
sib|languages/python|python grammar, guest rules
sib|languages/sql|sql grammar, guest rules
sib|languages/jq|jq grammar, guest rules
sib|languages/awk|awk grammar, guest rules
sib|languages/yaml|yaml parser, GH Actions sinks, placement
sib|languages/dockerfile|Dockerfile parser, `RUN` sinks, placement
sib|languages/shebang|shebang parse/strip/wrap ∀ guest
sib|languages/rust|rust parser, rust host sinks
sib|languages/ruby|ruby parser, ruby host sinks
sib|languages/html|html parser, inline script/style sinks
sib|languages/js|javascript grammar, guest rules
sib|languages/perl|perl grammar, guest rules

## §V INVARIANTS

V124: css guest: `trivial` = single declaration block ≤ `[threshold.css]`; `prelude` ⊥; ext `css`; `invoke` ⊥ (css is loaded, ⊥ run) ∴ load idiom is the host's `<link rel=stylesheet>`; checks & fixers per `src/lint` §I defaults.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M6 | web languages -- html (js & css guests) | T123 | inline `<script>`/`<style>` extraction w/ fixtures |

id|status|task|cites
T123|.|css `Guest` + fixtures (one rule inline, stylesheet flagged)|V124

## §B BUGS

id|date|cause|fix
