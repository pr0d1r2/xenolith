# SPEC

## §G GOAL

crate `xenolith-lang-pkl` (feature `lang-pkl`): pkl grammar; hk step sinks, `bash scripts/hk/x.sh {{files}}` load idiom.

## §V INVARIANTS

V52: pkl host placement: hk step site → name = step key, dir = `scripts/hk`, load = `bash scripts/hk/<name>.sh {{files}}` (hk passes files through).

## §T TASKS

id|status|task|cites
T13|.|host pkl (tree-sitter-pkl ?): hk step sinks, fixtures|`languages:V2`,`languages/shell:V3`,`tests:V14`,`tests:V15`

## §B BUGS

id|date|cause|fix
