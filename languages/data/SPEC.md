# SPEC

## §G GOAL

hub (`languages:V129`): data & text-processing languages, mostly guests.

## §F FEDERATION

dir|owns|⊥owns|tokens
python|python grammar, guest rules|sinks holding python (their host node)|-
sql|sql grammar, guest rules|sinks holding sql (their host node)|-
jq|jq grammar, guest rules|sinks holding jq (their host node)|-
awk|awk grammar, guest rules|sinks holding awk (their host node)|-
perl|perl grammar, guest rules|sinks holding perl (their host node)|-
xml|xml parser, launchd argv sinks, `xmllint` check|shell classification (`languages/shells/shell`)|-

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
self|languages/data|hub: data, text & markup -- python, sql, jq, awk, perl, xml
sib|languages/api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness
sib|languages/shebang|shebang parse/strip/wrap ∀ guest
sib|languages/shells|hub: shell family -- shell, bats, tcl
sib|languages/ci|hub: build, CI & config hosts -- nix, pkl, just, yaml, dockerfile
sib|languages/web|hub: web host & its guests -- html, js, css
sib|languages/rust|rust parser, rust host sinks
sib|languages/ruby|ruby parser, ruby host sinks
