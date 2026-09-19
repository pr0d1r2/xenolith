# SPEC

## §G GOAL

host → extract load edges; dangling load & orphan extract detection.

## §N NAV

rel|path|lens
up|.|-
up|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
self|src/graph|host → extract load edges, dangling & orphan
sib|src/config|`xenolith.toml` parse & validation
sib|src/extract|embed → own file, host rewrite, diff | `--write`
sib|src/lint|per-language linter map & invocation

## §V INVARIANTS

V7: `graph`: ∀ load in host resolves to existing file (dangling = violation); ∀ file under extract roots (V50) loaded by ≥1 host (orphan = violation).
V50: extract roots = static prefix (before first `{`) of ∀ rule `path` + layout `root` + ∀ `Host::placement` dir; orphan scan (V7) walks exactly these, ⊥ whole repo.
V72: orphan scan & load resolution ⊥ follow symlinks; load resolving through symlink → `dangling-load`.

## §T TASKS

id|status|task|cites
T21|.|`graph`: load-edge extraction per host idiom; dangling & orphan detection|V7
T52|.|extract roots from rules, layout & host dirs; orphan scan over roots only|V50,V7

## §B BUGS

id|date|cause|fix
