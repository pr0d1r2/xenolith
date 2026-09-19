# SPEC

## §G GOAL

∀ language = 1 workspace crate `xenolith-lang-<lang>` behind feature `lang-<lang>`, 1 spec node; shared language contract (parse, sinks, load idiom, default linter). host tasks for languages w/o own crate yet wait here.

## §F FEDERATION

dir|owns|⊥owns|tokens
api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness|any language specifics (its node), engines (`src`)|-
nix|nix parser, sinks, load idiom|shell classification (`languages/shell`)|-
pkl|pkl parser, hk step sinks, load idiom|shell classification (`languages/shell`)|-
shell|bash parser & host sinks, single-command classifier, shell linters|sinks in other hosts (their node)|-
just|just parser, recipe sinks, load idiom|shell classification (`languages/shell`)|-
python|python grammar, guest rules|sinks holding python (their host node)|-
sql|sql grammar, guest rules|sinks holding sql (their host node)|-
jq|jq grammar, guest rules|sinks holding jq (their host node)|-
awk|awk grammar, guest rules|sinks holding awk (their host node)|-

## §N NAV

rel|path|lens
up|.|-
self|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
sib|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
sib|scripts|∀ shell in repo: dev shell hook, guardrail scripts
sib|tests|fixtures per host & case, integration tests, bats mirroring `scripts/`

## §I INTERFACES

- placement prototypes ? (for evaluation, ⊥ binding): just → `scripts/just/<recipe>.<ext>`, load `bash scripts/just/<recipe>.sh {{args}}` (recipe params → `languages/api:V40` params); Dockerfile → `<dockerfile_dir>/docker/<stage>-<n>.sh`, load `COPY` + `RUN bash /tmp/<name>.sh` (2-line rewrite: inline must remove both); bash host → `<host_dir>/<host_stem>.<name>.<ext>`, load via `"$(dirname "${BASH_SOURCE[0]}")/…"` (⚠ `$(` breaks `languages/shell:V3` for the load itself); rust → `<src_dir>/sql/<name>.sql`, `include_str!` (host-relative natively); ruby → `File.read(File.join(__dir__, "sql", "<name>.sql"))`; html → `<page_dir>/assets/<page>/<name>.js`, `<script src>`.

## §V INVARIANTS

V2: detection uses parser AST (C4). ⊥ regex over raw source ∀ host. test ! prove: embed inside comment | inert data string ⊥ flagged.
V56: ∀ candidate file offered to ∀ compiled-in & enabled host's `claims`; ≥1 claim → scanned by each claiming host (sites merged, sorted); 0 claims → per `src:V13` (default ignored). engine ⊥ restricts hosts by dir — only `claims` does (GH Actions dialect = `.github/workflows/*.y*ml`).
V74: default runtime base: GH Actions dialect → `RepoRoot` (`run:` cwd = workspace); Dockerfile → `HostDir` (build context = Dockerfile dir, override via rule `base`); others → `HostDir`.
V75: GH Actions placement default: `.github/scripts/<workflow-stem>/<name>.sh`, name = step `id` \| slug(step `name`) \| `<job>-<n>`; load `run: bash .github/scripts/<workflow-stem>/<name>.sh` (base `RepoRoot`, V74).
V77: guest body failing guest parse ⊥ trivial → flagged `xenolith` (why `unparseable <guest>`) & extractable; parse errors then surface as lint findings on extract (`src/lint:V8`), fixers may auto-correct.
V78: site inside host `ERROR` node ⊥ reported & ⊥ extracted (spans unreliable); file w/ `ERROR` → per `[parse] host_errors` (`warn` → warning `host-parse-error`, `error` → violation `host-parse-error`).
V81: guest by EXCLUSION: start = `Host::candidates` ∩ compiled-in; detectors in fixed order only REMOVE — (1) `[[detect]]` (forces one), (2) explicit context (interpreter cmd, GH `shell:`, nix `writers.*`), (3) shebang in body (`shebang::resolves_to`), (4) heredoc tag, (5) `Guest::rejects`; 1 left → guest; >1 → first by host order, `--verbose` notes ambiguity; 0 → guest `unknown`, violation w/ `Judgment`. deterministic.

## §T TASKS

id|status|task|cites
T14|.|host yaml GH Actions `run:` + fixtures|`languages/shell:V3`,`tests:V14`,`tests:V15`
T17|.|host Dockerfile `RUN` + fixtures|`languages/shell:V3`,`tests:V14`,`tests:V15`
T18|.|host rust: `Command` shell `-c`, SQL literal ? + fixtures|V2,`tests:V14`,`tests:V15`
T19|.|host ruby: tagged heredocs, backticks, `system` + fixtures|V2,`tests:V14`,`tests:V15`
T20|.|host html: inline `<script>`/`<style>` + fixtures|V2,`tests:V14`,`tests:V15`
T57|.|`claims` ∀ host + fixtures: file claimed by 2 hosts, by none, by shebang only|V56
T74|.|GH Actions placement + fixture w/ step `id`, named step, anonymous step|V74,V75
T77|.|unparseable guest body flagged & extracted; fixture: broken bash in nix `script`|V77
T78|.|host `ERROR` regions skipped; fixtures: site before/inside/after syntax error|V78
T82|.|detection pipeline; fixtures: shebang overrides context, tag narrows, ambiguous → host order, none → `unknown`|V81
T86|.|evaluate placement prototypes: fixtures per host, decide & promote each to a node V or drop|V74,V75

## §B BUGS

id|date|cause|fix
