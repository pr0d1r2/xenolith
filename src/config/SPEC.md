# SPEC

## §G GOAL

`xenolith.toml` parse: extract layout & rules, `[[allow]]` (reason, hash|span, staleness), lint map, langs toggle, thresholds.

## §N NAV

rel|path|lens
up|.|-
up|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
self|src/config|`xenolith.toml` parse & validation
sib|src/extract|embed → own file, host rewrite, diff | `--write`
sib|src/graph|host → extract load edges, dangling & orphan
sib|src/lint|per-language linter map & invocation

## §I INTERFACES

- `[extract]`: `layout` ∈ `host` (default: host placement only) \| `mirror` (`<root>/<host path sans ext>/<name>.<ext>`) \| `sibling` (`<host_dir>/<host_stem>.<name>.<ext>`) \| `central` (`<root>/<guest>/<name>.<ext>`); `root` (default `scripts`). layer C.
- `[[extract.rule]]`: match `host`, `sink` (glob, `*` = one dotted segment), `guest` — each optional, ≥1 required; set any of `path` (template), `base` (`host` \| `root` \| `"<dir>"`, overrides `Host::runtime_base`), `invoke` (argv template, overrides `Guest::invoke`), `prelude` (`{ shebang, strict }`), `executable`, `companion` (template). layer B, highest precedence.
- `[threshold.shell]`: `allow` ⊆ {`pipe`, `and`, `or`, `seq`, `subst`, `backtick`, `redirect`, `if`, `for`, `while`, `case`, `heredoc`, `subshell`, `function`} (default `[]`) — constructs tolerated inline, relaxes `languages/shell:V3`. `[threshold.<guest>]` ∀ other guest: `max_lines` (default 1), `max_bytes` (default 80) — inline ceiling applied on top of guest's own `trivial` rule.
- `[extract] depth` (default 5, ≥1): max nesting levels extracted in one run.
- `[extract] inactive_rules` ∈ `ignore` \| `warn` (default) \| `error`: handling of `[[extract.rule]]` whose host or guest is compiled out | `[langs]`-disabled.
- `[threshold.exec]`: `max_args` (default 8), `max_len` (default 120) — systemd `ExecStart*` line kept inline when within.
- top-level `version = 1`: required config schema version.
- defaults table (single source; ∀ entry overridable in `xenolith.toml`): `[extract] layout = "host"`, `root = "scripts"`, `depth = 5`, `inactive_rules = "warn"`; `[threshold.shell] allow = []`; `[threshold.<guest>] max_lines = 1`, `max_bytes = 80`; `[threshold.exec] max_args = 8`, `max_len = 120`; `[langs] unclaimed = "ignore"`; `[threshold.load] max_params = 6`; `[parse] host_errors = "warn"`; `[extract.shell] strict = "preserve"`; `[lint.<guest>] extend = true`; rule `base` = host's `runtime_base`.
- `[langs] unclaimed` ∈ `ignore` (default) \| `warn` \| `error`: file no host claims.
- `[threshold.load] max_params` (default 6): holes passed as params in a one-liner load (`languages/api:V40`).
- `[parse] host_errors` ∈ `warn` (default) \| `error` \| `ignore`: host file w/ parse `ERROR` nodes.
- `[[exclude]]`: `glob`, `reason` (required) — tracked files ∀ verb skips; default = scan ∀ tracked file.
- per-verb exclusion lists `[check] exclude`, `[extract] exclude`, `[lint] exclude`, `[graph] exclude` = `[{ glob, reason }]`, applied on top of `[[exclude]]`.
- `[[detect]]`: `host`?, `sink`? (glob), `path`? (glob), `guest` — forces guest for matching sites, overrides detection.
- `[extract.shell] strict` ∈ `preserve` (default) \| `enforce`: `enforce` → prelude `set -euo pipefail` regardless of context, diff marks it `Judgment` (semantic change).
- `[lint.<guest>]`: `checks = ["<cmd> {file}", …]`, `fixers = [...]`, `extend` (default `true`: append to guest defaults; `false`: replace).

## §V INVARIANTS

V9: `[[allow]]` entry ! carry non-empty `reason`; entry matching nothing (stale) = violation. ⊥ wildcard path allow.
V10: allow key = host path + sink path (e.g. `systemd.services.foo.script`) + content hash of body; ⊥ line number, ⊥ byte span ∴ edits above site ⊥ break allow; edits to body ! invalidate allow.
V44: `[[extract.rule]]` checked @ load: unknown template var \| absolute path \| `..` escaping repo root → exit 2; rule matching ⊥ site in repo = `stale-rule` violation (∼ V9); rule for inactive language ⊥ stale → per `[extract] inactive_rules`.
V55: `[threshold]` validated @ load: unknown guest \| construct \| key, negative value → exit 2; threshold only RELAXES, ⊥ makes a guest-trivial body flagged.
V70: `version` missing \| unknown → exit 2 naming supported versions.
V73: ∀ default value (number, policy, path) ∃ config key & entry in defaults table; engines ⊥ literal defaults — read resolved config only. `--verbose` prints ∀ effective value w/ source (`default` \| `xenolith.toml`).
V79: ∀ exclude entry (`[[exclude]]` & per-verb) ! carry non-empty `reason`; glob matching ⊥ tracked file = `stale-exclude` violation; excluded file ⊥ read.

## §T TASKS

id|status|task|cites
T10|.|`xenolith.toml` parser: extract layout & rules, allow (reason required, hash/span keyed), lint map, langs toggle|C16,V9,V10
T25|.|allow staleness check: unmatched `[[allow]]` = violation|V9
T49|.|parse `[extract]` layout & `[[extract.rule]]`; template validation & `stale-rule`|V44,T10
T56|.|parse & validate `[threshold]`; fixtures: allowed construct passes, unknown construct exits 2|V55
T70|.|`inactive_rules` ignore/warn/error; fixture on `lang-nix`-only build w/ pkl rule|V44
T73|.|defaults table as one Rust const module; test: ∀ key in table parsed & overridable; grep guard ⊥ literal default in engines|V73
T80|.|exclude parsing, reason required, `stale-exclude`; fixture: vendored dir excluded|V79

## §B BUGS

id|date|cause|fix
