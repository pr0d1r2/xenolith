# SPEC

## §G GOAL

root crate: lib + `xnl` bin: core model, CLI, verbs; cross-language engines extract/graph/lint/config as child nodes.

## §F FEDERATION

dir|owns|⊥owns|tokens
config|`lydite.toml` parse & validation|applying config — each verb's node|-
extract|embed → own file, host rewrite, diff \| `--write`|detection (`languages`), load resolution (`graph`)|-
graph|host → extract load edges, dangling & orphan|writing files (`extract`)|-
lint|per-language linter map & invocation|deciding what is an extract (`graph`, `config`)|-

## §N NAV

rel|path|lens
up|.|-
self|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
sib|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
sib|scripts|∀ shell in repo: dev shell hook, guardrail scripts
sib|tests|fixtures per host & case, integration tests, bats mirroring `scripts/`

## §V INVARIANTS

V1: ∀ violation carries `rule`, `file:line:col`, host lang, embedded lang, sink, `why`, ≥1 direction. ⊥ bare "bad".
V11: deterministic: output order sorted (file, line, col); json byte-stable across runs & platforms.
V13: unsupported host file ⊥ silently passed when named explicitly — `xnl check x.foo` → exit 2 "host unsupported".
V24: exit codes stable: 0 ok, 1 violation, 2 usage/config/unsupported. json schema versioned (`"schema": 1`).
V41: registry = ONE file in root crate: `hosts() -> &'static [&'static dyn Host]`, `guests() -> &'static [&'static dyn Guest]`, each entry behind `#[cfg(feature = "lang-<lang>")]`, sorted by `LangId`. engines iterate registry; ⊥ `cfg(feature = "lang-*")` elsewhere (`.:V30` no-leak made checkable).
V42: guest compiled out: site whose `guest` ∉ `guests()` → exit 2 naming feature `lang-<guest>`, ⊥ silent skip, ⊥ guessing trivial/non-trivial ∴ same repo ⊥ passes on smaller build (C3).

## §T TASKS

id|status|task|cites
T8|.|core model: `Violation`, `Direction` over api `Site`/`LangId`; json schema v1; sorted output|V1,V11,V24,`languages/api:T42`
T9|.|CLI skeleton `xnl` (check, extract, graph, lint, hosts; `--format`, `--verbose`); exit codes|I.cmd,V24,V13
T46|.|registry file + compiled-out guest exit 2; test builds w/ `lang-nix` only & asserts nix→shell site exits 2 naming `lang-shell`|V41,V42,`.:V30`

## §B BUGS

id|date|cause|fix
