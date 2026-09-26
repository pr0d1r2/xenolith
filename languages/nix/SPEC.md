# SPEC

## §G GOAL

crate `xenolith-lang-nix` (feature `lang-nix`): `rnix` parser; nix sinks per matrix, `builtins.readFile` load idiom.

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
self|languages/nix|nix parser, sinks, load idiom
sib|languages/api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness
sib|languages/pkl|pkl parser, hk step sinks, load idiom
sib|languages/shell|bash parser & host sinks, single-command classifier, shell linters
sib|languages/just|just parser, recipe sinks, load idiom
sib|languages/python|python grammar, guest rules
sib|languages/sql|sql grammar, guest rules
sib|languages/jq|jq grammar, guest rules
sib|languages/awk|awk grammar, guest rules
sib|languages/bats|bats grammar (based-on shell), `@test` sinks, test-host rules
sib|languages/yaml|yaml parser, GH Actions sinks, placement
sib|languages/dockerfile|Dockerfile parser, `RUN` sinks, placement
sib|languages/shebang|shebang parse/strip/wrap ∀ guest
sib|languages/rust|rust parser, rust host sinks
sib|languages/ruby|ruby parser, ruby host sinks
sib|languages/html|html parser, inline script/style sinks
sib|languages/js|javascript grammar, guest rules
sib|languages/css|css grammar, guest rules
sib|languages/perl|perl grammar, guest rules

## §I INTERFACES

- sinks: `script`, `preStart`, `postStart`, `preStop`, `postStop`, `shellHook`, `ExecStart*`, `text` of `writeShellScript*`/`writeShellApplication`, `runCommand` body, `buildPhase`/`installPhase`/`*Phase`, phase hooks `pre`\|`post` + capital ⊥ `*Phases` (T156) → guest shell; value built w/ `+` → ∀ string operand = site (T155); load after extract: per `languages/nix:V53`.

## §V INVARIANTS

V53: nix host placement: name = attr path tail (`systemd.services.foo.script` → `foo-script`), dir = `<host_dir>/<host_stem>/`, load = `nix-shebang.lib.readWithoutStrict ./<host_stem>/<name>.<ext>` when prelude present (consumer flake needs `nix-shebang` input — kept a separate package, ⊥ vendored or re-exported by xenolith; absent → `Judgment` direction to add it), `builtins.readFile` when prelude empty.
V54: nix `hole_advice` (fallback when `languages/api/src/holes:V40` params ⊥ apply): `${…}` holes → propose `replaceVars ./<file> { var = …; }` w/ `@var@` placeholders, else pass as argv | env. advice only, ⊥ auto-applied.
V69: `ExecStart*` = systemd exec line, ⊥ shell grammar: trivial per `[threshold.exec]`; over → extract to shell script w/ simple interface (fixed argv, holes → params per `languages/api/src/holes:V40`, `"$@"` forwarded) & `ExecStart = "${nix-shebang.lib.toShellScript { inherit pkgs; name = "<name>"; src = ./…; }}/bin/<name>"` ∴ logic unit-testable (bats).

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end | T12, T55, T71, T146, T155-T159 | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |

id|status|task|cites
T12|x|host nix (`rnix`): sinks per matrix, fixtures pos+neg|`languages:V2`,`tests:V14`,`tests:V15`
T55|.|nix `Host::placement` & `hole_advice` + fixtures (attr-path names, `${…}` → `replaceVars` advice)|V53,V54
T71|.|`ExecStart*` classifier & extraction; fixtures: short line inline, long line → script|V69
T146|x|`src:C139` backfill: `languages/nix/src/tests.rs` (`lib.rs`), `languages/nix/src/sinks/tests.rs`|`src:C139`,`scripts/guard:V140`
T155|x|sink value built w/ `+` (`''…'' + extra`): ∀ string operand of `+` in sink position = site; fixture: shellHook concat w/ `\|\|` flagged|B1,`languages:V2`,`tests:V118`
T156|x|phase hooks `pre*`/`post*` (`preCheck`, `postInstall`, `preBuild` …) = sinks → shell; fixture: 3-command `preCheck` flagged|B1,`tests:V118`
T157|.|shebang-led string (`#!` first line) in any attr (e.g. `environment.etc.<f>.text`) = site, guest by shebang (`languages/shebang`); fixture: `#!/bin/sh` xinitrc flagged|B1,`tests:V118`
T158|.|indented-string dedent & `''` escapes before guest sees body (`languages/api/src/lens:V39`); fixture: heredoc in `writeShellScript` classified, ⊥ parse error|B1,`tests:V118`
T159|.|home-manager shell init attrs (`initContent`, `initExtra`, `profileExtra`, `bashrcExtra` …) → shell, dialect per program (zsh/bash); fixture|B1,`tests:V118`

## §B BUGS

id|date|cause|fix
B1|2026-09-26|fleet pilot (`tests:T154`, 16 consumer repos vs the legacy line-scanning hook): 4 embed shapes the legacy hook flags, `xnl` misses — `+`-concatenated sink value, `pre*`/`post*` phase hooks, shebang-led `text`, shell init attrs; + nix `''` body not dedented ∴ indented heredoc terminator = parse error ∴ reason wrong (still flagged). ∴ swap ⊥ drop-in yet|T155-T159
