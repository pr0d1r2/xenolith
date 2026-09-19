# SPEC

## §G GOAL

move embed → own file & rewrite host to load it: diff default, `--write`, lossless, idempotent, collision guard.

## §N NAV

rel|path|lens
up|.|-
up|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
self|src/extract|embed → own file, host rewrite, diff | `--write`
sib|src/config|`xenolith.toml` parse & validation
sib|src/graph|host → extract load edges, dangling & orphan
sib|src/lint|per-language linter map & invocation

## §V INVARIANTS

V4: `extract` LOSSLESS: extracted file bytes + host rewrite ! round-trip — inlining extract back into host reproduces original semantics (normalized whitespace equal); asserted before write.
V5: `extract` IDEMPOTENT: `extract(extract(x)) == extract(x)`; rerun on clean host = no-op.
V6: `extract --write` ⊥ overwrite existing file ≠ same bytes; collision → exit 2 w/ message.
V45: placement resolved PER FIELD (path, name, invoke, header, executable, companion): most specific matching `[[extract.rule]]` (B) > `[extract] layout` (C) > `Host::placement` / `Guest` defaults (D). specificity = count of matched keys (host, sink, guest); tie between rules = exit 2 naming both.
V46: template vars closed set `{name}`, `{ext}`, `{host_dir}`, `{host_stem}`, `{sink}`, `{guest}`, `{path}`, `{path_stem}`; render deterministic, result normalized relative to repo root.
V47: 2 sites resolving to same path → disambiguate by appending sink segment (`foo-script`, `foo-prestart`), deterministic; still equal → V6 exit 2.
V48: `xnl extract --verbose` prints ∀ site ∀ field the deciding layer (`rule #n` \| `layout` \| `host` \| `guest`) ∴ placement explainable, ⊥ guessed.
V49: `companion` configured → `--write` creates companion stub w/ extract; V4–V6 cover companion too: existing companion ⊥ rewritten, rerun = no-op, differing existing file → exit 2.

## §T TASKS

id|status|task|cites
T22|.|`extract` nix + pkl + yaml + bash (first wave): diff default, `--write`, lossless & idempotent asserts, collision guard|V4,V5,V6,C15
T23|.|`extract` remaining hosts (just, Dockerfile, rust, ruby, html)|V4,V5,V6
T50|.|placement resolver: per-field precedence, templates, collision suffix, `--verbose` explain|V45,V46,V47,V48

## §B BUGS

id|date|cause|fix
