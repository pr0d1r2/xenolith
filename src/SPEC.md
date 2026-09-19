# SPEC

## §G GOAL

root crate: lib + `xnl` bin: core model, CLI, verbs; cross-language engines extract/graph/lint/config as child nodes.

## §F FEDERATION

dir|owns|⊥owns|tokens
config|`xenolith.toml` parse & validation|applying config — each verb's node|-
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

V1: ∀ violation carries `rule`, `file:line:col`, host lang, guest lang, sink, site delimiter kind, `why`, ≥1 direction. ⊥ bare "bad".
V11: deterministic: output order sorted (file, line, col); json byte-stable across runs & platforms.
V13: unclaimed file (0 hosts claim, named or not) ⊥ scanned & ⊥ reported by default; `--strict-hosts` \| `[langs] unclaimed = "error"` → exit 2 "host unsupported"; `warn` → warning.
V24: exit codes stable: 0 ok, 1 violation, 2 usage/config/unsupported. json schema versioned (`"schema": 1`).
V41: registry = ONE file in root crate: `hosts() -> &'static [&'static dyn Host]`, `guests() -> &'static [&'static dyn Guest]`, each entry behind `#[cfg(feature = "lang-<lang>")]`, sorted by `LangId`. engines iterate registry; ⊥ `cfg(feature = "lang-*")` elsewhere (`.:V30` no-leak made checkable).
V42: guest compiled out: site whose `guest` ∉ `guests()` → per `[langs] missing_guest` (default `error`: exit 2 naming feature `lang-<guest>`; `warn` → warning; `ignore`); ⊥ guessing trivial/non-trivial.
V57: `xnl check|graph|lint` w/ ⊥ paths → candidates = `git ls-files` (tracked only ∴ `.gitignore` honoured); ⊥ git repo & ⊥ paths → exit 2 usage.

## §T TASKS

id|status|task|cites
T8|.|core model: `Violation`, `Direction` over api `Site`/`LangId`; json schema v1; sorted output|V1,V11,V24,`languages/api:T42`
T9|.|CLI skeleton `xnl` (check, extract, graph, lint, langs; `--format`, `--verbose`); exit codes|I.cmd,V24,V13
T46|.|registry file + compiled-out guest exit 2; test builds w/ `lang-nix` only & asserts nix→shell site exits 2 naming `lang-shell`|V41,V42,`.:V30`
T58|.|candidate discovery via `git ls-files`; fixture: untracked & ignored files ⊥ scanned; outside git → exit 2|V57
T75|.|unclaimed handling: default ignore, `--strict-hosts` & config error/warn; fixture: hk-style file list w/ `.png`, `.md`|V13
T88|.|`missing_guest` error/warn/ignore; fixture on `lang-nix`-only build|V42

## §B BUGS

id|date|cause|fix
