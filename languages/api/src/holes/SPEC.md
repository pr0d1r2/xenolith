# SPEC

## §G GOAL

HOLES as params: named env params, their references & inverse, host hole advice.

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
up|languages/api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness
up|languages/api/src|api modules: site, lens, holes; hub root `lib.rs` & `shebang.rs` re-export
self|languages/api/src/holes|param naming, param refs, hole advice
sib|languages/api/src/site|Site/Delim/GuestEnv types, placement, claims, candidates
sib|languages/api/src/lens|rewrite/inline, escape, runtime base, laws harness

## §I INTERFACES

- `Host::hole_advice(&Site) -> Vec<String>`: host's proposed strategies for holes (nix `replaceVars`, pass as arg, env var); root wraps each as `Judgment` direction (V40).
- `Guest::param(name) -> Option<String>`: guest's reference to named env param (shell `"$FOO_BIN"`, python `os.environ["FOO_BIN"]`); `Guest::param_refs(body, names) -> Vec<(Span, name)>`: parse-based finder for inverse; ⊥ → holes of that guest stay `Judgment`.

## §V INVARIANTS

V40: holes → named env params: each distinct hole (+ attached path tail up to whitespace \| quote, e.g. `${pkgs.foo}/bin/foo`) → NAME = UPPER_SNAKE of last segment + kind suffix (`/bin/foo` → `FOO_BIN`, `${cfg.port}` → `PORT`), collision or clash w/ var used in body \| reserved name (`PATH`, `HOME`, `IFS`, `PWD`, `OLDPWD`, `SHELL`, `USER`, `LOGNAME`, `TERM`, `TMPDIR`, `LANG`, `LC_*`, `BASH*`, `ZSH*`, `SHLVL`, `PS1`–`PS4`, `HOSTNAME`, `UID`, `EUID`, `CI`, `GITHUB_*`, `RUNNER_*`) → `_2`… \| `_PARAM`; `[threshold.load] param_prefix` prepended when set; extract uses `Guest::param(NAME)`; load = one-liner `NAME=<hole> … <invoke>` in host syntax (holes stay host interpolations). MECHANICAL iff count ≤ `[threshold.load] max_params` & ∀ hole in expanding context (⊥ single-quoted, ⊥ quoted heredoc, ⊥ inside guest string literal) & `param` ≠ ⊥ & load one-liner trivial (`languages/shell:V3`); else `Judgment` w/ `hole_advice` & `rewrite` refuses (exit 2). ⊥ copying `${…}` into guest file verbatim.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end | T76 | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |

id|status|task|cites
T76|.|holes → params rewrite; fixtures: `${pkgs.foo}` ×2 → one param, hole in single quotes → `Judgment`, 7 holes → `Judgment`|V40

## §B BUGS

id|date|cause|fix
