# SPEC

## §G GOAL

∀ language = 1 workspace crate `lydite-lang-<lang>` behind feature `lang-<lang>`, 1 spec node; shared language contract (parse, sinks, load idiom, default linter). host tasks for languages w/o own crate yet wait here.

## §F FEDERATION

dir|owns|⊥owns|tokens
api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness|any language specifics (its node), engines (`src`)|-
nix|nix parser, sinks, load idiom|shell classification (`languages/shell`)|-
pkl|pkl parser, hk step sinks, load idiom|shell classification (`languages/shell`)|-
shell|bash parser & host sinks, single-command classifier, shell linters|sinks in other hosts (their node)|-

## §N NAV

rel|path|lens
up|.|-
self|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
sib|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
sib|scripts|∀ shell in repo: dev shell hook, guardrail scripts
sib|tests|fixtures per host & case, integration tests, bats mirroring `scripts/`

## §V INVARIANTS

V2: detection uses parser AST (C4). ⊥ regex over raw source ∀ host. test ! prove: embed inside comment | inert data string ⊥ flagged.

## §T TASKS

id|status|task|cites
T14|.|host yaml GH Actions `run:` + fixtures|`languages/shell:V3`,`tests:V14`,`tests:V15`
T16|.|host just (tree-sitter-just ?) + fixtures|`languages/shell:V3`,`tests:V14`,`tests:V15`
T17|.|host Dockerfile `RUN` + fixtures|`languages/shell:V3`,`tests:V14`,`tests:V15`
T18|.|host rust: `Command` shell `-c`, SQL literal ? + fixtures|V2,`tests:V14`,`tests:V15`
T19|.|host ruby: tagged heredocs, backticks, `system` + fixtures|V2,`tests:V14`,`tests:V15`
T20|.|host html: inline `<script>`/`<style>` + fixtures|V2,`tests:V14`,`tests:V15`

## §B BUGS

id|date|cause|fix
