# SPEC

## §G GOAL

per-language linter map & invocation over extracts; missing linter binary = error.

## §N NAV

rel|path|lens
up|.|-
up|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
self|src/lint|per-language linter map & invocation
sib|src/config|`xenolith.toml` parse & validation
sib|src/extract|embed → own file, host rewrite, diff | `--write`
sib|src/graph|host → extract load edges, dangling & orphan

## §I INTERFACES

- default checks / fixers (all overridable; more = better): sh → `shellcheck -s sh`, `checkbashisms`, `shfmt -d`; bash → `shellcheck -s bash`, `shfmt -d` / `shfmt -w`; zsh → `zsh -n`; python → `ruff check`, `ruff format --check`, `mypy`, `bandit` / `ruff check --fix`, `ruff format`; sql → `sqlfluff lint`, `squawk` ? (postgres DDL) / `sqlfluff fix`; jq → `jq -n -f {file}`; awk → `gawk --lint -f {file} /dev/null`; ∀ extract → `typos`, `editorconfig-checker` ?.
- json (`xnl lint --format json`): `{"schema": 1, "results": [{"file": "scripts/hk/fmt.sh", "kind": "extract", "guest": "shell", "dialect": "bash", "check": "shellcheck", "argv": ["shellcheck", "-s", "bash", "-f", "json", "scripts/hk/fmt.sh"], "source": "default", "status": "fail", "exit": 1, "findings": [{"line": 3, "col": 7, "code": "SC2086", "severity": "warning", "message": "…"}], "raw_tail": null}], "violations": [], "warnings": []}`; `status` ∈ `pass` \| `fail` \| `error` \| `skipped` (untrusted \| excluded); `kind` ∈ `extract` \| `host` \| `site`.

## §V INVARIANTS

V8: `lint`: ∀ extract run ∀ check of its guest & `[lint] all`, ∀ host file ∀ `Host::checks` (when `[lint] hosts`) (defaults ∪ | replaced by `[lint.<guest>]`), each reported separately, ⊥ stop at first; binary absent from PATH = exit 2, ⊥ silent skip; `--fix` runs fixers then re-checks, touches extracts only.
V91: config-defined commands run only w/ `--trust-config`; untrusted → skipped w/ warning `untrusted-command` naming each, built-in defaults still run.
V92: findings parsed per `LintCmd.format` into `{line, col, code, severity, message}`; unparseable output → `raw_tail` (last 40 lines), ⊥ dropped; `fail` \| `error` → exit 1 \| 2 per `src` §I exit.
V93: `xnl lint --sites` lints in-host sites BEFORE extraction: body materialised to temp file via `wrap`, checks run, finding positions mapped back to host `file:line:col` through `Delim.body` & `unescape`; `kind: site`.

## §T TASKS

id|status|task|cites
T24|.|`lint`: per-language linter map w/ defaults (shellcheck+shfmt, ruff ?, sqlfluff ?, eslint ?, stylelint ?), missing binary = exit 2|V8
T87|.|multi-check runner, per-check results, `--fix`; defaults table; fixture w/ 2 checks both failing|V8
T92|.|`--trust-config` gate; fixture: config check skipped w/ warning, runs w/ flag|V91
T93|.|findings parsers (shellcheck json, ruff json, sqlfluff json, SARIF) + raw fallback|V92
T94|.|virtual-extract linting w/ source mapping; fixture: shellcheck finding in nix `script` reported at nix line|V93

## §B BUGS

id|date|cause|fix
