# SPEC

## §G GOAL

∀ language = 1 workspace crate `xenolith-lang-<lang>` behind feature `lang-<lang>`, 1 spec node; shared language contract (parse, sinks, load idiom, default linter). host tasks for languages w/o own crate yet wait here.

## §F FEDERATION

dir|owns|⊥owns|tokens
api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness|any language specifics (its node), engines (`src`)|-
nix|nix parser, sinks, load idiom|shell classification (`languages/shell`)|-
pkl|pkl parser, hk step sinks, load idiom|shell classification (`languages/shell`)|-
shell|bash parser & host sinks, single-command classifier, shell linters|sinks in other hosts (their node)|-
just|just parser, recipe sinks, load idiom|shell classification (`languages/shell`)|-
python|python grammar, guest rules|sinks holding python (their host node)|-
sql|sql grammar, guest rules|sinks holding sql (their host node)|-

## §N NAV

rel|path|lens
up|.|-
self|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
sib|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
sib|scripts|∀ shell in repo: dev shell hook, guardrail scripts
sib|tests|fixtures per host & case, integration tests, bats mirroring `scripts/`

## §V INVARIANTS

V2: detection uses parser AST (C4). ⊥ regex over raw source ∀ host. test ! prove: embed inside comment | inert data string ⊥ flagged.
V56: ∀ candidate file offered to ∀ compiled-in & enabled host's `claims`; ≥1 claim → scanned by each claiming host (sites merged, sorted); 0 claims → skipped, unless named explicitly → `src:V13` exit 2. engine ⊥ restricts hosts by dir — only `claims` does (GH Actions dialect = `.github/workflows/*.y*ml`).

## §T TASKS

id|status|task|cites
T14|.|host yaml GH Actions `run:` + fixtures|`languages/shell:V3`,`tests:V14`,`tests:V15`
T17|.|host Dockerfile `RUN` + fixtures|`languages/shell:V3`,`tests:V14`,`tests:V15`
T18|.|host rust: `Command` shell `-c`, SQL literal ? + fixtures|V2,`tests:V14`,`tests:V15`
T19|.|host ruby: tagged heredocs, backticks, `system` + fixtures|V2,`tests:V14`,`tests:V15`
T20|.|host html: inline `<script>`/`<style>` + fixtures|V2,`tests:V14`,`tests:V15`
T57|.|`claims` ∀ host + fixtures: file claimed by 2 hosts, by none, by shebang only|V56

## §B BUGS

id|date|cause|fix
