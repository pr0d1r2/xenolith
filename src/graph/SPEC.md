# SPEC

## §G GOAL

host → extract load edges; dangling load & orphan extract detection.

## §N NAV

rel|path|lens
up|.|-
up|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
self|src/graph|host → extract load edges, dangling & orphan
sib|src/config|`xenolith.toml` parse & validation
sib|src/extract|embed → own file, host rewrite, diff \| `--write`
sib|src/lint|per-language linter map & invocation
sib|src/cli|verbs, flags, exit codes, rule ids, output formats, hk wiring
sib|src/check|check engine: candidates → claims → sites → guests → violations; unclaimed & missing-guest policy, parallel scan
sib|src/discover|candidate discovery: `git ls-files` \| named paths, normalisation, symlink screen
sib|src/registry|language registry: `hosts()`, `guests()`, feature gates, feature names

## §I INTERFACES

- json (`xnl graph --format json`): envelope (`src/cli` §I) + `edges`: `{"schema": 1, "edges": [{"host": "nixos/foo.nix", "sink": "systemd.services.foo.script", "line": 12, "col": 5, "extract": "nixos/foo/foo-script.sh", "guest": "shell", "params": ["FOO_BIN"]}], "violations": [{"rule": "orphan-extract", ...}, {"rule": "dangling-load", ...}], "warnings": []}`; orphans & dangling ONLY as violations, ⊥ separate arrays.
- lib: `xenolith::graph(&Path, &Config, &graph::Options) -> Result<Graph, GraphError>`; `Graph` = `edges` sorted (host, line, col, extract) + `Report`; `GraphError` = exit 2 (discovery, nested config, unclaimed strict, outside root).
- roots (V50): rule `path` → text before first `{`; `[extract] root` iff `layout` ∈ `mirror` \| `central` (layouts placing under it); `Placement.dir` ∀ site the run sees, `{host_dir}` `{host_stem}` rendered, cut as rule `path`, taken as dir; `Unsupported` \| empty \| `..` \| absolute → ⊥ root (⊥ whole repo).
- extract (V7) = candidate under a root that a compiled-in guest reads (shebang, else extension: `src/lint` §I targets), ⊥ excluded; other files under roots (SPEC.md, `xenolith.toml`) ⊥ orphan. unclaimed = ⊥ host claims ∧ ⊥ guest reads (`src/check:V13`).
- orphan (V7) judged only on whole-tree run ∧ ∀ claimed host's loads known: named paths = partial view (∼ `src/config:V9` staleness); `loads` `Unsupported` → warning `loads-unsupported` ∀ host lang, ⊥ parse → `[parse] host_errors`; either → orphan ⊥ judged.
- load resolves from host file dir (`languages/api/src/lens:V66` default); `runtime_base` & rule `base` ⊥ applied (`LoadRef` ∌ sink ∴ rule ⊥ matchable); outside root \| through symlink (V72) \| ⊥ regular file → `dangling-load`.
- fields: edge `sink` = sink of the 1 site the load reads back to (`src/extract:V270`; ⊥ read back → extract bytes put back as is), ⊥ such site → `""`; `params` `[]` until `LoadRef` carries them; `dangling-load` @ load span, guest = load's; `orphan-extract` @ `1:1`, host = guest = reader; `sink` `""`, `site` `argv-string` (`src:V1` unsited stand-in, as `src/check:V152` engine).
- misplaced \| inlineable (V98, V100): judged per edge by `src/extract:V270` read-back; expected = placed path \| its `src/extract:V47` suffix form; read-back ⊥ → ⊥ warned; body judged as `src/check:V152` (trivial, `[threshold]`); `inlineable-extract` only if run sees 1 load of it; warning @ extract, names its load & `xnl extract --relocate` \| `xnl inline`.

## §V INVARIANTS

V7: `graph`: ∀ load in host resolves to existing file (dangling = violation); ∀ file under extract roots (V50) loaded by ≥1 host (orphan = violation).
V50: extract roots = static prefix (before first `{`) of ∀ rule `path` + layout `root` + ∀ `Host::placement` dir; orphan scan (V7) walks exactly these, ⊥ whole repo.
V72: orphan scan & load resolution ⊥ follow symlinks; load resolving through symlink → `dangling-load`.
V98: extract whose path ≠ path current placement config would give (layout change, renamed sink) → warning `misplaced-extract` naming expected path.
V100: extract whose body is trivial for its guest (incl. threshold) → warning `inlineable-extract`.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end |  | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |

id|status|task|cites

## §B BUGS

id|date|cause|fix
B1|2026-09-26|shell host registered (`src/registry:B11`), `loads` ⊥ ∴ `loads-unsupported` warned on named-path runs & rootless trees, where no orphan scan runs|warn only when a scan would run: whole-tree ∧ ≥1 root
B2|2026-09-27|edge `sink` always `""` (§I json example names it): `LoadRef` ∌ sink & edge ⊥ read back|sink from judge's read-back (V98) site; ⊥ read back → body put back as is
