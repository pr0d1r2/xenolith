# SPEC

## §G GOAL

crate `xenolith-lang-nix` (feature `lang-nix`): `rnix` parser; nix sinks per matrix, load idiom.

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

- sinks: `script`, `preStart`, `postStart`, `preStop`, `postStop`, `shellHook`, `ExecStart*`, `text` of `writeShellScript*`/`writeShellApplication`, `runCommand` body, `buildPhase`/`installPhase`/`*Phase`, phase hooks `pre`\|`post` + capital ⊥ `*Phases` → guest shell; `programs.<zsh|bash>` init options (home-manager `initContent`/`initExtra*`/`envExtra`/`bashrcExtra`/`profileExtra`/`loginExtra`/`logoutExtra`, NixOS `shellInit`/`loginShellInit`/`interactiveShellInit`/`promptInit`; closed list per program) → shell, dialect = program, ⊥ options; value built w/ `+` → ∀ string operand = site; whole attr value w/ shebang 1st line (after dedent) → guest by `shebang::guest_of`, dialect = interpreter ∈ sh\|bash\|zsh, unknown → ⊥ site; `mk*` order/priority wrap transparent; load: V53.

## §V INVARIANTS

V53: nix host placement: name = last ≤2 attr segments, builders ⊥, kebab (`systemd.services.foo.script` → `foo-script`), dir = `<host_dir>/<host_stem>/`, load = `nix-shebang.lib.readWithoutStrict ./<host_stem>/<name>.<ext>` iff `nix-shebang` in scope (V170; ⊥ vendored), else `builtins.readFile`; prelude left in string harmless (shebang = comment, `set -e`). `loads` = either call, anywhere, on relative path literal `.sh`\|`.bash`\|`.zsh`.
V54: `hole_advice` (V174 ⊥): `replaceVars` w/ `__var__`, argv, env; ⊥ applied.
V69: `ExecStart*` = systemd exec line, ⊥ shell grammar: trivial per `[threshold.exec]`; over → extract to shell script w/ simple interface (fixed argv, holes → params per `languages/api/src/holes:V40`, `"$@"` forwarded) & `ExecStart = "${nix-shebang.lib.toShellScript { inherit pkgs; name = "<name>"; src = ./…; }}/bin/<name>"` ∴ logic unit-testable (bats).
V170: write side: `rewrite` → V53 load; in scope = PROVEN lexical binding: lambda formal\|arg\|`@`, `let`, `rec`, `with` of attrset literal w/ it; `<x>.nix-shebang.lib…` used under site's binder of `<x>` → that prefix. `loads(y)` ∌ path → refuse. path `./`-prefixed, `(…)` unless slot = attr value\|binop\|paren\|body; `Unsupported` ∀ holes (V174), `ExecStart*` (V69), guest ⊥ shell, `readWithoutStrict` w/ body line 1 `set -euo pipefail` (stripStrict). `inline` either load: body ∋ `\n` → `''…''` else `"…"`, via `escape`; drops parens `rewrite` added.
V174: `rewrite_bound` w/ `languages/api/src/holes:V40` params: body `__NAME__`, load `builtins.replaceStrings [ "__NAME__" … ] [ "<hole>" … ] (<V53 load>)` (⊥ IFD); ⊥ exact → refuse; `inline` inverse.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end | T12, T55, T71, T146, T155-T161, T170, T174 | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |

id|status|task|cites
T12|x|host nix (`rnix`): sinks per matrix, fixtures pos+neg|`languages:V2`,`tests:V14`,`tests:V15`
T55|x|nix `Host::placement` & `hole_advice` + fixtures|V53,V54
T71|.|`ExecStart*` classifier & extraction; fixtures: short line inline, long line → script|V69
T146|x|`src:C139` backfill: `languages/nix/src/tests.rs` (`lib.rs`), `languages/nix/src/sinks/tests.rs`|`src:C139`,`scripts/guard:V140`
T155|x|`+`-built sink value: ∀ string operand = site; fixture|B1,`languages:V2`,`tests:V118`
T156|x|phase hooks `pre*`/`post*` = shell sinks; fixture|B1,`tests:V118`
T157|x|shebang-led string in any attr = site, guest by shebang; fixture|B1,`tests:V118`
T158|x|`''` dedent & escapes before guest sees body (`languages/api/src/lens:V39`); fixture|B1,`tests:V118`
T159|x|shell init attrs → shell, dialect per program; fixture|B1,`tests:V118`
T160|x|T157 ∀ whole text arg of `writeScript`/`writeText`; fixture|B1,`tests:V118`
T161|x|sink value wrapped once in `mk*` order/priority = its site; fixture|B1,`tests:V118`
T170|x|`rewrite`/`inline`/`escape`; laws (a)-(c) & `escape_law` ∀ fixture site|V170,V53,`languages/api/src/lens:V34`
T174|x|holes via `replaceStrings`; laws ∀ hole fixture|V174

## §B BUGS

id|date|cause|fix
B1|2026-09-26|fleet pilot (`tests:T154`, 16 consumer repos vs the legacy line-scanning hook): 4 embed shapes the legacy hook flags, `xnl` misses — `+`-concatenated sink value, `pre*`/`post*` phase hooks, shebang-led `text`, shell init attrs; + nix `''` body not dedented ∴ indented heredoc terminator = parse error ∴ reason wrong (still flagged).|T155-T159
