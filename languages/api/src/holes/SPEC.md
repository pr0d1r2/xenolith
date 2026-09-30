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

- `Host::hole_advice(&Site) -> Result<Vec<String>>`: host's strategies for holes (`languages/ci/nix:V54`); root wraps each as `Judgment` direction (V40); default `Unsupported` (`languages/api:V37`).
- `holes::bind(host, guest, src, &Site, &Limits { max_params, prefix }) -> Result<Outcome>`: `Mechanical { body, params: [Param { name, hole, marker }] }` \| `Judgment(Refusal)` (`TooMany` \| `Unexpanded(name)` \| `Unsupported(op)` \| `Marker`): markers `XNL_HOLE_<i>_` → `unescape` → `Guest::vars` (taken) → names → `Guest::params`. ⊥ holes → `unescape` only. `collect`, `names`, `reserved` pub (nix `__NAME__`).
- guest side, default `Unsupported` (`languages/api:V37`) → `Judgment`: `Guest::vars(body)` (names read\|assigned); `Guest::params(body, &[Param]) -> Result<Bound>`: marker → env ref its context needs (shell `"$NAME"`, in `"…"` `${NAME}`), `Unexpanded(name)` iff ⊥ expanding; `Guest::param_refs(body, names) -> Result<Vec<(Span, name)>>`: inverse finder (shell `"$N"` → whole string).

## §V INVARIANTS

V40: holes → named env params: each distinct hole (same bytes + attached `/`-led tail of `[A-Za-z0-9._+/-]`, e.g. `${pkgs.foo}/bin/foo`) → NAME = UPPER_SNAKE (camel split) of last segment + kind suffix = 1st tail dir (`/bin/foo` → `FOO_BIN`, `${cfg.port}` → `PORT`; ⊥ word → `PARAM`, digit-led → `PARAM_`…), collision or clash w/ var used in body \| reserved name (`PATH`, `HOME`, `IFS`, `PWD`, `OLDPWD`, `SHELL`, `USER`, `LOGNAME`, `TERM`, `TMPDIR`, `LANG`, `LC_*`, `BASH*`, `ZSH*`, `SHLVL`, `PS1`–`PS4`, `HOSTNAME`, `UID`, `EUID`, `CI`, `GITHUB_*`, `RUNNER_*`) → `_2`… \| `_PARAM`; `[threshold.load] param_prefix` prepended when set; extract refs via `Guest::params` \| host form, load passes `NAME=<hole>` in host syntax (`Host::rewrite_bound`; holes stay host interpolations). MECHANICAL iff count ≤ `[threshold.load] max_params` & ∀ hole in expanding context (shell: plain word \| `"…"`; ⊥ `'…'`, `$'…'`, heredoc, comment, `$((…))`, `${…}` operand, `[[ ]]`, `case` pattern, var name) & guest side ≠ ⊥ & load one-liner trivial (`languages/shells/shell:V3`); else `Judgment` w/ `hole_advice` & `rewrite` refuses (exit 2). ⊥ copying `${…}` into guest file verbatim.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end |  | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |

id|status|task|cites

## §B BUGS

id|date|cause|fix
