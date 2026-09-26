# SPEC

## §G GOAL

api crate modules: `site` (what a host finds), `lens` (the round trip), `holes` (host interpolations as params). hub root (this node) owns `lib.rs` (crate root, re-exports) & `shebang.rs` (re-export of `xenolith-shebang` + interpreter → `LangId` map, `languages/api:V32`).

## §F FEDERATION

site|Site/Delim/GuestEnv types, placement, claims, candidates|rewriting (`lens`), hole params (`holes`)|-
lens|rewrite/inline, escape, runtime base, laws harness|site discovery (`site`), hole naming (`holes`)|-
holes|param naming, param refs, hole advice|the laws that check them (`lens`)|-

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
up|languages/api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness
self|languages/api/src|api modules: site, lens, holes; hub root `lib.rs` & `shebang.rs` re-export

## §V INVARIANTS


## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end | T145 | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |

id|status|task|cites
T145|x|`src:C139` backfill: `languages/api/src/tests.rs` (`lib.rs`), `languages/api/src/shebang/tests.rs`|`src:C139`,`scripts/guard:V140`

## §B BUGS

id|date|cause|fix
