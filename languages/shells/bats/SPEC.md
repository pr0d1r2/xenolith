# SPEC

## §G GOAL

crate `xenolith-lang-bats` (feature `lang-bats`): bats as HOST, based-on shell (`languages:V130`) -- claims `*.bats`, parses w/ tree-sitter-bash + bats syntax (`@test NAME { }`, `load`, `setup`/`teardown`, `run`), finds foreign guests inside test bodies & fixture blobs. TEST host (`languages/shells/bats:V133`) ∴ test body itself stays, its extracts carry no companion.

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
up|languages/shells|hub: shell family -- shell, bats, tcl
self|languages/shells/bats|bats grammar (based-on shell), `@test` sinks, test-host rules
sib|languages/shells/shell|bash parser & host sinks, single-command classifier, shell linters
sib|languages/shells/tcl|tcl grammar (expect dialect), `exec`/`spawn` sinks, guest rules

## §C CONSTRAINTS

- C27: ⊥ own grammar crate: bats = bash + 4 extra forms ∴ parse w/ `tree-sitter-bash` (dep shared w/ `languages/shells/shell`) & recognise bats forms over that tree; a separate grammar would fork bash's & drift. `@test NAME { ... }` already parses as bash (command + brace group, measured 2026-09-21) ∴ recognition = SHAPE match on that tree, ⊥ new parser.

## §I INTERFACES

- sinks: heredoc fed to interpreter inside a test body (`python <<`, `psql <<`, `jq <<`), `run <interp> -c '...'` / `-e '...'`, fixture blob written by heredoc (`cat > "$BATS_TMPDIR/x.sql" <<SQL`) → guest per `languages:V81`; load after extract = `<interp> "$(dirname "${BATS_TEST_FILENAME}")/<extract>"` (bats' own path var, ≠ shell's `BASH_SOURCE`).
- `[extract.bats] fixtures` (default `fixtures`): dir, relative to host file, for extracts from a test host; `[extract.bats] body` ∈ `keep` (default) | `report` -- `report` flags a non-trivial test body as `Judgment`, ⊥ extracts it.

## §V INVARIANTS

V133: TEST HOST (bats; rspec|pytest ?): test body ⊥ extracted (it IS a script by design); foreign guest INSIDE it = site & extractable; extract = test DATA ∴ ⊥ companion (`scripts/guard:V21` exempt) & ⊥ coverage-bearing. rules & fixtures: `languages/shells/bats:V135`.
V134: bats claims `*.bats` & `*.bats` ⊥ claimed by shell (`languages:V130`, `languages/shells/shell:V137`). recognition over the bash tree: `@test` = command whose name is `@test` followed by brace group; `load`, `setup`, `teardown`, `setup_file`, `teardown_file`, `run`, `bats_*` = bats vocabulary, ⊥ user commands ∴ ⊥ reported as unknown.
V135: `@test` body ⊥ a site (`languages/shells/bats:V133`): a test body IS a script by design. sites = foreign guests INSIDE it only. extract from a `*.bats` host lands in `[extract.bats] fixtures` dir & carries ⊥ companion & ⊥ mirrored bats (`scripts/guard:V21` exempt). fixtures: test body w/ pipeline & `if` ⊥ flagged; heredoc python inside test body → flagged & extracted; `run bash -c 'a | b'` → flagged.
V136: bats checks ? (unconfirmed until measured): `shellcheck --shell=bash` reports `@test` as a command it ⊥ know ∴ either shellcheck w/ `--exclude` list recorded here, or `bats --count <file>` as syntax check, or a bats-aware linter -- decide by MEASURING all three on this repo's own 5 `*.bats` files, ⊥ by guessing. fixers: `shfmt` ? (`@test` braces may reformat wrongly -- measure).

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M2 | kinship & test hosts | T129 | ∀ `base` & `lookalike` edge carries its fixtures (`languages:V130`, `languages:V131`) |
| M8 | test hosts | T132-T134 | `*.bats` claimed by bats ⊥ shell, guests inside test bodies extracted w/o companion (`languages/shells/bats:V135`) |

id|status|task|cites
T129|.|test-host rules: `@test`-shaped body ⊥ extracted, guests inside it extracted, extract ⊥ companion; fixtures per rule|V133,`scripts/guard:V21`
T132|.|scaffold `languages/shells/bats` crate: `claims`, bats-form recognition over tree-sitter-bash; fixtures: `@test` file ⊥ claimed by shell, bats vocabulary ⊥ unknown|V134,C27,`languages:V130`
T133|.|sinks inside test bodies + `BATS_TEST_FILENAME` load idiom; fixtures per `languages/shells/bats:V135` incl. fixture-blob heredoc|V135,`languages/shells/bats:V133`,`tests:V15`
T134|.|MEASURE the 3 check candidates on this repo's `*.bats`, record counts & shapes, promote the winner to V136 & drop the `?`|V136,`src/lint:V8`

## §B BUGS

id|date|cause|fix
