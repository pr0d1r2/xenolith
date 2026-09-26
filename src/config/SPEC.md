# SPEC

## §G GOAL

`xenolith.toml` format, discovery, merge, validation & defaults table; engine-specific sections live in their engine node (`[lint.*]` → `src/lint`, `[extract]` & `[[extract.rule]]` → `src/extract`, `[extract.shell]` → `languages/shell`); here: extract layout & rules, `[[allow]]` (reason, hash|span, staleness), lint map, langs toggle, thresholds.

## §N NAV

rel|path|lens
up|.|-
up|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
self|src/config|`xenolith.toml` parse & validation
sib|src/extract|embed → own file, host rewrite, diff \| `--write`
sib|src/graph|host → extract load edges, dangling & orphan
sib|src/lint|per-language linter map & invocation
sib|src/cli|verbs, flags, exit codes, rule ids, output formats, hk wiring

## §C CONSTRAINTS

- C16: single config `xenolith.toml` at consumer repo root. replaces per-language allowlist files (`.nix-embedded-shell-allowlist`, `.pkl-embedded-shell-allowlist`, …).

## §I INTERFACES

- `[threshold.shell]`: `allow` ⊆ {`pipe`, `and`, `or`, `seq`, `subst`, `backtick`, `redirect`, `if`, `for`, `while`, `case`, `heredoc`, `subshell`, `function`} (default `[]`) — constructs tolerated inline, relaxes `languages/shell:V3`. `[threshold.<guest>]` ∀ other guest: `max_lines` (default 1), `max_bytes` (default 80) — inline ceiling applied on top of guest's own `trivial` rule.
- `[threshold.exec]`: `max_args` (default 8), `max_len` (default 120) — systemd `ExecStart*` line kept inline when within.
- top-level `version = 1`: required config schema version.
- defaults table (single source; ∀ entry overridable in `xenolith.toml`): `[extract] layout = "host"`, `root = "scripts"`, `depth = 5`, `inactive_rules = "warn"`; `[threshold.shell] allow = []`; `[threshold.<guest>] max_lines = 1`, `max_bytes = 80`; `[threshold.exec] max_args = 8`, `max_len = 120`; `[langs] unclaimed = "ignore"`; `[threshold.load] max_params = 6`; `[parse] host_errors = "warn"`; `[extract.shell] strict = "preserve"`; `[lint.<guest>] extend = true`; `[langs] missing_guest = "error"`; `[lint] hosts = true`; `[threshold.load] param_prefix = ""`; rule `base` = host's `runtime_base`.
- `[langs] unclaimed` ∈ `ignore` (default) \| `warn` \| `error`: file no host claims.
- `[threshold.load] max_params` (default 6): holes passed as params in a one-liner load (`languages/api/src/holes:V40`).
- `[parse] host_errors` ∈ `warn` (default) \| `error` \| `ignore`: host file w/ parse `ERROR` nodes.
- `[[exclude]]`: `glob`, `reason` (required) — tracked files ∀ verb skips; default = scan ∀ tracked file.
- per-verb exclusion lists `[check] exclude`, `[extract] exclude`, `[lint] exclude`, `[graph] exclude` = `[{ glob, reason }]`, applied on top of `[[exclude]]`.
- `[[detect]]`: `host`?, `sink`? (glob), `path`? (glob), `guest` — forces guest for matching sites, overrides detection.
- `[langs] missing_guest` ∈ `error` (default) \| `warn` \| `ignore`: site whose guest is compiled out.
- file: `xenolith.toml` — schema & defaults in `src/config` §I: `version`, `[extract]`, `[[extract.rule]]`, `[extract.shell]`, `[[allow]] path, sink, hash, reason`, `[[exclude]]` & per-verb `exclude`, `[[detect]]`, `[lint.<guest>]`, `[langs]`, `[parse]`, `[threshold.*]`.
- discovery: `xenolith.toml` in any dir; file's effective config = merge root → file's dir (nearest last): scalars override, tables deep-merge, lists (`allow`, `exclude`, `rule`, `detect`, checks) append; globs relative to declaring file; staleness judged within declaring file's subtree; ⊥ file = defaults (convention over configuration).
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

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end | T10, T25, T49, T56, T70, T73, T80, T143 | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |
| M3 | publication | T89 | public doc set, release machinery, history audit green, crates published (`.:T32`) |

id|status|task|cites
T10|x|`xenolith.toml` parser: extract layout & rules, allow (reason required, hash/span keyed), lint map, langs toggle|C16,V9,V10
T25|.|allow staleness check: unmatched `[[allow]]` = violation|V9
T49|.|parse `[extract]` layout & `[[extract.rule]]`; template validation & `stale-rule`|V44,T10
T56|.|parse & validate `[threshold]`; fixtures: allowed construct passes, unknown construct exits 2|V55
T70|.|`inactive_rules` ignore/warn/error; fixture on `lang-nix`-only build w/ pkl rule|V44
T73|x|defaults table as one Rust const module; test: ∀ key in table parsed & overridable; grep guard ⊥ literal default in engines|V73
T80|.|exclude parsing, reason required, `stale-exclude`; fixture: vendored dir excluded|V79
T89|.|generate `docs/config.md` from defaults & schema; drift test|V85,V73
T143|.|`src:C139` backfill: `src/config/defaults/tests.rs`|`src:C139`,`scripts/guard:V140`

## §B BUGS

id|date|cause|fix
