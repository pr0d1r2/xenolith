# SPEC

## §G GOAL

crate `xenolith-lang-rust` (feature `lang-rust`): tree-sitter-rust; host: `Command::new("sh"|"bash").arg("-c")`, SQL literal passed to query fn ?.

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
self|languages/rust|rust parser, rust host sinks
sib|languages/api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness
sib|languages/shebang|shebang parse/strip/wrap ∀ guest
sib|languages/shells|hub: shell family -- shell, bats
sib|languages/ci|hub: build, CI & config hosts -- nix, pkl, just, yaml, dockerfile
sib|languages/data|hub: data & text guests -- python, sql, jq, awk, perl
sib|languages/web|hub: web host & its guests -- html, js, css
sib|languages/ruby|ruby parser, ruby host sinks

## §I INTERFACES

- sinks: `Command::new("sh"\|"bash").arg("-c")`, SQL string literal passed to query fn ? · shell \| sql · `include_str!("x.sql")`.
- placement prototype ? (T86 evaluates): rust → `<src_dir>/sql/<name>.sql`, `include_str!` (host-relative natively).

## §V INVARIANTS

V103: `claims`: `*.rs`.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M7 | app languages -- rust, ruby | T18 | each host lands w/ fixtures |

id|status|task|cites
T18|.|host rust: `Command` shell `-c`, SQL literal ? + fixtures|`languages:V2`,`tests:V14`,`tests:V15`

## §B BUGS

id|date|cause|fix
