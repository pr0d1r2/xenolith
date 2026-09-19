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

## §V INVARIANTS

V53: nix host placement: name = attr path tail (`systemd.services.foo.script` → `foo-script`), dir = `<host_dir>/<host_stem>/`, load = `builtins.readFile ./<host_stem>/<name>.<ext>`.

## §T TASKS

id|status|task|cites
T12|.|host nix (`rnix`): sinks per matrix, fixtures pos+neg|`languages:V2`,`tests:V14`,`tests:V15`

## §B BUGS

id|date|cause|fix
