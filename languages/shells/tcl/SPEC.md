# SPEC

## §G GOAL

crate `xenolith-lang-tcl` (feature `lang-tcl`): vendored tree-sitter-tcl; tcl (& expect dialect) as host (`exec`/`spawn` argv, `exec … <<`) & as guest (shell heredoc to `tclsh`, `expect -c`).

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
up|languages/shells|hub: shell family -- shell, bats, tcl
self|languages/shells/tcl|tcl grammar (expect dialect), `exec`/`spawn` sinks, guest rules
sib|languages/shells/shell|bash parser & host sinks, single-command classifier, shell linters
sib|languages/shells/bats|bats grammar (based-on shell), `@test` sinks, test-host rules

## §I INTERFACES

- host sinks (V196): `exec sh|bash|zsh|dash -c <word>` → shell; `exec <interp> << <word>` (Tcl's stdin-from-value redirection) → guest per interp; expect `spawn sh|bash -c <word>` → shell. `spawn` + `send` (interactive) ⊥ site; `open "\|sh -c …"` ?.
- body: braced `{…}` word verbatim; `"…"` word w/ `$var` \| `[cmd]` → holes (`languages/api/src/holes:V40`) ?, until fixture.
- guest: per V197; shell side = `languages/shells/shell:V139`, added by T200 ⊥ here.
- checks & fixers: V198; `LintCmd` = `[xenolith-tcl-syntax]`, file appended, `Format::Raw`. `loads`/`rewrite`/`inline` = `Unsupported` (⊥ load idiom here).
- walk: script = top level, `[cmd]`, builtin bodies (`proc` `if` `while` `foreach` `catch` `try`), `namespace eval`; braced ARG of any other cmd = opaque data, ⊥ searched. grammar parses ∀ braces as script ∴ ERROR inside opaque braces ⊥ fails file, else `languages:V78`. vendored grammar carries a local patch (B1).
- shebang claim read in-crate: `api` shebang table ⊥ tcl (widening it moves other hosts' guests).

## §R RESEARCH

id|topic|finding|src
R193|fleet 2026-09-27|5 repos / 23 files (14 `.tcl`, 9 `.exp`); 13 use `exec`, 5 run `sh\|bash -c`, 8 use expect `spawn`. fleet hook (`*.tcl`, `*.exp`): `tclsh` reads the file, `info complete` (unclosed braces, brackets, quotes) + lint "`#` inside `set x { }` = literal, ⊥ comment" — itself a Tcl program fed to `tclsh` by a shell heredoc, i.e. a V197 site|read-only fleet survey, counts only (`scripts/guard` C17)
R194|grammar 2026-09-27|`tree-sitter-tcl` (tree-sitter-grammars, MIT) ⊥ on crates.io (404) ∴ VENDOR per `languages:V121`|crates.io API, upstream repo
R208|Rust tcl checker 2026-09-27|crates.io: `molt` 0.3.1 (BSD-3, last release 2020-05-11): `Interp::complete` = its parser, tclsh's messages but ⊥ position, pulls `indexmap` 1; `molt-ng` 0.3.2 (2022 fork, same); `tcl` 0.1.9 = bindings to C libtcl (external tool by another name); `rtcl` `tcl-parser` 404 ∴ own word parser, ⊥ dep. differential fuzz vs `tclsh` 8.5.9 (random texts over `{}[]"$\;#()` & friends): `info complete` verdict 0 mismatches / 140k; extra-chars vs eval in a command-less interp 0 mismatches / 50k `$`-free texts, but 3 where `{*}` hit a runtime list error first (static check still right)|crates.io API, molt 0.3.1 source, fuzz in scratch

## §V INVARIANTS

V195: `claims`: `*.tcl`, `*.tk`, `*.exp`, shebang resolving to `tclsh` \| `wish` \| `expect`. expect = DIALECT of tcl (same grammar, extra commands `spawn` `expect` `send` `interact`; `languages/shells:V132`) ∴ ⊥ own `LangId`, `env.dialect = expect` iff `.exp` \| expect shebang.
V196: host site ⇐ plain interp name as `exec`/`spawn` arg 1 (⊥ `$var`, ⊥ `[cmd]`); `-c` word → shell, dialect = interp, env from ITS argv (argv form of `languages/shells/shell:V139`); `<<` word → guest per interp; ⊥ other shape = site.
V197: guest tcl ⇐ shell heredoc to `tclsh` \| `wish` \| `expect` & `expect -c '…'` (`tclsh` has ⊥ `-c`). defaults: `invoke` = `tclsh {path}` \| `expect {path}`; ext `tcl` \| `exp`; prelude shebang `#!/usr/bin/env tclsh` \| `expect`, strict ⊥; `trivial` ? (default `max_lines`/`max_bytes`, `languages/api:V37`).
V198: checks (host & guest, both dialects) = bin `xenolith-tcl-syntax` of this crate: Tcl WORD syntax in Rust (R208), ⊥ external tool. ∀ command of the top-level script & of each `[…]` in it (Tcl parses both before running): unclosed `{` `"` `[` `${` `$a(` \| text ending in `\`-newline (≡ `info complete`, R193) \| extra chars after close-brace \| close-quote → `file:line:col: <tclsh's words>`, 1st error only (Tcl stops there), exit 1; clean 0; usage \| unreadable 2. braced word = matched, ⊥ entered (data, or script parsed later); ⊥ semantics. fixers ⊥.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M3 | publication -- just, xml, tcl |  | host & guest tcl w/ fixtures, checks decided (`languages:V56`) |

id|status|task|cites

## §B BUGS

id|date|cause|fix
B1|2026-09-27|vendored grammar (upstream `main` \| `fixes` \| `update` alike) rejects valid `"$ "` (lone `$`), `a; b` (scanner concats `;`), `expr {1+2}` (`number` eats `+2`, fn name eats `1+2`), `set x $y(z)` (scanner concats `(`), `"a]"`, `if … then`, `catch` options var, `try … on ok \| trap`, `max(1,2)` → `host-parse-error`|`vendor/tree-sitter-tcl/xenolith.patch` (grammar.js & scanner.c), parser.c regenerated w/ upstream's tree-sitter-cli 0.25.3 (reproduces upstream byte for byte unpatched), upstream corpus 24/24, recorded in `UPSTREAM` (`languages:V121`); fixture `pos-grammar-gaps`, grammar tests. open: `set x a(b)`, `regexp {a(b)} …`, `[f]($x)`, `expr {0x1F}`
