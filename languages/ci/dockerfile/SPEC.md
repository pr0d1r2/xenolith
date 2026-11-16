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
- placement: ⊥ for now. The prototype (`<dockerfile_dir>/docker/<stage>-<n>.sh` plus `COPY`/`RUN bash /tmp/<name>.sh`) is dropped: `COPY` is resolved from the build context while the extract is host-relative, and the rewrite would silently depend on context layout and stage ordering. Revisit with a Dockerfile-native load contract and fixtures before offering placement.

## §V INVARIANTS

V87: `claims`: `Dockerfile`, `Dockerfile.*`, `*.dockerfile`, `Containerfile`, `Containerfile.*`.
V88: Dockerfile offers no placement until a load idiom preserves the `HostDir` base (`languages:V74`) across build contexts and multi-stage rewrites; the dropped prototype is covered by `tests/fixtures/placement-drop`.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M4 | CI languages -- yaml, dockerfile | T17 | each host claims its files & extracts w/ fixtures (`languages:V56`) |

id|status|task|cites
T17|.|host Dockerfile `RUN` + fixtures|`languages/shells/shell:V3`,`tests:V14`,`tests:V15`

## §B BUGS

id|date|cause|fix
