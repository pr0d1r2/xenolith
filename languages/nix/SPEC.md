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

## §I INTERFACES

- sinks (host|sink detected|embedded|load idiom after extract): `nix` · `script`, `preStart`, `postStart`, `shellHook`, `ExecStart*`, `text` of `writeShellScript*`/`writeShellApplication`, `runCommand` body, `buildPhase`/`installPhase`/`*Phase` · shell · per `languages/nix:V53`

## §V INVARIANTS

V53: nix host placement: name = attr path tail (`systemd.services.foo.script` → `foo-script`), dir = `<host_dir>/<host_stem>/`, load = `nix-shebang.lib.readWithoutStrict ./<host_stem>/<name>.<ext>` when prelude present (consumer flake needs `nix-shebang` input; absent → `Judgment` direction to add it), `builtins.readFile` when prelude empty.
V54: nix `hole_advice` (fallback when `languages/api:V40` params ⊥ apply): `${…}` holes → propose `replaceVars ./<file> { var = …; }` w/ `@var@` placeholders, else pass as argv | env. advice only, ⊥ auto-applied.
V69: `ExecStart*` = systemd exec line, ⊥ shell grammar: trivial per `[threshold.exec]`; over → extract to shell script w/ simple interface (fixed argv, holes → params per `languages/api:V40`, `"$@"` forwarded) & `ExecStart = "${nix-shebang.lib.toShellScript { inherit pkgs; name = "<name>"; src = ./…; }}/bin/<name>"` ∴ logic unit-testable (bats).

## §T TASKS

id|status|task|cites
T12|.|host nix (`rnix`): sinks per matrix, fixtures pos+neg|`languages:V2`,`tests:V14`,`tests:V15`
T55|.|nix `Host::placement` & `hole_advice` + fixtures (attr-path names, `${…}` → `replaceVars` advice)|V53,V54
T71|.|`ExecStart*` classifier & extraction; fixtures: short line inline, long line → script|V69

## §B BUGS

id|date|cause|fix
