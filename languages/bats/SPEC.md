# SPEC

## §G GOAL

crate `xenolith-lang-bats` (feature `lang-bats`): bats as HOST, based-on shell (`languages:V130`) — claims `*.bats`, parses w/ tree-sitter-bash + bats syntax (`@test NAME { }`, `load`, `setup`/`teardown`, `run`), finds foreign guests inside test bodies & fixture blobs. TEST host (`languages:V133`) ∴ test body itself stays, its extracts carry no companion.

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
self|languages/bats|bats grammar (based-on shell), `@test` sinks, test-host rules
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
sib|languages/css|css grammar, guest rules
sib|languages/perl|perl grammar, guest rules

## §C CONSTRAINTS

- C27: ⊥ own grammar crate: bats = bash + 4 extra forms ∴ parse w/ `tree-sitter-bash` (dep shared w/ `languages/shell`) & recognise bats forms over that tree; a separate grammar would fork bash's & drift. `@test NAME { … }` already parses as bash (command + brace group, measured 2026-09-21) ∴ recognition = SHAPE match on that tree, ⊥ new parser.

## §I INTERFACES

- sinks: heredoc fed to interpreter inside a test body (`python <<`, `psql <<`, `jq <<`), `run <interp> -c '…'` / `-e '…'`, fixture blob written by heredoc (`cat > "$BATS_TMPDIR/x.sql" <<SQL`) → guest per `languages:V81`; load after extract = `<interp> "$(dirname "${BATS_TEST_FILENAME}")/<extract>"` (bats' own path var, ≠ shell's `BASH_SOURCE`).
- `[extract.bats] fixtures` (default `fixtures`): dir, relative to host file, for extracts from a test host; `[extract.bats] body` ∈ `keep` (default) | `report` — `report` flags a non-trivial test body as `Judgment`, ⊥ extracts it.

## §V INVARIANTS

V134: bats claims `*.bats` & `*.bats` ⊥ claimed by shell (`languages:V130`, `languages/shell:V137`). recognition over the bash tree: `@test` = command whose name is `@test` followed by brace group; `load`, `setup`, `teardown`, `setup_file`, `teardown_file`, `run`, `bats_*` = bats vocabulary, ⊥ user commands ∴ ⊥ reported as unknown.
V135: `@test` body ⊥ a site (`languages:V133`): a test body IS a script by design. sites = foreign guests INSIDE it only. extract from a `*.bats` host lands in `[extract.bats] fixtures` dir & carries ⊥ companion & ⊥ mirrored bats (`scripts/guard:V21` exempt). fixtures: test body w/ pipeline & `if` ⊥ flagged; heredoc python inside test body → flagged & extracted; `run bash -c 'a | b'` → flagged.
V136: bats checks ? (unconfirmed until measured): `shellcheck --shell=bash` reports `@test` as a command it ⊥ know ∴ either shellcheck w/ `--exclude` list recorded here, or `bats --count <file>` as syntax check, or a bats-aware linter — decide by MEASURING all three on this repo's own 5 `*.bats` files, ⊥ by guessing. fixers: `shfmt` ? (`@test` braces may reformat wrongly — measure).

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M8 | test hosts | T132-T134 | `*.bats` claimed by bats ⊥ shell, guests inside test bodies extracted w/o companion (`languages/bats:V135`) |

id|status|task|cites
T132|.|scaffold `languages/bats` crate: `claims`, bats-form recognition over tree-sitter-bash; fixtures: `@test` file ⊥ claimed by shell, bats vocabulary ⊥ unknown|V134,C27,`languages:V130`
T133|.|sinks inside test bodies + `BATS_TEST_FILENAME` load idiom; fixtures per `languages/bats:V135` incl. fixture-blob heredoc|V135,`languages:V133`,`tests:V15`
T134|.|MEASURE the 3 check candidates on this repo's `*.bats`, record counts & shapes, promote the winner to V136 & drop the `?`|V136,`src/lint:V8`

## §B BUGS

id|date|cause|fix
