# SPEC

## §G GOAL

the SITE a host finds: `Site`, `Delim`, `DelimKind`, `GuestEnv`, holes as spans; placement, claims, guest candidates.

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
up|languages/api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness
up|languages/api/src|api modules: site, lens, holes; hub root `lib.rs` & `shebang.rs` re-export
self|languages/api/src/site|Site/Delim/GuestEnv types, placement, claims, candidates
sib|languages/api/src/lens|rewrite/inline, escape, runtime base, laws harness
sib|languages/api/src/holes|param naming, param refs, hole advice

## §I INTERFACES

- type `Site { sink, guest: LangId, env: GuestEnv, delim: Delim, holes: Vec<Span> }`. `Delim { kind, open: Span, body: Span, close: Span }` = the braces bounding foreign code, taken from AST string|block node, ⊥ scanned. `DelimKind` ∈ nix `''…''` \| `"…"`, heredoc `<<[-~]TAG` (quoted \| not), pkl `"""…"""` / `#"""…"""#`, yaml block `\|`/`>` + chomp, html `<script>`/`<style>` element, rust `r#"…"#`, ruby `<<~TAG`, argv string (`-c`/`-e`), just recipe body (indent block) & shebang recipe. `holes` = host interpolations inside body (nix `${…}`, pkl `\(…)`, shell `$VAR` in unquoted heredoc, yaml `${{ }}`, just `{{…}}`).
- `DelimKind::runs_line_by_line() -> bool`: host runs each body line as its own program (just recipe, `languages/ci/just:V180`) ∴ engine may judge line by line (`src/config:V240`); ∀ other kind `false`.
- `Host::placement(&Site) -> Result<Placement { name, dir }>`: host's default extract name (from site syntax: nix attr path, hk step name, GH job/step id) & dir (pkl hk step → `scripts/hk`); layer D of extract resolution, lowest precedence. both = `src/extract:V46` templates, engine renders (`Site` ∌ host path: nix dir `{host_dir}/{host_stem}`); default `Unsupported` (`languages/api:V37`).
- `Host::claims(path, head: &str) -> bool`: host claims file by filename, extension, path glob or shebang in `head` (first line); ∀ file is a candidate host.
- `Host::candidates(&Site) -> Vec<LangId>`: ordered guest set the sink context permits; `Guest::rejects(body, &GuestEnv) -> bool`: cheap structural veto (⊥ full parse — `languages:V77`).
- type `GuestEnv { dialect: Option<String>, options: Vec<String> }`: interpreter dialect & effective options at site, derived by host from context (shell: `sh`\|`bash`\|`zsh`, `errexit`, `nounset`, `pipefail`, …).

## §V INVARIANTS

V38: site = delimiter ∧ sink context. delimiter alone (same `''…''` under nix `description`) = inert data ⊥ site (`languages:V2` negative fixture). delimiter bounds from grammar node ∴ escapes, nesting, heredoc terminators, indent rules resolved by parser, ⊥ brace counting over raw bytes.
V43: `placement` name deterministic & semantic: derived from site syntax (attr path, step name, job id), kebab-case, ⊥ line numbers, ⊥ random | hash-only names; no semantic name → `<host_stem>-<sink>`.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end |  | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |

id|status|task|cites

## §B BUGS

id|date|cause|fix
