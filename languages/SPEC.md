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
shebang|shebang parse/strip/wrap ∀ guest|guest rules (`languages/<lang>`), laws (`languages/api`)|-
rust|rust parser, rust host sinks|sql & shell guest rules (their nodes)|-
ruby|ruby parser, ruby host sinks|sql, shell, js guest rules (their nodes)|-
html|html parser, inline script/style sinks|js & css guest rules (future nodes)|-

## §N NAV

rel|path|lens
up|.|-
self|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
sib|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
sib|scripts|∀ shell in repo: dev shell hook, guardrail scripts
sib|tests|fixture format & runner, integration fixtures, bats mirroring `scripts/` & `.github/scripts/`
sib|nix|flake inputs, packaging, devShell, cachix, subset override, closure
sib|docs|public project docs & notices

## §C CONSTRAINTS

- C24: ∀ language = member crate `languages/<lang>` named `xenolith-lang-<lang>`: grammar dep, parser, sinks as host, load idiom, checks as guest; OPTIONAL behind feature `lang-<lang>`. contract crate `languages/api` = `xenolith-lang-api` (⊥ feature-gated, ⊥ grammar dep; re-exports `xenolith-shebang` from `languages/shebang`): `Host` & `Guest` traits, shared types, law harness. language crate depends only on api + own grammar; ⊥ root crate, ⊥ other language crate ∴ host names guest by `LangId`, ⊥ by type.
- C4: real parsers, ⊥ regex over source. nix → `rnix`; others → `tree-sitter` + per-language grammar crates (bash, yaml, rust, ruby, html, javascript, css, python, sql, jq, awk, just, dockerfile ?, pkl ?). grammar missing for host → host unsupported, ⊥ regex fallback.
- C18: markdown fenced code = documentation, ⊥ embed. out of scope by default ?.
- C23: language set closed (`LangId`, `languages/api:V33`); third-party language crates / plugins = potential ?, ⊥ now — revisit once api traits are stable semver surface.
- C26: markdown as GUEST = potential ? (M3+ per `.` C25): md embedded in host string literals, e.g. rust tests holding spec fixtures (`const SOURCE: &str = "# SPEC\n\n## §G…"`) → extract to `tests/fixtures/<name>.md` + `include_str!`; detection by exclusion (`languages:V81`) via heading/table shape; checks `markdownlint`, `mth check` for SPEC-shaped bodies. markdown as HOST stays per C18 (fenced code = docs).

## §V INVARIANTS

V2: detection uses parser AST (C4). ⊥ regex over raw source ∀ host. test ! prove: embed inside comment | inert data string ⊥ flagged.
V56: ∀ candidate file offered to ∀ compiled-in & enabled host's `claims`; ≥1 claim → scanned by each claiming host (sites merged, sorted); 0 claims → per `src:V13` (default ignored). engine ⊥ restricts hosts by dir — only `claims` does (GH Actions dialect = `.github/workflows/*.y*ml`).
V74: default runtime base: GH Actions dialect → `RepoRoot` (`run:` cwd = workspace); Dockerfile → `HostDir` (build context = Dockerfile dir, override via rule `base`); others → `HostDir`.
V77: guest body failing guest parse ⊥ trivial → flagged `xenolith` (why `unparseable <guest>`) & extractable; parse errors then surface as lint findings on extract (`src/lint:V8`), fixers may auto-correct.
V78: site inside host parse-error region (tree-sitter `ERROR`/`MISSING`, `rnix` error node) ⊥ reported & ⊥ extracted (spans unreliable); file w/ parse error → per `[parse] host_errors` (`warn` → warning `host-parse-error`, `error` → violation `host-parse-error`).
V81: guest by EXCLUSION: start = `Host::candidates` ∩ compiled-in; detectors in fixed order only REMOVE — (1) `[[detect]]` (forces one), (2) explicit context (interpreter cmd, GH `shell:`, nix `writers.*`), (3) shebang in body (`shebang::resolves_to`), (4) heredoc tag, (5) `Guest::rejects`; 1 left → guest; >1 → first by host order, `--verbose` notes ambiguity; 0 → guest `unknown`, violation w/ `Judgment`. deterministic.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end | T57, T77, T78, T82 | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |
| M4 | CI languages -- yaml, dockerfile, just | T86 | each host claims its files & extracts w/ fixtures (`languages:V56`) |

id|status|task|cites
T57|.|`claims` ∀ host + fixtures: file claimed by 2 hosts, by none, by shebang only|V56
T77|.|unparseable guest body flagged & extracted; fixture: broken bash in nix `script`|V77
T78|.|host `ERROR` regions skipped; fixtures: site before/inside/after syntax error|V78
T82|.|detection pipeline; fixtures: shebang overrides context, tag narrows, ambiguous → host order, none → `unknown`|V81
T86|.|evaluate placement prototypes: fixtures per host, decide & promote each to a node V or drop|V74,`languages/yaml:V75`

## §B BUGS

id|date|cause|fix
