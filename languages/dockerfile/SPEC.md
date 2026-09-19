# SPEC

## §G GOAL

crate `xenolith-lang-dockerfile` (feature `lang-dockerfile`): tree-sitter-dockerfile; host: `RUN` (shell form & heredoc `RUN <<EOF`).

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
self|languages/dockerfile|Dockerfile parser, `RUN` sinks, placement
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
sib|languages/shebang|shebang parse/strip/wrap ∀ guest
sib|languages/rust|rust parser, rust host sinks

## §I INTERFACES

- sinks: `RUN` > single simple command, `RUN <<EOF` heredoc → guest shell (dialect per `SHELL`, default `sh -c`); load after extract: `COPY` + `RUN sh /tmp/<name>.sh` (prototype, `languages` §I).
- placement prototype ? (T86 evaluates): Dockerfile → `<dockerfile_dir>/docker/<stage>-<n>.sh`, load `COPY` + `RUN bash /tmp/<name>.sh` (2-line rewrite: inline must remove both).

## §V INVARIANTS

V87: `claims`: `Dockerfile`, `Dockerfile.*`, `*.dockerfile`, `Containerfile`, `Containerfile.*`.

## §T TASKS

id|status|task|cites
T17|.|host Dockerfile `RUN` + fixtures|`languages/shell:V3`,`tests:V14`,`tests:V15`

## §B BUGS

id|date|cause|fix
