# SPEC

## §G GOAL

`xenolith.toml` parse: extract dirs, `[[allow]]` (reason, hash|span, staleness), lint map, langs toggle, thresholds.

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

## §V INVARIANTS

V9: `[[allow]]` entry ! carry non-empty `reason`; entry matching nothing (stale) = violation. ⊥ wildcard path allow.
V10: allow keyed by content hash | span, ⊥ line number alone ∴ edits above embed ⊥ break allow; edits to embed itself ! invalidate allow.

## §T TASKS

id|status|task|cites
T10|.|`xenolith.toml` parser: extract dirs, allow (reason required, hash/span keyed), lint map, langs toggle|C16,V9,V10
T25|.|allow staleness check: unmatched `[[allow]]` = violation|V9

## §B BUGS

id|date|cause|fix
