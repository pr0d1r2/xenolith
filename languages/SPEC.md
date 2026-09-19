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
yaml|yaml parser, GH Actions sinks, placement|shell classification (`languages/shell`)|-
dockerfile|Dockerfile parser, `RUN` sinks, placement|shell classification (`languages/shell`)|-

## §N NAV

rel|path|lens
up|.|-
self|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
sib|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
sib|scripts|∀ shell in repo: dev shell hook, guardrail scripts
sib|tests|fixture format & runner, integration fixtures, bats mirroring `scripts/` & `.github/scripts/`
sib|nix|flake inputs, packaging, devShell, cachix, subset override, closure

## §C CONSTRAINTS

- C24: ∀ language = member crate `languages/<lang>` named `xenolith-lang-<lang>`: grammar dep, parser, sinks as host, load idiom, checks as guest; OPTIONAL behind feature `lang-<lang>`. contract crate `languages/api` = `xenolith-lang-api` (⊥ feature-gated, ⊥ grammar dep): `Host` & `Guest` traits, shared types, law harness. language crate depends only on api + own grammar; ⊥ root crate, ⊥ other language crate ∴ host names guest by `LangId`, ⊥ by type.
- C4: real parsers, ⊥ regex over source. nix → `rnix`; others → `tree-sitter` + per-language grammar crates (bash, yaml, rust, ruby, html, javascript, css, python, sql, jq, awk, just, dockerfile ?, pkl ?). grammar missing for host → host unsupported, ⊥ regex fallback.
- C18: markdown fenced code = documentation, ⊥ embed. out of scope by default ?.
- C23: language set closed (`LangId`, `languages/api:V33`); third-party language crates / plugins = potential ?, ⊥ now — revisit once api traits are stable semver surface.

## §I INTERFACES

### host × sink matrix (hosts w/o own node yet)
host|sink detected|embedded|load idiom after extract
rust|`Command::new("sh"\|"bash").arg("-c")`, SQL string literal passed to query fn ?|shell \| sql|`include_str!("x.sql")`
ruby|squiggly heredoc tagged `SQL`/`SH`/`JS`, backticks, `system("…")` w/ control syntax|sql \| shell \| js|`File.read(…)` / `Rails.root.join` ?
html|inline `<script>` body, inline `<style>` body, `on*=` attrs ?|js \| css|`<script src>`, `<link rel=stylesheet>`

- placement prototypes ? (for evaluation, ⊥ binding): just → `scripts/just/<recipe>.<ext>`, load `bash scripts/just/<recipe>.sh {{args}}` (recipe params → `languages/api:V40` params); Dockerfile → `<dockerfile_dir>/docker/<stage>-<n>.sh`, load `COPY` + `RUN bash /tmp/<name>.sh` (2-line rewrite: inline must remove both); bash host → `<host_dir>/<host_stem>.<name>.<ext>`, load via `"$(dirname "${BASH_SOURCE[0]}")/…"` (⚠ `$(` breaks `languages/shell:V3` for the load itself); rust → `<src_dir>/sql/<name>.sql`, `include_str!` (host-relative natively); ruby → `File.read(File.join(__dir__, "sql", "<name>.sql"))`; html → `<page_dir>/assets/<page>/<name>.js`, `<script src>`.

## §V INVARIANTS

V2: detection uses parser AST (C4). ⊥ regex over raw source ∀ host. test ! prove: embed inside comment | inert data string ⊥ flagged.
V56: ∀ candidate file offered to ∀ compiled-in & enabled host's `claims`; ≥1 claim → scanned by each claiming host (sites merged, sorted); 0 claims → per `src:V13` (default ignored). engine ⊥ restricts hosts by dir — only `claims` does (GH Actions dialect = `.github/workflows/*.y*ml`).
V74: default runtime base: GH Actions dialect → `RepoRoot` (`run:` cwd = workspace); Dockerfile → `HostDir` (build context = Dockerfile dir, override via rule `base`); others → `HostDir`.
V77: guest body failing guest parse ⊥ trivial → flagged `xenolith` (why `unparseable <guest>`) & extractable; parse errors then surface as lint findings on extract (`src/lint:V8`), fixers may auto-correct.
V78: site inside host parse-error region (tree-sitter `ERROR`/`MISSING`, `rnix` error node) ⊥ reported & ⊥ extracted (spans unreliable); file w/ parse error → per `[parse] host_errors` (`warn` → warning `host-parse-error`, `error` → violation `host-parse-error`).
V81: guest by EXCLUSION: start = `Host::candidates` ∩ compiled-in; detectors in fixed order only REMOVE — (1) `[[detect]]` (forces one), (2) explicit context (interpreter cmd, GH `shell:`, nix `writers.*`), (3) shebang in body (`shebang::resolves_to`), (4) heredoc tag, (5) `Guest::rejects`; 1 left → guest; >1 → first by host order, `--verbose` notes ambiguity; 0 → guest `unknown`, violation w/ `Judgment`. deterministic.

## §T TASKS

id|status|task|cites
T18|.|host rust: `Command` shell `-c`, SQL literal ? + fixtures|V2,`tests:V14`,`tests:V15`
T19|.|host ruby: tagged heredocs, backticks, `system` + fixtures|V2,`tests:V14`,`tests:V15`
T20|.|host html: inline `<script>`/`<style>` + fixtures|V2,`tests:V14`,`tests:V15`
T57|.|`claims` ∀ host + fixtures: file claimed by 2 hosts, by none, by shebang only|V56
T77|.|unparseable guest body flagged & extracted; fixture: broken bash in nix `script`|V77
T78|.|host `ERROR` regions skipped; fixtures: site before/inside/after syntax error|V78
T82|.|detection pipeline; fixtures: shebang overrides context, tag narrows, ambiguous → host order, none → `unknown`|V81
T86|.|evaluate placement prototypes: fixtures per host, decide & promote each to a node V or drop|V74,`languages/yaml:V75`

## §B BUGS

id|date|cause|fix
