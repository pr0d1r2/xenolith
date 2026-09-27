# SPEC

## §G GOAL

∀ language = 1 workspace crate `xenolith-lang-<lang>` behind feature `lang-<lang>`, 1 spec node; shared language contract (parse, sinks, load idiom, default linter). host tasks for languages w/o own crate yet wait here.

## §F FEDERATION

dir|owns|⊥owns|tokens
api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness|any language specifics (its node), engines (`src`)|-
shebang|shebang parse/strip/wrap ∀ guest|guest rules (their node), laws (`languages/api`)|-
shells|hub: shell family -- shell, bats, tcl|other families (sibling hubs), contract (`languages/api`)|-
ci|hub: build, CI & config hosts -- nix, pkl, just, yaml, dockerfile|other families (sibling hubs), shell classification (`languages/shells/shell`)|-
data|hub: data, text & markup -- python, sql, jq, awk, perl, xml|other families (sibling hubs), sinks holding them (their host node)|-
web|hub: web host & its guests -- html, js, css|other families (sibling hubs)|-
rust|rust parser, rust host sinks|sql & shell guest rules (their nodes)|-
ruby|ruby parser, ruby host sinks|sql, shell, js guest rules (their nodes)|-

## §N NAV

rel|path|lens
up|.|-
self|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
sib|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
sib|scripts|∀ shell in repo: dev shell hook, guardrail scripts; gate config `hk.pkl`, vendored hk schema `pkl/Config.pkl`, `.github/workflows/`, `.github/zizmor.yml`
sib|tests|fixture format & runner, integration fixtures, bats mirroring `scripts/` & `.github/scripts/`
sib|nix|flake inputs, packaging, devShell, cachix, subset override, closure
sib|docs|public project docs & notices

## §C CONSTRAINTS

- C24: ∀ language = member crate `languages/<hub>/<lang>` (hub per V129; ⊥ family → `languages/<lang>`) named `xenolith-lang-<lang>`: grammar dep, parser, sinks as host, load idiom, checks as guest; OPTIONAL behind feature `lang-<lang>`. contract crate `languages/api` = `xenolith-lang-api` (⊥ feature-gated, ⊥ grammar dep; re-exports `xenolith-shebang` from `languages/shebang`): `Host` & `Guest` traits, shared types, law harness. language crate depends only on api + own grammar; ⊥ root crate, ⊥ other language crate ∴ host names guest by `LangId`, ⊥ by type.
- C4: real parsers, ⊥ regex over source. nix → `rnix`; others → `tree-sitter` + per-language grammar crates (bash, yaml, rust, ruby, html, javascript, css, python, sql, jq, awk, just, xml, dockerfile ?, pkl ?, tcl ?). grammar missing for host → host unsupported, ⊥ regex fallback.
- C18: markdown fenced code = documentation, ⊥ embed. out of scope by default ?.
- C23: language set closed (`LangId`, `languages/api:V33`); third-party language crates / plugins = potential ?, ⊥ now — revisit once api traits are stable semver surface.
- C26: markdown as GUEST = potential ? (M3+ per `.` C25): md embedded in host string literals, e.g. rust tests holding spec fixtures (`const SOURCE: &str = "# SPEC\n\n## §G…"`) → extract to `tests/fixtures/<name>.md` + `include_str!`; detection by exclusion (`languages:V81`) via heading/table shape; checks `markdownlint`, `mth check` for SPEC-shaped bodies. markdown as HOST stays per C18 (fenced code = docs).

## §V INVARIANTS

V2: detection uses parser AST (C4). ⊥ regex over raw source ∀ host. test ! prove: embed inside comment | inert data string ⊥ flagged.
V56: ∀ candidate file offered to ∀ compiled-in & enabled host's `claims`; ≥1 claim → scanned by each claiming host (sites merged, sorted); 0 claims → per `src/check:V13` (default ignored). engine ⊥ restricts hosts by dir — only `claims` does (GH Actions dialect = `.github/workflows/*.y*ml`).
V74: default runtime base: GH Actions dialect → `RepoRoot` (`run:` cwd = workspace); Dockerfile → `HostDir` (build context = Dockerfile dir, override via rule `base`); others → `HostDir`.
V77: guest body failing guest parse ⊥ trivial → flagged `xenolith` (why `unparseable <guest>`) & extractable; parse errors then surface as lint findings on extract (`src/lint:V8`), fixers may auto-correct.
V78: site inside host parse-error region (tree-sitter `ERROR`/`MISSING`, `rnix` error node) ⊥ reported & ⊥ extracted (spans unreliable); file w/ parse error → per `[parse] host_errors` (`warn` → warning `host-parse-error`, `error` → violation `host-parse-error`).
V81: guest by EXCLUSION: start = `Host::candidates` ∩ compiled-in; detectors in fixed order only REMOVE — (1) `[[detect]]` (forces one), (2) explicit context (interpreter cmd, GH `shell:`, nix `writers.*`), (3) shebang in body (`shebang::resolves_to`), (4) heredoc tag, (5) `Guest::rejects`; 1 left → guest; >1 → first by host order, `--verbose` notes ambiguity; 0 → guest `unknown`, violation w/ `Judgment`. deterministic.
V121: grammar ⊥ published on crates.io → VENDOR its generated C (`src/parser.c`, `src/scanner.c`) into the language crate, built w/ `cc`, recording upstream repo, rev & license in the crate & in `docs:V108`; ⊥ git dep (unpublishable, breaks `src` C1 & `nix:V112`). refresh = own commit naming the new rev. measured 2026-09-20: `tree-sitter-pkl` (apple, Apache-2.0) & `tree-sitter-awk` (Beaglefoot, MIT) exist only as repos; bash, nix (`rnix`), yaml, just, jq, dockerfile, rust, ruby, html, python ship as crates.
V130: KINSHIP `base`: B based-on A (bats→shell, gawk→awk, gojq|jaq→jq, postgres→sql) ⇒ B ∃ own `LangId` (`languages/api:V33`) & own `claims`; A ⊥ claims B's files; B's checks|fixers = A's ∪ deltas in B's node. ≥2 fixtures ∀ `base` pair (`tests:V15`): (a) B text A's grammar ACCEPTS w/ WRONG verdict, (b) B text A's grammar REJECTS. both shapes measured & recorded at `languages/shells/bats:V134` & `languages/shells/shell:V137`.
V131: KINSHIP `lookalike`: unordered pair {A,B}, ⊥ `base` edge, surface text of one parses as other (awk~shell, perl~shell, jq~shell, pkl~nix) ⇒ ∃ discriminating fixture PAIR (`tests:V15`) & `Guest::rejects` vetoes (`languages/api/src/site` §I); `Host::candidates` ORDER ⊥ sole discriminator (tie-break only, V81). measured 2026-09-21: awk `/^x/ { n++ } END { print n }` parses as ONE simple bash command; perl `my $x = shift; print "$x\n";` as bash `sequence`.
V129: languages grouped by family under hubs `languages/{shells,ci,data,web}` (`.:C22`), hub §F = members; rule ∀ 1 family → its hub, spanning families → here; ⊥ family → direct child (rust, ruby). measured 2026-09-27: chain = §G §C §I §V §T, ⊥ §F/§N ∴ a hub cuts other chains only by rows it takes from here.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end | T57, T77, T78, T82, T120 | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |
| M2 | kinship & test hosts | T128, T130, T137 | ∀ `base` & `lookalike` edge carries its fixtures (`languages:V130`, `languages:V131`) |
| M4 | CI languages -- yaml, dockerfile | T86 | each host claims its files & extracts w/ fixtures (`languages:V56`) |

id|status|task|cites
T57|.|`claims` ∀ host + fixtures: file claimed by 2 hosts, by none, by shebang only|V56
T77|.|unparseable guest body flagged & extracted; fixture: broken bash in nix `script`|V77
T78|.|host `ERROR` regions skipped; fixtures: site before/inside/after syntax error|V78
T82|.|detection pipeline; fixtures: shebang overrides context, tag narrows, ambiguous → host order, none → `unknown`|V81
T86|.|evaluate placement prototypes: fixtures per host, decide & promote each to a node V or drop|V74,`languages/ci/yaml:V75`
T120|.|vendoring harness: `cc` build, rev+license record, notices hook, `cargo package` proves the C ships (`nix:V113`)|V121,`docs:V108`
T128|.|kinship table in api + fixture pairs ∀ `base` & ∀ `lookalike` edge (2 per base, 1 pair per lookalike)|V130,V131,`languages/api:V33`,`tests:V15`
T130|.|evaluate tree-sitter `injections.scm` as a CANDIDATE source ?: upstream grammars already declare which node holds which language ∴ `Host::candidates` could READ them than restate sinks by hand. decide: consume, vendor-and-consume, or reject w/ reason|V81,`languages/api/src/site` §I
T137|x|category hubs per V129; rows ∀ one family moved by `sherd adopt`|V129,`scripts:V26`

## §B BUGS

id|date|cause|fix
