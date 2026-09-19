# SPEC

## §G GOAL

HOLES as params: named env params, their references & inverse, host hole advice.

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
up|languages/api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness
up|languages/api/src|api modules: site, lens, holes
self|languages/api/src/holes|param naming, param refs, hole advice
sib|languages/api/src/site|Site/Delim/GuestEnv types, placement, claims, candidates
sib|languages/api/src/lens|rewrite/inline, escape, runtime base, laws harness

## §V INVARIANTS


## §T TASKS

id|status|task|cites

## §B BUGS

id|date|cause|fix
