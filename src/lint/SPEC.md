# SPEC

## §G GOAL

per-language linter map & invocation over extracts; missing linter binary = error.

## §N NAV

rel|path|lens
up|.|-
up|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
self|src/lint|per-language linter map & invocation
sib|src/config|`xenolith.toml` parse & validation
sib|src/extract|embed → own file, host rewrite, diff \| `--write`
sib|src/graph|host → extract load edges, dangling & orphan
sib|src/cli|verbs, flags, exit codes, rule ids, output formats, hk wiring
sib|src/check|check engine: candidates → claims → sites → guests → violations; unclaimed & missing-guest policy, parallel scan
sib|src/discover|candidate discovery: `git ls-files` \| named paths, normalisation, symlink screen
sib|src/registry|language registry: `hosts()`, `guests()`, feature gates, feature names

## §I INTERFACES

- default checks / fixers (all overridable; more = better): sh → `shellcheck -s sh`, `checkbashisms`, `shfmt -d`; bash → `shellcheck -s bash`, `shfmt -d` / `shfmt -w`; zsh → `zsh -n`; python → `ruff check`, `ruff format --check`, `mypy`, `bandit` / `ruff check --fix`, `ruff format`; sql → `sqlfluff lint`, `squawk` ? (postgres DDL) / `sqlfluff fix`; jq → `jq -n -f {file}`; awk → `gawk --lint -f {file} /dev/null`; ∀ extract → `typos`, `editorconfig-checker` ?.
- json (`xnl lint --format json`): `{"schema": 1, "results": [{"file": "scripts/hk/fmt.sh", "kind": "extract", "guest": "shell", "dialect": "bash", "check": "shellcheck", "argv": ["shellcheck", "-s", "bash", "-f", "json", "scripts/hk/fmt.sh"], "source": "default", "status": "fail", "exit": 1, "findings": [{"line": 3, "col": 7, "code": "SC2086", "severity": "warning", "message": "…"}], "raw_tail": null}], "violations": [], "warnings": []}`; `status` ∈ `pass` \| `fail` \| `error` \| `skipped` (untrusted \| excluded); `kind` ∈ `extract` \| `host` \| `site`.
- `[lint.<guest>]`: `checks = ["<cmd> {file}", …]`, `fixers = [...]`, `extend` (default `true`: append to guest defaults; `false`: replace).
- `[lint] hosts` (default `true`): run `Host::checks` on host files; `[lint] all = ["<cmd> {file}", …]`: checks ∀ extract regardless of guest (e.g. `typos`, `editorconfig-checker`).
- `[lint] timeout` (default 60s): per check \| fixer wall clock; 0 = no limit.
- targets (until `src/graph` names extracts): extract = candidate whose shebang names a compiled-in guest (dialect = interpreter basename) \| ⊥ shebang & ext ≡ `Guest::extension` (dialect ⊥); host file = host-claimed; neither → `src/check:V13`; excluded ⊥ result. cwd = root, file repo-relative.
- config cmd: whitespace-split words, ⊥ shell, ⊥ quoting; word `{file}` → path, else appended.
- status: exit 0 `pass`, ≠0 `fail`; ⊥ spawned \| signal \| timeout → `error`, why in `raw_tail`; `raw_tail` only when ⊥ `pass`; fixer listed only when ⊥ `pass`. human: `file: <check>: <why>` + tail, `N checks, N failed` last.
- findings (V92), from stdout only, by `LintCmd.format`: `Json("shellcheck")` (array \| json1 `{comments}`) → `SC<code>`, severity = `level`, col counted w/ tab = 1 char (json's tab stops of 8 undone against the file); `Json("ruff")` → `location.row/column`, `code` (null → `""`), severity `error`; `Json("sqlfluff")` → ∀ file ∀ violation `start_line_no|line_no`, `start_line_pos|line_pos`, `code`, `description`, severity `warning` iff `warning: true` else `error`; `Sarif` → ∀ run ∀ result `ruleId`, `level` (default `warning`), `message.text`, 1st location `region.startLine/startColumn` (absent → 0 \| 1). other `Json` name, `Raw`, config cmd, unparseable → `findings: []`. findings sorted (line, col, code, message); `raw_tail` null iff ≥1 finding. human: ∀ finding `file:line:col <check>: <code> (<severity>) <message>` in place of the tail.
- sites (V93, `--sites`): additive to extract & host runs; ∀ host-claimed file ∀ `Host::sites` whose guest is compiled in: holes → `holes::marker(i)`, `Host::unescape`, `wrap(body, Guest::prelude(site.env))` → temp file `<tmp>/xnl-lint-<pid>-<n>/site.<Guest::extension>`, removed after; checks = the guest's plan (defaults ∪ trusted config), ⊥ fixers; `file` = host, `dialect` = site's; the temp path in `argv` & `raw_tail` reads `<host>:<line>:<col>` (body start). finding on a prelude line → body start; else unescaped char → raw byte by in-line forward match (a char unmatched on its line stays put), leading raw lines unescape dropped skipped; hole marker → hole start. host parse \| unescape failure → warning `site-unlinted` naming why, ⊥ result.
- `∀ extract → typos` ⊥ engine-run: `src/config:V73` bars engine defaults & `[lint] all` default = `[]`.

## §V INVARIANTS

V8: `lint`: ∀ extract run ∀ check of its guest & `[lint] all`, ∀ host file ∀ `Host::checks` (when `[lint] hosts`) (defaults ∪ | replaced by `[lint.<guest>]`), each reported separately, ⊥ stop at first; binary absent from PATH = exit 2 naming tool & install hint (`nix` package ships ∀ confirmed tool), ⊥ silent skip; tools marked `?` = unconfirmed, optional until confirmed; `--fix` runs fixers then re-checks, touches extracts only.
V91: config-defined commands run only w/ `--trust-config`; untrusted → skipped w/ warning `untrusted-command` naming each, built-in defaults still run.
V92: findings parsed per `LintCmd.format` into `{line, col, code, severity, message}`; unparseable output → `raw_tail` (last 40 lines), ⊥ dropped; `fail` \| `error` → exit 1 \| 2 per `src` §I exit.
V93: `xnl lint --sites` lints in-host sites BEFORE extraction: body materialised to temp file via `wrap`, checks run, finding positions mapped back to host `file:line:col` through `Delim.body` & `unescape`; `kind: site`.
V126: ∀ check & fixer killed at `[lint] timeout` (default 60s) → `status: error`, exit 2, naming tool & limit; ⊥ hang. a gate that hangs is bypassed next commit.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end | T24, T87, T92, T93, T125 | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |
| M3 | publication | T94 | public doc set, release machinery, history audit green, crates published (`.:T32`) |

id|status|task|cites
T24|x|`lint`: per-language linter map w/ defaults (shellcheck+shfmt, ruff ?, sqlfluff ?, eslint ?, stylelint ?), missing binary = exit 2|V8
T87|x|multi-check runner, per-check results, `--fix`; defaults table; fixture w/ 2 checks both failing|V8
T92|x|`--trust-config` gate; fixture: config check skipped w/ warning, runs w/ flag|V91
T93|x|findings parsers (shellcheck json, ruff json, sqlfluff json, SARIF) + raw fallback|V92
T94|x|virtual-extract linting w/ source mapping; fixture: shellcheck finding in nix `script` reported at nix line|V93
T125|x|timeout per check/fixer + fixture: a sleeping tool errors at the limit|V126

## §B BUGS

id|date|cause|fix
