# SPEC

## §G GOAL

crate `xenolith-shebang` (⊥ feature, ⊥ grammar): Rust equivalent of `github:pr0d1r2/nix-shebang` generalised ∀ guest; re-exported by `xenolith-lang-api` as `api::shebang` ∴ one surface for language crates.

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
self|languages/shebang|shebang parse/strip/wrap ∀ guest
sib|languages/api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness
sib|languages/nix|nix parser, sinks, load idiom
sib|languages/pkl|pkl parser, hk step sinks, load idiom
sib|languages/shell|bash parser & host sinks, single-command classifier, shell linters
sib|languages/just|just parser, recipe sinks, load idiom
sib|languages/python|python grammar, guest rules
sib|languages/sql|sql grammar, guest rules
sib|languages/jq|jq grammar, guest rules
sib|languages/awk|awk grammar, guest rules
sib|languages/yaml|yaml parser, GH Actions sinks, placement
sib|languages/dockerfile|Dockerfile parser, `RUN` sinks, placement
sib|languages/rust|rust parser, rust host sinks
sib|languages/ruby|ruby parser, ruby host sinks
sib|languages/html|html parser, inline script/style sinks

## §I INTERFACES

- mod `shebang`: Rust equivalent of `github:pr0d1r2/nix-shebang` generalised ∀ guest — `has`, `get`, `strip`, `strip_strict(text, &Prelude)`, `parse -> Shebang { interpreter, args, is_env, resolved_interpreter }`, `resolves_to(LangId)`; bash/sh semantics & vectors shared w/ nix-shebang, per-language vectors ∀ other guest (`python3`, `awk -f`, `env -S jq -f`).
- `shebang::wrap(body, &Prelude) -> String`: extract file content = prelude + body; inverse of `strip_strict`.

## §V INVARIANTS


## §T TASKS

id|status|task|cites
T63|.|`shebang` module port + shared vectors w/ nix-shebang; law harness inlines from disk|`languages/api:V63`,`languages/api:V34`

## §B BUGS

id|date|cause|fix
