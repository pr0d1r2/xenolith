# SPEC

## §G GOAL

crate `xenolith-lang-api`: contract ∀ language crate — `Host` & `Guest` traits, shared types, lens law harness. language plays host (file holding embed), guest (embedded code), or both; extract & inline = one lens, 2 directions.

## §F FEDERATION

dir|owns|⊥owns|tokens
src|api modules: site, lens, holes|traits & LangId (this node)|-

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
self|languages/api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness
sib|languages/nix|nix parser, sinks, load idiom
sib|languages/pkl|pkl parser, hk step sinks, load idiom
sib|languages/shell|bash parser & host sinks, single-command classifier, shell linters
sib|languages/just|just parser, recipe sinks, load idiom
sib|languages/python|python grammar, guest rules
sib|languages/sql|sql grammar, guest rules
sib|languages/jq|jq grammar, guest rules
sib|languages/awk|awk grammar, guest rules
sib|languages/yaml|yaml parser, GH Actions sinks, placement
sib|languages/dockerfile|Dockerfile parser, `RUN` sinks, placement
sib|languages/shebang|shebang parse/strip/wrap ∀ guest
sib|languages/rust|rust parser, rust host sinks
sib|languages/ruby|ruby parser, ruby host sinks
sib|languages/html|html parser, inline script/style sinks

## §I INTERFACES

- trait `Host`: `id() -> LangId`; `sites(src: &str) -> Result<Vec<Site>>` (sinks holding guest code); `loads(src) -> Result<Vec<LoadRef>>` (for `src/graph`); `rewrite(src, &Site, &Invoke, path) -> Result<String>` (extract direction: body out, load in); `inline(src, &LoadRef, body) -> Result<String>` (inverse direction).
- trait `Guest`: `id() -> LangId`; `extension() -> &'static str`; `invoke(path) -> Invoke` (how to run a file of me: `bash x.sh`, `jq -f x.jq`); `trivial(body) -> Result<bool>` (may stay inline — shell = single simple command, `languages/shell:V3`); `checks(&GuestEnv) -> Vec<LintCmd>`, `fixers(&GuestEnv) -> Vec<LintCmd>`.
- crate impl: host-only | guest-only | both. guest-only language (python, sql, jq, …) ⊥ needs host grammar.
- type `LangId`: closed enum ∀ language in root host × sink matrix + guest-only (python, sql, js, css, perl, awk, jq, ruby); ⊥ feature-gated.
- type `LoadRef { span, path, guest: LangId }`, `Invoke { argv }`, `LintCmd { argv, file_arg, format: Json(parser) | Sarif | Raw }`, `Error`.
- `Guest::prelude(&GuestEnv) -> Prelude` & `Guest::executable() -> bool`: default content & mode of extract file, overridable by config. type `Prelude { shebang: Option<Shebang>, strict: Option<&'static str> }` (strict: bash `set -euo pipefail`; ⊥ for python/sql/jq/awk) — one value per guest, consumed by mod `shebang`.
- `Host::hole_advice(&Site) -> Vec<String>`: host's proposed strategies for holes (nix `replaceVars`, pass as arg, env var); root wraps each as `Judgment` direction (V40).
- `Guest::param(name) -> Option<String>`: guest's reference to named env param (shell `"$FOO_BIN"`, python `os.environ["FOO_BIN"]`); `Guest::param_refs(body, names) -> Vec<(Span, name)>`: parse-based finder for inverse; ⊥ → holes of that guest stay `Judgment`.
- `Host::checks() -> Vec<LintCmd>` & `Host::fixers()`: checks for host files themselves (nix `statix`, `deadnix`, `nixfmt --check`; GH `actionlint`, `zizmor`; Dockerfile `hadolint`; just `just --fmt --check --unstable`; pkl `pkl format --diff` ?).

## §V INVARIANTS

V32: dependency shape: `xenolith-lang-api` deps ⊆ {`xenolith-shebang`}, ⊥ grammar dep, ⊥ feature; language crate deps ⊆ {`xenolith-lang-api`, own grammar, std-ish}; ⊥ root crate, ⊥ other language crate. checked from `cargo metadata`, ⊥ by review.
V33: `LangId` closed & ungated: ∃ variant ∀ known language regardless of enabled features ∴ host names guest compiled out; adding language = add variant here first.
V35: load idiom split: guest owns `invoke` (how to run file of me); host owns wrapping `Invoke` in own syntax (`builtins.readFile`, hk step, `run:`). ⊥ host hardcodes guest command; ⊥ guest knows host syntax.
V36: trait fns pure: ⊥ fs, ⊥ process, ⊥ env, ⊥ clock; `&str` in, values out ∴ engines own IO (`--write`, lint runs) & C3 determinism holds per crate. `Vec` outputs sorted by span.
V37: missing capability = missing impl, ⊥ default method returning empty. ⊥ silent skip (`src/lint:V8` spirit).
V40: holes → named env params: each distinct hole (+ attached path tail up to whitespace \| quote, e.g. `${pkgs.foo}/bin/foo`) → NAME = UPPER_SNAKE of last segment + kind suffix (`/bin/foo` → `FOO_BIN`, `${cfg.port}` → `PORT`), collision or clash w/ var used in body \| reserved name (`PATH`, `HOME`, `IFS`, `PWD`, `OLDPWD`, `SHELL`, `USER`, `LOGNAME`, `TERM`, `TMPDIR`, `LANG`, `LC_*`, `BASH*`, `ZSH*`, `SHLVL`, `PS1`–`PS4`, `HOSTNAME`, `UID`, `EUID`, `CI`, `GITHUB_*`, `RUNNER_*`) → `_2`… \| `_PARAM`; `[threshold.load] param_prefix` prepended when set; extract uses `Guest::param(NAME)`; load = one-liner `NAME=<hole> … <invoke>` in host syntax (holes stay host interpolations). MECHANICAL iff count ≤ `[threshold.load] max_params` & ∀ hole in expanding context (⊥ single-quoted, ⊥ quoted heredoc, ⊥ inside guest string literal) & `param` ≠ ⊥ & load one-liner trivial (`languages/shell:V3`); else `Judgment` w/ `hole_advice` & `rewrite` refuses (exit 2). ⊥ copying `${…}` into guest file verbatim.

## §T TASKS

id|status|task|cites
T42|.|scaffold `languages/api` crate: `LangId`, `Site`, `LoadRef`, `Invoke`, `LintCmd`, `Error`, `Host`, `Guest`; workspace member, ⊥ features|V33,V36,V37,C24
T44|.|port per-language tasks onto traits: shell `Host`+`Guest` (`languages/shell:T11`, `languages/shell:T15`), nix `Host` (`languages/nix:T12`), pkl `Host` (`languages/pkl:T13`); each crate runs `laws::check`|`languages/api/src/lens:V34`,V35
T76|.|holes → params rewrite; fixtures: `${pkgs.foo}` ×2 → one param, hole in single quotes → `Judgment`, 7 holes → `Judgment`|V40

## §B BUGS

id|date|cause|fix
