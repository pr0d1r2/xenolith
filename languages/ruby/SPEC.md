# SPEC

## §G GOAL

crate `xenolith-lang-ruby` (feature `lang-ruby`): tree-sitter-ruby; host: tagged heredocs, backticks, `system("…")`.

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
self|languages/ruby|ruby parser, ruby host sinks
sib|languages/api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness
sib|languages/shebang|shebang parse/strip/wrap ∀ guest
sib|languages/shells|hub: shell family -- shell, bats
sib|languages/ci|hub: build, CI & config hosts -- nix, pkl, just, yaml, dockerfile
sib|languages/data|hub: data, text & markup -- python, sql, jq, awk, perl, xml
sib|languages/web|hub: web host & its guests -- html, js, css
sib|languages/rust|rust parser, rust host sinks

## §I INTERFACES

- sinks: squiggly heredoc tagged `SQL`/`SH`/`JS`, backticks, `system("…")` w/ control syntax · sql \| shell \| js · `File.read(…)` / `Rails.root.join` ?.
- placement prototype ? (T86 evaluates): ruby → `File.read(File.join(__dir__, "sql", "<name>.sql"))`.

## §V INVARIANTS

V104: `claims`: `*.rb`, `*.rake`, `Gemfile`, `Rakefile`, `*.gemspec`, shebang `ruby`.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M7 | app languages -- rust, ruby | T19 | each host lands w/ fixtures |

id|status|task|cites
T19|.|host ruby: tagged heredocs, backticks, `system` + fixtures|`languages:V2`,`tests:V14`,`tests:V15`

## §B BUGS

id|date|cause|fix
