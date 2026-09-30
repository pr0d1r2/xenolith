# SPEC

## §G GOAL

check engine (`src/check.rs`, `xenolith::check`): candidates → claims → sites → guests → violations & warnings, parallel & sorted.

## §N NAV

rel|path|lens
up|.|-
up|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
self|src/check|check engine: candidates → claims → sites → guests → violations; unclaimed & missing-guest policy, parallel scan
sib|src/config|`xenolith.toml` parse & validation
sib|src/extract|embed → own file, host rewrite, diff \| `--write`
sib|src/graph|host → extract load edges, dangling & orphan
sib|src/lint|per-language linter map & invocation
sib|src/cli|verbs, flags, exit codes, rule ids, output formats, hk wiring
sib|src/discover|candidate discovery: `git ls-files` \| named paths, normalisation, symlink screen
sib|src/registry|language registry: `hosts()`, `guests()`, feature gates, feature names

## §V INVARIANTS
V13: unclaimed file (0 hosts claim, named or not) ⊥ scanned & ⊥ reported by default; `--strict-hosts` \| `[langs] unclaimed = "error"` → exit 2 "host unsupported"; `warn` → warning.
V42: guest compiled out: site whose `guest` ∉ `guests()` → per `[langs] missing_guest` (default `error`: exit 2 naming guest & its feature `lang-<guest>` if ∃; `warn` → warning; `ignore`); guest named by shebang (`Host::guest_by_shebang`) → warning `missing-guest` always; ⊥ guessing trivial/non-trivial.
V95: scan parallel per file, results merged then sorted (`src:V11`) ∴ output byte-identical to serial run; `--jobs N` (default cores).
V120: scan throughput recorded (files/s over the M2 corpus & a synthetic large tree); regression beyond recorded budget = warning `slow-scan`, ⊥ gate failure (timing noise); budget raise only w/ Why.
V152: `xenolith::check` = ONE engine: candidates (`src/discover:V57`, `src/discover:V128`, `src/config:V79`) → registry hosts claim (`src/registry:V41`, V13) → `Host::sites` → guest ∈ `guests()` (V42) → `Guest::trivial` relaxed by `[threshold]` (`src/config:V55`) → non-trivial ∧ ⊥ `[[allow]]` → `Violation` (`src:V1`); unmatched allow → `stale-allow` (`src/config:V9`). `src/cli` renders & maps exit codes only.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end |  | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |
| M3 | publication | T98, T119 | public doc set, release machinery, history audit green, crates published (`.:T32`) |

id|status|task|cites
T98|.|parallel scan + determinism test (serial vs `--jobs 8` byte-equal)|V95,`src:V11`
T119|.|benchmark harness + recorded budget file|V120,V95

## §B BUGS

id|date|cause|fix
B1|2026-09-26|`Report::push` keyed (file,line,col,rule), `warn` (code,file): ties w/ differing rendered fields kept arrival order ∴ parallel scan (V95) → bytes vary run to run|key extended to ∀ rendered field (`report_order`, + message); `src:V11` ⊇ ties
B2|2026-09-26|file marked scanned before its host parsed it ∴ parse error (syntax, ⊥ UTF-8) → its `[[allow]]` all `stale-allow`|staleness ⊥ judged ∀ file ⊥ parsed
B3|2026-09-26|whole-tree run judged ∀ `[[allow]]` ∴ allow for file ⊥ scanned (host compiled out, excluded) → `stale-allow`|judge allow only ∀ scanned file \| ⊥ candidate
B4|2026-09-26|`repo_name` kept absolute path absolute ∴ ⊥ met allow, exclude, nested config; path outside root scanned|named path → root-relative (lexical \| canonical root); outside → exit 2
B5|2026-09-26|engine ⊥ called `stale_excludes` ∴ stale exclude ⊥ reported (`src/config:V79`)|whole-tree run: warning `stale-exclude` ∀ layer; violation shape open (`src:V1`)
B6|2026-09-26|extract direction `Mechanical` (even ∀ unparseable body) while `xnl extract` refuses ∀ host (`src/extract:T22` open) ∴ SARIF fix nothing applies|`Judgment`, says by hand until T22
B7|2026-09-26|direction command printed path unquoted ∴ space \| quote → pastes as ≠ words|path quoted as 1 POSIX shell word
B8|2026-09-26|`languages/ci/nix:T157` shebang site → guest ∉ any build (python …) → V42 exit 2 ∀ run, shell findings hidden; message named `lang-python`, ⊥ ∃|`Host::guest_by_shebang` → warning always; message names feature only if ∃ (`FEATURED`)
B10|2026-09-26|`--strict-hosts` refusal built `lang-<id>` from the extension's language ∴ `.py` → "rebuild with feature `lang-python`", ⊥ ∃ (B8 fixed guests only)|name the feature only if it exists (`registry::existing_feature`), else "no support … yet"
B12|2026-09-27|extract direction `Judgment` ∀ site after `src/extract:T22` landed (B6: until T22)|`Mechanical` `xnl extract <file>:<line>` iff `src/extract` `viable`, else `Judgment` w/ its why
