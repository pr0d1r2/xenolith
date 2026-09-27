# SPEC

## §G GOAL

crate `xenolith-shebang` (⊥ feature, ⊥ grammar): Rust equivalent of `github:pr0d1r2/nix-shebang` generalised ∀ guest; re-exported by `xenolith-lang-api` as `api::shebang` ∴ one surface for language crates.

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
self|languages/shebang|shebang parse/strip/wrap ∀ guest
sib|languages/api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness
sib|languages/shells|hub: shell family -- shell, bats
sib|languages/ci|hub: build, CI & config hosts -- nix, pkl, just, yaml, dockerfile
sib|languages/data|hub: data, text & markup -- python, sql, jq, awk, perl, xml
sib|languages/web|hub: web host & its guests -- html, js, css
sib|languages/rust|rust parser, rust host sinks
sib|languages/ruby|ruby parser, ruby host sinks

## §I INTERFACES

- mod `shebang`: Rust equivalent of `github:pr0d1r2/nix-shebang` generalised ∀ guest — `has`, `get`, `strip`, `strip_strict(text, &Prelude)`, `parse -> Shebang { interpreter, args, is_env, resolved_interpreter }`, `resolves_to(LangId)`; bash/sh semantics & vectors shared w/ nix-shebang, per-language vectors ∀ other guest (`python3`, `awk -f`, `env -S jq -f`).
- `shebang::wrap(body, &Prelude) -> String`: extract file content = prelude + body; inverse of `strip_strict`.

## §V INVARIANTS


## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end | T63, T148 | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |

id|status|task|cites
T63|x|`shebang` module port + shared vectors w/ nix-shebang; law harness inlines from disk|`languages/api/src/lens:V63`,`languages/api/src/lens:V34`
T148|x|`src:C139` backfill: `languages/shebang/src/tests.rs` (`lib.rs`)|`src:C139`,`scripts/guard:V140`

## §B BUGS

id|date|cause|fix
