# SPEC

## §G GOAL

the ROUND TRIP: `rewrite`/`inline`, `unescape`/`escape`, runtime base, `laws::check` & the lens laws every language crate runs.

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
up|languages/api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness
up|languages/api/src|api modules: site, lens, holes
self|languages/api/src/lens|rewrite/inline, escape, runtime base, laws harness
sib|languages/api/src/site|Site/Delim/GuestEnv types, placement, claims, candidates

## §V INVARIANTS


## §T TASKS

id|status|task|cites

## §B BUGS

id|date|cause|fix
