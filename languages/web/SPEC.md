# SPEC

## §G GOAL

hub (`languages:V129`): web page host & its script/style guests.

## §F FEDERATION

dir|owns|⊥owns|tokens
html|html parser, inline script/style sinks|js & css guest rules (future nodes)|-
js|javascript grammar, guest rules|html sinks (`languages/web/html`)|-
css|css grammar, guest rules|html sinks (`languages/web/html`)|-

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
self|languages/web|hub: web host & its guests -- html, js, css
sib|languages/api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness
sib|languages/shebang|shebang parse/strip/wrap ∀ guest
sib|languages/shells|hub: shell family -- shell, bats
sib|languages/ci|hub: build, CI & config hosts -- nix, pkl, just, yaml, dockerfile
sib|languages/data|hub: data & text guests -- python, sql, jq, awk, perl
sib|languages/rust|rust parser, rust host sinks
sib|languages/ruby|ruby parser, ruby host sinks
