# SPEC

## §G GOAL

`xenolith.toml` format, discovery, merge, validation & defaults table; engine-specific sections live in their engine node (`[lint.*]` → `src/lint`, `[extract]` & `[[extract.rule]]` → `src/extract`, `[extract.shell]` → `languages/shells/shell`); here: extract layout & rules, `[[allow]]` (reason, hash|span, staleness), lint map, langs toggle, thresholds.

## §N NAV

rel|path|lens
up|.|-
up|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
self|src/config|`xenolith.toml` parse & validation
sib|src/extract|embed → own file, host rewrite, diff \| `--write`
sib|src/graph|host → extract load edges, dangling & orphan
sib|src/lint|per-language linter map & invocation
sib|src/cli|verbs, flags, exit codes, rule ids, output formats, hk wiring
sib|src/check|check engine: candidates → claims → sites → guests → violations; unclaimed & missing-guest policy, parallel scan
sib|src/discover|candidate discovery: `git ls-files` \| named paths, normalisation, symlink screen
sib|src/registry|language registry: `hosts()`, `guests()`, feature gates, feature names

## §C CONSTRAINTS

- C16: single config `xenolith.toml` at consumer repo root. replaces per-language allowlist files (`.nix-embedded-shell-allowlist`, `.pkl-embedded-shell-allowlist`, …).

## §I INTERFACES

- `[threshold.shell]`: `allow` ⊆ {`and-or`, `case`, `command-substitution`, `for`, `function-definition`, `heredoc`, `if`, `pipeline`, `redirect`, `sequence`, `subshell`, `while`} (= `languages/shells/shell` `Construct` names, ONE vocabulary) (default `[]`) — constructs tolerated inline, relaxes `languages/shells/shell:V3`. `[threshold.just]`: `max_lines` (default 1) only — lines a just recipe may run & stay inline, each judged alone (V240). `[threshold.<guest>]` ∀ other guest: `max_lines` (default 1), `max_bytes` (default 80) — inline ceiling applied on top of guest's own `trivial` rule.
- `[threshold.exec]`: `max_args` (default 8), `max_len` (default 120) — systemd `ExecStart*` line kept inline when within.
- top-level `version = 1`: required config schema version.
- defaults table (single source; ∀ entry overridable in `xenolith.toml`): `[extract] layout = "host"`, `root = "scripts"`, `depth = 5`, `inactive_rules = "warn"`; `[threshold.shell] allow = []`; `[threshold.just] max_lines = 1`; `[threshold.<guest>] max_lines = 1`, `max_bytes = 80`; `[threshold.exec] max_args = 8`, `max_len = 120`; `[langs] unclaimed = "ignore"`; `[threshold.load] max_params = 6`; `[parse] host_errors = "error"`; `[extract.shell] strict = "preserve"`; `[lint.<guest>] extend = true`; `[langs] missing_guest = "error"`; `[lint] hosts = true`; `[threshold.load] param_prefix = ""`; rule `base` = host's `runtime_base`.
- `[langs] unclaimed` ∈ `ignore` (default) \| `warn` \| `error`: file no host claims.
- `[threshold.load] max_params` (default 6): holes passed as params in a one-liner load (`languages/api/src/holes:V40`).
- `[parse] host_errors` ∈ `error` (default) \| `warn` \| `ignore`: claimed host file ⊥ parsed (`ERROR` nodes, BOM, ⊥ UTF-8) ∴ ⊥ checked; default ⊥ silent pass.
- `[[exclude]]`: `glob`, `reason` (required) — tracked files ∀ verb skips; default = scan ∀ tracked file.
- per-verb exclusion lists `[check] exclude`, `[extract] exclude`, `[lint] exclude`, `[graph] exclude` = `[{ glob, reason }]`, applied on top of `[[exclude]]`.
- `[[detect]]`: `host`?, `sink`? (glob), `path`? (glob), `guest` — forces guest for matching sites, overrides detection.
- `[langs] missing_guest` ∈ `error` (default) \| `warn` \| `ignore`: site whose guest is compiled out.
- file: `xenolith.toml` — schema & defaults in `src/config` §I: `version`, `[extract]`, `[[extract.rule]]`, `[extract.shell]`, `[[allow]] path, sink, hash, reason`, `[[exclude]]` & per-verb `exclude`, `[[detect]]`, `[lint.<guest>]`, `[langs]`, `[parse]`, `[threshold.*]`.
- discovery: `xenolith.toml` in any dir; file's effective config = merge root → file's dir (nearest last): scalars override, tables deep-merge, entry lists (`allow`, `exclude` & per-verb, `rule`, `detect`, `lint.all`, `checks`, `fixers`) append, `threshold.shell.allow` overrides as a scalar; globs & `[[allow]] path` relative to declaring file's dir; staleness judged within declaring file's subtree; root = dir `xnl` runs in, ⊥ ancestor above it (V88); ⊥ file = defaults (convention over configuration).
- `[threshold.load] param_prefix` (default `""`): prefix for derived param names (e.g. `XNL_`).

## §V INVARIANTS

V9: `[[allow]]` entry ! carry non-empty `reason`; entry matching nothing (stale) = violation. ⊥ wildcard path allow.
V10: allow key = host path + sink path (e.g. `systemd.services.foo.script`) + content hash of body; ⊥ line number, ⊥ byte span ∴ edits above site ⊥ break allow; edits to body ! invalidate allow.
V44: `[[extract.rule]]` checked @ load: unknown template var \| absolute path \| `..` escaping repo root → exit 2; rule matching ⊥ site in repo = `stale-rule` violation (∼ V9); rule for inactive language ⊥ stale → per `[extract] inactive_rules`.
V55: `[threshold]` validated @ load: unknown guest \| construct \| key, negative value → exit 2; threshold only RELAXES, ⊥ makes a guest-trivial body flagged.
V70: `version` missing \| unknown → exit 2 naming supported versions; ∀ `xenolith.toml` in one merge chain ! declare same `version`, else exit 2 naming both files.
V73: ∀ default value (number, policy, path) ∃ config key & entry in defaults table; engines ⊥ literal defaults — read resolved config only. `--verbose` prints ∀ effective value w/ source (`default` \| `xenolith.toml`).
V79: ∀ exclude entry (`[[exclude]]` & per-verb) ! carry non-empty `reason`; glob matching ⊥ tracked file = `stale-exclude` violation; excluded file ⊥ read.
V85: config reference `docs/config.md` generated from defaults table & §I schema by a test (`UPDATE=1` rewrites); gate: generated ≡ committed, ⊥ hand-edited.
V88: independently buildable: ∀ crate \| sherd node dir checkable standalone (`xnl check` inside it w/o ancestors, e.g. from crates.io package) → own `xenolith.toml` + defaults; ⊥ correctness depends on ancestor config.
V89: convention over configuration: key whose value ≡ inherited effective value → warning `redundant-config` ∴ configs hold only deviations.
V240: `[threshold.just] max_lines` = N: site whose `DelimKind::runs_line_by_line()` (`languages/ci/just:V180`), flagged whole, 2 ≤ lines ≤ N → re-judged per line (guest `unsupported`, `trivial`, `[threshold.<guest>]`): inline iff ∀ line passes, else why names the line; > N → why names line count & key. engine host-generic: `Config::line_ceiling(host)`, ⊥ `LangId::Just` in `src/check`; only RELAXES (V55).

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end | T10, T25, T49, T56, T70, T73, T80, T143 | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |
| M3 | publication | T89, T240 | public doc set, release machinery, history audit green, crates published (`.:T32`) |

id|status|task|cites
T10|x|`xenolith.toml` parser: extract layout & rules, allow (reason required, hash/span keyed), lint map, langs toggle|C16,V9,V10
T25|x|allow staleness check: unmatched `[[allow]]` = violation|V9
T49|.|parse `[extract]` layout & `[[extract.rule]]`; template validation & `stale-rule`|V44,T10
T56|x|parse & validate `[threshold]`; fixtures: allowed construct passes, unknown construct exits 2|V55
T70|.|`inactive_rules` ignore/warn/error; fixture on `lang-nix`-only build w/ pkl rule|V44
T73|x|defaults table as one Rust const module; test: ∀ key in table parsed & overridable; grep guard ⊥ literal default in engines|V73
T80|x|exclude parsing, reason required, `stale-exclude`; fixture: vendored dir excluded|V79
T89|.|generate `docs/config.md` from defaults & schema; drift test|V85,V73
T143|x|`src:C139` backfill: `src/config/defaults/tests.rs`|`src:C139`,`scripts/guard:V140`
T240|x|`[threshold.just] max_lines`: parse, default row, nested merge, `Config::line_ceiling`; engine per-line verdict via `DelimKind::runs_line_by_line`; tests: 2 lines flagged by default, clean at 2, 3 flagged, negative → exit 2|V240,V55,V73,`languages/ci/just:T185`

## §B BUGS

id|date|cause|fix
B1|2026-09-26|§I listed 14 construct names (`pipe`, `and`, `or`, `subst`, `backtick` …) while `languages/shells/shell` classifier emits 12 others (`pipeline`, `and-or`, `command-substitution` …) ∴ T56 validated names the classifier never produces; an `allow` naming a real construct was refused|§I adopts the classifier's names; `SHELL_CONSTRUCTS` follows
B2|2026-09-26|`Tree::load` rebuilt ∀ layer per file read ∴ O(N²) merges|1 merge/layer onto resolved parent
B3|2026-09-26|`Tree::load` read ∀ candidate's nested config before exclusion ∴ excluded tree's config read (V79), broken one → exit 2|load root → down, skip dir its chain excludes
