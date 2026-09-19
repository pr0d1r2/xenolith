# SPEC

## §G GOAL

api crate modules: `site` (what a host finds), `lens` (the round trip), `holes` (host interpolations as params).

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
up|languages/api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness
self|languages/api/src|api modules: site, lens, holes

## §V INVARIANTS


## §T TASKS

id|status|task|cites

## §B BUGS

id|date|cause|fix
