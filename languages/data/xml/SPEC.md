# SPEC

## §G GOAL

crate `xenolith-lang-xml` (feature `lang-xml`): tree-sitter-xml; host: text plists & `*.xml` -- launchd `ProgramArguments` shell argv; host check `xmllint --noout`.

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
up|languages/data|hub: data, text & markup -- python, sql, jq, awk, perl, xml
self|languages/data/xml|xml parser, launchd argv sinks, `xmllint` check
sib|languages/data/python|python grammar, guest rules
sib|languages/data/sql|sql grammar, guest rules
sib|languages/data/jq|jq grammar, guest rules
sib|languages/data/awk|awk grammar, guest rules
sib|languages/data/perl|perl grammar, guest rules

## §I INTERFACES

- sinks: launchd plist `ProgramArguments` `<array>` of `<string>` (V189) → guest shell; CDATA \| element text known to hold a script ? (T191) -- ⊥ named until a fixture shows one.
- body as the guest reads it = `<string>` text w/ entities decoded (`&lt;` `&gt;` `&amp;` `&quot;` `&apos;` `&#N;` `&#xN;`), CDATA verbatim (`languages/api/src/lens:V39`), literal CR/CRLF → LF; escape: `&` `<` `>` → entities, CR → `&#13;`; other entity ⊥ decoded → `unparseable`.
- load after extract ? (T190): launchd runs jobs w/ cwd `/` unless `WorkingDirectory` ∴ ⊥ repo-relative path is right by default → extract direction `Judgment` until decided.
- host checks: `xmllint --noout` (libxml2) ∀ claimed file -- the fleet hook's command (R186); `plutil -lint` ⊥ (macOS only, `.:C3`).

## §R RESEARCH

id|topic|finding|src
R186|fleet 2026-09-27|15 repos / 42 files (33 `.xml`, 9 `.plist`); 3 launchd plists w/ `ProgramArguments` (argv arrays, may be `sh -c <script>`); 3 files w/ CDATA. fleet hook = `xmllint --noout` per staged `*.xml` (`.plist` ⊥ checked)|read-only fleet survey, counts only (`scripts/guard` C17)
R187|grammar 2026-09-27|`tree-sitter-xml` on crates.io 0.7.0 (tree-sitter-grammars, MIT) ∴ crate dep, ⊥ vendored (`languages:V121`)|crates.io API

## §V INVARIANTS

V188: `claims`: `*.xml`; `*.plist` iff text XML (starts `<?xml` \| `<!DOCTYPE plist` \| `<plist`); binary plist (`bplist00`) ⊥ claimed (⊥ parse, `src/check:V13`). other XML exts (`.svg`, `.xsd`, `.xsl`, `.xhtml`, ...) ⊥ claimed ? until a sink needs them.
V189: launchd sink: top `<plist><dict>` w/ `<key>ProgramArguments</key>` → `<array>`; ⊥ `<key>Program</key>` (else argv[0] ≠ executable ?); argv[0] basename ∈ {`sh`,`bash`,`zsh`,`dash`} & argv[1] = `-c` exactly → argv[2] = site, dialect = argv[0], env from ITS argv (argv form of `languages/shells/shell:V139`); combined flags (`-lc`, `-ec`) ?. ⊥ other shape = site.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M3 | publication -- just, xml, tcl | T190-T191 | host claims its files, flags launchd `sh -c` argv w/ fixtures, runs `xmllint` (`languages:V56`) |

id|status|task|cites
T190|.|DECIDE by 2026-10-15: xml load after extract -- `WorkingDirectory`-relative, absolute install path, or stay `Judgment`; fixture per shape|V189,`languages:V74`,`languages/api:V35`
T191|.|DECIDE by 2026-10-15: CDATA sinks -- measure the fleet's 3 CDATA files' element names (counts & shapes only); name an element here only w/ that evidence, else drop|R186,`languages:V81`

## §B BUGS

id|date|cause|fix
