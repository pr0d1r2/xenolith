# SPEC

## §G GOAL

crate `xenolith-lang-html` (feature `lang-html`): tree-sitter-html; host: inline `<script>`, `<style>`, `on*=` attrs ?.

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
self|languages/html|html parser, inline script/style sinks
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

## §I INTERFACES

- sinks: inline `<script>` body, inline `<style>` body, `on*=` attrs ? · js \| css · `<script src>`, `<link rel=stylesheet>`.
- placement prototype ? (T86 evaluates): html → `<page_dir>/assets/<page>/<name>.js`, `<script src>`.

## §V INVARIANTS

V105: `claims`: `*.html`, `*.htm`; templates (`*.erb`, `*.hbs`) ? later.

## §T TASKS

id|status|task|cites
T20|.|host html: inline `<script>`/`<style>` + fixtures|`languages:V2`,`tests:V14`,`tests:V15`

## §B BUGS

id|date|cause|fix
