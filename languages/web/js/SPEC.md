# SPEC

## §G GOAL

crate `xenolith-lang-js` (feature `lang-js`): tree-sitter-javascript; guest (html inline `<script>`, bash `node -e`, ruby tagged heredoc).

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
up|languages/web|hub: web host & its guests -- html, js, css
self|languages/web/js|javascript grammar, guest rules
sib|languages/web/html|html parser, inline script/style sinks
sib|languages/web/css|css grammar, guest rules

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
