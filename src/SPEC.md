# SPEC

## §G GOAL

root crate: lib + `lyd` bin: core model, CLI, verbs; cross-language engines extract/graph/lint/config as child nodes.

## §F FEDERATION

dir|owns|⊥owns|tokens
config|`lydite.toml` parse & validation|applying config — each verb's node|-
extract|embed → own file, host rewrite, diff \| `--write`|detection (`languages`), load resolution (`graph`)|-
graph|host → extract load edges, dangling & orphan|writing files (`extract`)|-
lint|per-language linter map & invocation|deciding what is an extract (`graph`, `config`)|-

## §N NAV

rel|path|lens
up|.|-
self|src|root crate: lib + `lyd` bin, core model, CLI, cross-language engines extract/graph/lint/config
sib|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
sib|scripts|∀ shell in repo: dev shell hook, guardrail scripts
sib|tests|fixtures per host & case, integration tests, bats mirroring `scripts/`

## §V INVARIANTS

V1: ∀ violation carries `rule`, `file:line:col`, host lang, embedded lang, sink, `why`, ≥1 direction. ⊥ bare "bad".
V11: deterministic: output order sorted (file, line, col); json byte-stable across runs & platforms.
V13: unsupported host file ⊥ silently passed when named explicitly — `lyd check x.foo` → exit 2 "host unsupported".
V24: exit codes stable: 0 ok, 1 violation, 2 usage/config/unsupported. json schema versioned (`"schema": 1`).

## §T TASKS

id|status|task|cites
T8|.|core model: `Host`, `Sink`, `Embed`, `Violation`, `Direction`; json schema v1; sorted output|V1,V11,V24
T9|.|CLI skeleton `lyd` (check, extract, graph, lint, hosts; `--format`, `--verbose`); exit codes|I.cmd,V24,V13

## §B BUGS

id|date|cause|fix
