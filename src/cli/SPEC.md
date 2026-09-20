# SPEC

## §G GOAL

`xnl` command surface: verbs, flags, exit codes, rule ids, output formats (human, json, sarif), consumer hk wiring.

## §N NAV

rel|path|lens
up|.|-
up|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
self|src/cli|verbs, flags, exit codes, rule ids, output formats, hk wiring
sib|src/config|`xenolith.toml` parse & validation
sib|src/extract|embed → own file, host rewrite, diff \| `--write`
sib|src/graph|host → extract load edges, dangling & orphan
sib|src/lint|per-language linter map & invocation

## §I INTERFACES

- cmd: `xnl check [--format human|json|sarif] [paths…]` → detect embeds; exit 1 ∃ violation.
- cmd: `xnl extract [--write] [--relocate] <path>[:line]…` → move embed to extract file, rewrite host to load it; ≥1 path, each ∀ its sites or one `:line`; default = print unified diff (all paths, sorted); exit 1 if diff non-empty.
- cmd: `xnl graph [--format human|json|sarif] [paths…]` → host → extract load edges; flags orphan extracts & dangling loads.
- cmd: `xnl lint [--fix] [paths…]` → run ∀ configured check ∀ extract by language; `--fix` runs fixers then re-checks, extracts only; missing binary = error, ⊥ skip.
- cmd: `xnl langs [--format human|json]` → ∀ known language (`LangId`): role host \| guest \| both, compiled in (feature `lang-<lang>` on) y/n, sinks, delimiter kinds, load idiom, default linter.
- flag: `--verbose` ∀ verb; silence = success otherwise.
- flag: `--strict-hosts` ∀ verb: unclaimed file → exit 2 (≡ `[langs] unclaimed = "error"`).
- warnings: human → stderr, json → `warnings[]`; ⊥ change exit code.
- exit: 0 ok · 1 violation | drift · 2 usage | config error | refused. several apply → highest wins (2 > 1 > 0).
- rules: `xenolith` (non-trivial guest in sink; holes → same rule w/ `Judgment` direction, `languages/api/src/holes:V40`), `dangling-load` & `orphan-extract` (`src/graph:V7`), `stale-allow` (`src/config:V9`), `stale-rule` (`src/config:V44`), `host-parse-error` (`languages:V78`), `stale-exclude` (`src/config:V79`). kebab-case, stable ∀ schema version.
- human output: `file:line:col xenolith: <guest> in <host> <sink> (<why>)`; `xnl extract` diff header `removing xenolith → <extract path>`. metaphor lives in rule id & wording, verbs stay conventional (`check`, `extract`, `graph`, `lint`, `langs`) ∼ fleet `mth check`, `sherd check`.
- json: violation = `{rule, file, line, col, host, guest, sink, site, why, directions[]}`; `site` = `DelimKind` (`languages/api` §I); each direction `Mechanical`|`Judgment` (mirror microlith shape).
- json envelope: `{"schema": 1, "violations": [...], "warnings": [{"code", "file"?, "message"}]}`; keys sorted, arrays sorted per `src:V11`.
- hk: consumer step `check = "xnl check {{files}}"`, `check_diff = "xnl extract {{files}}"` (hk shows proposed extraction), `fix = "xnl extract --write {{files}}"` (explicit `hk fix` only, C15); `xnl graph`, `xnl lint` as own steps.
- flag: `--trust-config`: permit commands defined in any `xenolith.toml` (`[lint.<guest>]` checks/fixers, `[lint] all`); ⊥ config key can grant it.
- cmd: `xnl lint` gains `--format human|json|sarif` (see `src/lint` §I).
- cmd: `xnl init` → write minimal `xenolith.toml` (`version = 1`) in cwd; ⊥ overwrite; prints detected languages & suggested deviations, writes none (convention over configuration).
- cmd: `xnl migrate [--write]` → read `.nix-embedded-shell-allowlist`, `.pkl-embedded-shell-allowlist` (& siblings) → `[[allow]]` entries keyed per `src/config:V10`, `reason` = original comment \| `migrated from <file>`; default prints diff.
- cmd: `xnl inline [--write] <extract>…` → put extract body back into host via lens inverse (`languages/api/src/lens:V34`); default prints diff; refuses (exit 2) if extract loaded by >1 host.
- json (`xnl langs`): envelope + `langs`: `[{"id": "shell", "role": "both", "compiled_in": true, "feature": "lang-shell", "sinks": [...], "delims": [...], "checks": [...], "fixers": [...]}]`.
- human (`graph`, `lint`): one line per item `file:line:col <rule|check>: <message>`, summary line last (`N edges, N violations` \| `N checks, N failed`), silent on success unless `--verbose`.

## §V INVARIANTS

V24: exit codes stable: 0 ok, 1 violation, 2 usage/config/unsupported. json schema versioned (`"schema": 1`): ONE number ∀ verbs; any shape change in any verb bumps it.
V102: `--format sarif` = SARIF 2.1.0: rule id → `ruleId`, `file:line:col` → `physicalLocation`, `why` → `message`, `Mechanical` direction → `fixes` when diff known; lint findings keep tool `code` as `ruleId` under tool's own `run`; deterministic like `src:V11`.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end | T9 | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |
| M3 | publication | T96, T97, T103 | public doc set, release machinery, history audit green, crates published (`.:T32`) |

id|status|task|cites
T9|.|CLI skeleton `xnl` (check, extract, graph, lint, langs; `--format`, `--verbose`); exit codes|I.cmd,V24,`src:V13`
T96|.|`xnl init` + fixture: empty repo, existing config refused|`src/config:V89`
T97|.|`xnl migrate` + fixtures per legacy allowlist format|`src/config:V10`
T103|.|SARIF writer ∀ verb + schema validation test|V102

## §B BUGS

id|date|cause|fix
