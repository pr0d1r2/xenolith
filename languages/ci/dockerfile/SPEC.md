# SPEC

## §G GOAL

crate `xenolith-lang-dockerfile` (feature `lang-dockerfile`): tree-sitter-dockerfile; host: `RUN` (shell form & heredoc `RUN <<EOF`).

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
up|languages/ci|hub: build, CI & config hosts -- nix, pkl, just, yaml, dockerfile
self|languages/ci/dockerfile|Dockerfile parser, `RUN` sinks, placement
sib|languages/ci/nix|nix parser, sinks, load idiom
sib|languages/ci/pkl|pkl parser, hk step sinks, load idiom
sib|languages/ci/just|just parser, recipe sinks, load idiom
sib|languages/ci/yaml|yaml parser, GH Actions sinks, placement

## §I INTERFACES

- sinks: `RUN` > single simple command, `RUN <<EOF` heredoc → guest shell (dialect per `SHELL`, default `sh -c`); load after extract: `COPY` + `RUN sh /tmp/<name>.sh` (prototype, `languages` §I).
- placement prototype ? (T86 evaluates): Dockerfile → `<dockerfile_dir>/docker/<stage>-<n>.sh`, load `COPY` + `RUN bash /tmp/<name>.sh` (2-line rewrite: inline must remove both).

## §V INVARIANTS

V87: `claims`: `Dockerfile`, `Dockerfile.*`, `*.dockerfile`, `Containerfile`, `Containerfile.*`.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M4 | CI languages -- yaml, dockerfile | T17 | each host claims its files & extracts w/ fixtures (`languages:V56`) |

id|status|task|cites
T17|.|host Dockerfile `RUN` + fixtures|`languages/shells/shell:V3`,`tests:V14`,`tests:V15`

## §B BUGS

id|date|cause|fix
