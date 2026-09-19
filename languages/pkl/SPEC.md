# SPEC

## §G GOAL

crate `xenolith-lang-pkl` (feature `lang-pkl`): pkl grammar; hk step sinks, `bash scripts/hk/x.sh {{files}}` load idiom.

## §I INTERFACES

- sinks: hk step `check`, `fix`, `shell`, `check_diff`, `check_list_files` → guest shell; load after extract: `bash scripts/hk/x.sh {{files}}`.

## §V INVARIANTS

V52: pkl host placement: hk step site → name = step key, dir = `scripts/hk`, load = `bash scripts/hk/<name>.sh {{files}}` (hk passes files through).

## §T TASKS

id|status|task|cites
T13|.|host pkl (tree-sitter-pkl ?): hk step sinks, fixtures|`languages:V2`,`languages/shell:V3`,`tests:V14`,`tests:V15`
T54|.|pkl `Host::placement` for hk steps + fixture (`{{files}}` forwarded)|V52

## §B BUGS

id|date|cause|fix
