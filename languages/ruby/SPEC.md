# SPEC

## §G GOAL

crate `xenolith-lang-ruby` (feature `lang-ruby`): tree-sitter-ruby; host: tagged heredocs, backticks, `system("…")`.

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
self|languages/ruby|ruby parser, ruby host sinks
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
sib|languages/shebang|shebang parse/strip/wrap ∀ guest
sib|languages/rust|rust parser, rust host sinks

## §V INVARIANTS

V104: `claims`: `*.rb`, `*.rake`, `Gemfile`, `Rakefile`, `*.gemspec`, shebang `ruby`.

## §T TASKS

id|status|task|cites

## §B BUGS

id|date|cause|fix
