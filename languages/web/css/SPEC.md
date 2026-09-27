# SPEC

## §G GOAL

crate `xenolith-lang-css` (feature `lang-css`): tree-sitter-css; guest (html inline `<style>`, `style=` attrs ?).

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
up|languages/web|hub: web host & its guests -- html, js, css
self|languages/web/css|css grammar, guest rules
sib|languages/web/html|html parser, inline script/style sinks
sib|languages/web/js|javascript grammar, guest rules

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
