# SPEC

## §G GOAL

crate `xenolith-lang-js` (feature `lang-js`): tree-sitter-javascript; guest (html inline `<script>`, bash `node -e`, ruby tagged heredoc).

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
self|languages/js|javascript grammar, guest rules
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
sib|languages/css|css grammar, guest rules
sib|languages/perl|perl grammar, guest rules

## §V INVARIANTS

V123: js guest: `trivial` = single expression statement, ⊥ `function`/`class`/`import`, within `[threshold.js]`; `prelude` ⊥ (browser context has no shebang; `node -e` extract gets `#!/usr/bin/env node` ?); ext `js`; `invoke` = `node {path}`; checks & fixers per `src/lint` §I defaults.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M6 | web languages -- html (js & css guests) | T122 | inline `<script>`/`<style>` extraction w/ fixtures |

id|status|task|cites
T122|.|js `Guest` + fixtures (inline handler flagged, one-expression `<script>` inline)|V123

## §B BUGS

id|date|cause|fix
