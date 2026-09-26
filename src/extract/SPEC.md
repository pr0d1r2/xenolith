# SPEC

## §G GOAL

move embed → own file & rewrite host to load it: diff default, `--write`, lossless, idempotent, collision guard.

## §N NAV

rel|path|lens
up|.|-
up|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
self|src/extract|embed → own file, host rewrite, diff \| `--write`
sib|src/config|`xenolith.toml` parse & validation
sib|src/graph|host → extract load edges, dangling & orphan
sib|src/lint|per-language linter map & invocation
sib|src/cli|verbs, flags, exit codes, rule ids, output formats, hk wiring

## §C CONSTRAINTS

- C15: report-only by default. `xnl extract` writes only w/ explicit `--write`; default prints diff.

## §I INTERFACES

- `[extract]`: `layout` ∈ `host` (default: host placement only) \| `mirror` (`<root>/<host path sans ext>/<name>.<ext>`) \| `sibling` (`<host_dir>/<host_stem>.<name>.<ext>`) \| `central` (`<root>/<guest>/<name>.<ext>`); `root` (default `scripts`). layer C.
- `[[extract.rule]]`: match `host`, `sink` (glob, `*` = one dotted segment), `guest` — each optional, ≥1 required; set any of `path` (template), `base` (`host` \| `root` \| `"<dir>"`, overrides `Host::runtime_base`), `invoke` (argv template, overrides `Guest::invoke`), `prelude` (`{ shebang, strict }`), `executable`, `companion` (template). layer B, highest precedence.
- `[extract] depth` (default 5, ≥1): max nesting levels extracted in one run.
- lib: `xenolith::extract(root, &Config, &Options) -> Result<Edit, ExtractError>` extracts exactly the `xenolith` sites `xenolith::check` flags (`src:V152`) in files ⊥ `[[exclude]]` \| `[extract] exclude`; `:line` w/ ⊥ such site → refused. `Edit` = per host: before, after, extracts, refusals, explain; `apply(root, &Edit)` = `--write`.
- template vars: `{host_dir}` @ root = `""`; `{path}`/`{path_stem}` = extract path w/ \| sans ext as loaded (`invoke` only; in `path` → exit 2). load text = extract path relative to base (default host dir; `Host::runtime_base` ⊥ yet), `./`-led unless `../`.
- refused until V4 provable: holes (`Guest::param` ⊥, `languages/api/src/holes:T76`), host `loads` \| `inline` ⊥, rule `companion` (T51: stub content unspecified), `strict = "enforce"`. extract ends `\n`.
- collision suffix (V47) = sink's last dotted segment, lowercased, ⊥ `[a-z0-9]` → `-`, appended to ∀ colliding stems; lock (V127) = OS advisory lock, text `pid start`, removed on release; `apply` re-reads hosts under it, changed → refused.
- `[extract] inactive_rules` ∈ `ignore` \| `warn` (default) \| `error`: handling of `[[extract.rule]]` whose host or guest is compiled out | `[langs]`-disabled.

## §V INVARIANTS

V4: `extract` LOSSLESS: extracted file bytes + host rewrite ! round-trip — inlining extract back into host reproduces original semantics (normalized whitespace equal); asserted before write.
V5: `extract` IDEMPOTENT: `extract(extract(x)) == extract(x)`; rerun on clean host = no-op.
V6: `extract --write` ⊥ overwrite existing file ≠ same bytes; collision → exit 2 w/ message.
V45: placement resolved PER FIELD (path, name, base, invoke, prelude, executable, companion): most specific matching `[[extract.rule]]` (B) > `[extract] layout` (C) > `Host::placement` / `Guest` defaults (D). specificity = count of matched keys (host, sink, guest); tie between rules → nearer `xenolith.toml` wins; same file → exit 2 naming both.
V46: template vars closed set `{name}`, `{ext}`, `{host_dir}`, `{host_stem}`, `{sink}`, `{guest}`, `{path}`, `{path_stem}`; render deterministic, result normalized relative to repo root.
V47: 2 sites resolving to same path → disambiguate by appending sink segment (`foo-script`, `foo-prestart`), deterministic; still equal → V6 exit 2.
V48: `xnl extract --verbose` prints ∀ site ∀ field the deciding layer (`rule #n` \| `layout` \| `host` \| `guest`) ∴ placement explainable, ⊥ guessed.
V49: `companion` configured → `--write` creates companion stub w/ extract; V4–V6 cover companion too: existing companion ⊥ rewritten, rerun = no-op, differing existing file → exit 2.
V64: multi-site host: rewrites applied back-to-front (descending span) on one parse ∴ earlier spans stay valid; under `--write` file all-or-nothing — any site refused (holes, collision) → file & its extracts untouched, other files proceed, refusal reported.
V65: nested xenoliths: extract runs to fixpoint — ∀ extract re-scanned as host & extracted in turn until ⊥ site or `[extract] depth` (default 5) reached; depth reached w/ sites left → exit 2 naming chain host → … → site. V5 idempotence holds over whole tree.
V68: rule-configured `invoke` ! yield load host's `loads()` recognises: after rewrite, `loads(y) ∋ p` (`languages/api/src/lens:V34` (c)) checked @ runtime; ⊥ → exit 2 naming rule & host ∴ custom invoke & `graph` ⊥ disagree.
V71: ∀ write path: canonicalize existing ancestors (resolve symlinks); resolved path ∉ repo root \| any existing component = symlink → exit 2; ⊥ write through symlink, ⊥ create dir via symlink.
V80: extract skips site covered by `[[allow]]` & files under `[[exclude]]` \| `[extract] exclude`; `--verbose` names skip reason.
V83: rendered extract path & path in load text ⊆ `[A-Za-z0-9._/-]`, ⊥ leading `-`; else exit 2 ∴ ⊥ injection into host syntax (nix path, yaml, pkl strings).
V84: writes atomic per file: temp in same dir → fsync → rename; per host: extracts & companions written BEFORE host rewrite ∴ crash leaves orphan extract (`src/graph:V7` finds it), ⊥ dangling load.
V99: `xnl extract --relocate [--write]` moves ∀ `misplaced-extract` to its expected path & rewrites the load, same atomicity & order as V84 (new file, host, then old file removed); companion moves along.
V101: `xnl inline` = exact inverse of extract: removes extract & companion only after host rewrite written; result passes V5 (rerun extract = no-op for trivial body).
V127: ONE writer: `extract --write` (& `--relocate`, `xnl inline`) takes an advisory lock `.xenolith.lock` @ repo root; a 2nd run exits 2 naming the holder's pid & start time; stale lock (holder gone) reclaimed. ⊥ 2 writers: V64's all-or-nothing is per file & assumes one rewriter.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end | T22, T50, T51, T64-T66, T69, T72, T81, T84-T85, T126 | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |
| M3 | publication | T67, T101, T102 | public doc set, release machinery, history audit green, crates published (`.:T32`) |
| M4 | CI languages -- yaml, dockerfile, just | T23 | each host claims its files & extracts w/ fixtures (`languages:V56`) |

id|status|task|cites
T22|x|`extract` nix + pkl + yaml + bash (first wave): diff default, `--write`, lossless & idempotent asserts, collision guard|V4,V5,V6,C15
T23|.|`extract` remaining hosts (just, Dockerfile, rust, ruby, html)|V4,V5,V6
T50|x|placement resolver: per-field precedence, templates, collision suffix, `--verbose` explain|V45,V46,V47,V48
T51|.|companion creation under `--write` w/ V4–V6 laws|V49
T64|x|back-to-front multi-site rewrite + all-or-nothing file write; fixture w/ 3 sites, 1 refused|V64
T65|.|fixpoint extraction w/ depth limit; fixture nix → shell → python (3 levels) & one over limit|V65
T66|x|repo-root placement path → host-relative load path; fixtures w/ host in subdir|`languages/api/src/lens:V66`,V46
T67|.|byte fidelity (CRLF, BOM, trailing newline, non-UTF-8) via dedicated crate in preparation, ⊥ public yet (unnamed per C17); until then V4 compares normalized whitespace|V4,C17
T69|.|runtime `languages/api/src/lens:V34` (c) check for rule `invoke`; fixture w/ unrecognisable invoke → exit 2|V68
T72|x|symlink guard on write; fixtures: symlinked dir inside root, symlink pointing outside|V71
T81|x|extract skips allowed & excluded; fixture: allowed site untouched by `--write`|V80
T84|x|path charset guard; fixtures: rule template yielding space, quote, leading `-`|V83
T85|x|atomic writes & write order; test kills between extract & host write|V84
T101|.|`--relocate` + fixture: layout change → file moved, load rewritten, graph clean|V99,`src/graph:V98`
T102|.|`xnl inline` + `inlineable-extract`; fixtures: shrunk extract inlined, shared extract refused|V101,`src/graph:V100`
T126|.|repo lock around write paths + fixture: concurrent `--write` exits 2, stale lock reclaimed|V127,V64

## §B BUGS

id|date|cause|fix
