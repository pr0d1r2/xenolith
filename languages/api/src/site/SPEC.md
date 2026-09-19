# SPEC

## §G GOAL

the SITE a host finds: `Site`, `Delim`, `DelimKind`, `GuestEnv`, holes as spans; placement, claims, guest candidates.

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
up|languages/api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness
up|languages/api/src|api modules: site, lens, holes
self|languages/api/src/site|Site/Delim/GuestEnv types, placement, claims, candidates

## §V INVARIANTS


## §T TASKS

id|status|task|cites

## §B BUGS

id|date|cause|fix
