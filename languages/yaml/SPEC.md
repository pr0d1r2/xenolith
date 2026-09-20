# SPEC

## §G GOAL

crate `xenolith-lang-yaml` (feature `lang-yaml`): tree-sitter-yaml; host: GH Actions workflow dialect (`run:` blocks, `${{ }}` holes); generic yaml sinks ?.

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
self|languages/yaml|yaml parser, GH Actions sinks, placement
sib|languages/api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness
sib|languages/nix|nix parser, sinks, load idiom
sib|languages/pkl|pkl parser, hk step sinks, load idiom
sib|languages/shell|bash parser & host sinks, single-command classifier, shell linters
sib|languages/just|just parser, recipe sinks, load idiom
sib|languages/python|python grammar, guest rules
sib|languages/sql|sql grammar, guest rules
sib|languages/jq|jq grammar, guest rules
sib|languages/awk|awk grammar, guest rules
sib|languages/dockerfile|Dockerfile parser, `RUN` sinks, placement
sib|languages/shebang|shebang parse/strip/wrap ∀ guest
sib|languages/rust|rust parser, rust host sinks
sib|languages/ruby|ruby parser, ruby host sinks
sib|languages/html|html parser, inline script/style sinks

## §I INTERFACES

- sinks: `run:` block → guest shell (dialect per `shell:`, default `bash -e`); load after extract: per V75.

## §V INVARIANTS

V86: `claims`: `.github/workflows/*.y*ml` & `.github/actions/**/action.y*ml` (GH dialect: `run:` sinks); other `*.yml`/`*.yaml` claimed w/o sinks ? (future dialects).
V75: GH Actions placement default: `.github/scripts/<workflow-stem>/<name>.sh`, name = step `id` \| slug(step `name`) \| `<job>-<n>`; load `run: bash .github/scripts/<workflow-stem>/<name>.sh` (base `RepoRoot`, `languages:V74`).
V97: GH `${{ }}` holes → step `env:` entries `NAME: ${{ expr }}` (NAME per `languages/api/src/holes:V40`), extract body uses `"$NAME"`, `run:` load carries no hole ∴ extraction also removes template injection (zizmor `template-injection`); overrides inline `NAME=` assignment for this dialect.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M4 | CI languages -- yaml, dockerfile, just | T14, T74, T100 | each host claims its files & extracts w/ fixtures (`languages:V56`) |

id|status|task|cites
T14|.|host yaml GH Actions `run:` + fixtures|`languages/shell:V3`,`tests:V14`,`tests:V15`
T74|.|GH Actions placement + fixture w/ step `id`, named step, anonymous step|`languages:V74`,V75
T100|.|holes → step `env:`; fixture: `${{ github.event.issue.title }}` in `run:` → env entry, zizmor clean after|V97

## §B BUGS

id|date|cause|fix
