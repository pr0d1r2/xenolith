# SPEC

## §G GOAL

hub (`languages:V129`): shell family -- shell dialects & hosts built on the shell grammar.

## §F FEDERATION

dir|owns|⊥owns|tokens
shell|bash parser & host sinks, single-command classifier, shell linters|sinks in other hosts (their node)|-
bats|bats grammar (based-on shell), `@test` sinks, test-host rules|shell classification (`languages/shells/shell`), mirror rule (`scripts/guard`)|-
tcl|tcl grammar (expect dialect), `exec`/`spawn` sinks, guest rules|shell classification & shell-side sinks (`languages/shells/shell`)|-

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
self|languages/shells|hub: shell family -- shell, bats, tcl
sib|languages/api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness
sib|languages/shebang|shebang parse/strip/wrap ∀ guest
sib|languages/ci|hub: build, CI & config hosts -- nix, pkl, just, yaml, dockerfile
sib|languages/data|hub: data, text & markup -- python, sql, jq, awk, perl, xml
sib|languages/web|hub: web host & its guests -- html, js, css
sib|languages/rust|rust parser, rust host sinks
sib|languages/ruby|ruby parser, ruby host sinks

## §V INVARIANTS
V132: `dialect` ≠ `base`: dialect = SAME grammar, other options|semantics (`GuestEnv.dialect`, `languages/shells/shell:V82`); base = own syntax ∴ own `LangId`. zsh = both — dialect ∀ options, base-like ∀ zsh-only syntax: construct ⊥ in the base grammar → `Judgment`, ⊥ `host-parse-error`, ⊥ `Err` (`languages/shells/shell:V138`).
