# SPEC

## §G GOAL

hub (`languages:V129`): build, CI & config hosts whose sinks mostly hold shell.

## §F FEDERATION

dir|owns|⊥owns|tokens
nix|nix parser, sinks, load idiom|shell classification (`languages/shells/shell`)|-
pkl|pkl parser, hk step sinks, load idiom|shell classification (`languages/shells/shell`)|-
just|just parser, recipe sinks, load idiom|shell classification (`languages/shells/shell`)|-
yaml|yaml parser, GH Actions sinks, placement|shell classification (`languages/shells/shell`)|-
dockerfile|Dockerfile parser, `RUN` sinks, placement|shell classification (`languages/shells/shell`)|-

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
self|languages/ci|hub: build, CI & config hosts -- nix, pkl, just, yaml, dockerfile
sib|languages/api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness
sib|languages/shebang|shebang parse/strip/wrap ∀ guest
sib|languages/shells|hub: shell family -- shell, bats, tcl
sib|languages/data|hub: data, text & markup -- python, sql, jq, awk, perl, xml
sib|languages/web|hub: web host & its guests -- html, js, css
sib|languages/rust|rust parser, rust host sinks
sib|languages/ruby|ruby parser, ruby host sinks
