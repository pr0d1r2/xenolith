# SPEC

## §G GOAL

crate `xenolith-lang-html` (feature `lang-html`): tree-sitter-html; host: inline `<script>`, `<style>`, `on*=` attrs ?.

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
up|languages/web|hub: web host & its guests -- html, js, css
self|languages/web/html|html parser, inline script/style sinks
sib|languages/web/js|javascript grammar, guest rules
sib|languages/web/css|css grammar, guest rules

## §I INTERFACES

- sinks: inline `<script>` body, inline `<style>` body, `on*=` attrs ? · js \| css · `<script src>`, `<link rel=stylesheet>`.
- placement prototype ? (T86 evaluates): html → `<page_dir>/assets/<page>/<name>.js`, `<script src>`.

## §V INVARIANTS

V105: `claims`: `*.html`, `*.htm`; templates (`*.erb`, `*.hbs`) ? later.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M6 | web languages -- html (js & css guests) | T20 | inline `<script>`/`<style>` extraction w/ fixtures |

id|status|task|cites
T20|.|host html: inline `<script>`/`<style>` + fixtures|`languages:V2`,`tests:V14`,`tests:V15`

## §B BUGS

id|date|cause|fix
