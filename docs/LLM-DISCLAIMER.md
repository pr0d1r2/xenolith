# Written by an LLM, reviewed by a human

Nearly all of this repository — Rust, shell, Nix, fixtures, specs and this
prose — was written by [Claude Code](https://claude.com/claude-code), running
Anthropic's Claude Opus models. Commits a model wrote carry a
`Co-Authored-By: Claude …` trailer naming the model; at the time of writing
that is every commit but the first. A human owns the project, decides what it does, reviews the changes and
answers for what ships.

That is the disclosure. The rest of this page is about what it means for
you as a reader, and what you can check instead of taking it on trust.

## Why it matters here

A model produces plausible code, and plausible is not the same as correct.
Knowing how code was made tells you how hard to look at it.

For this tool the bar is higher than usual. xenolith rewrites other people's
files: it moves a script out of a Nix attribute, a justfile recipe or an hk
step into its own file and replaces it with a line that runs that file. An
extraction that quietly changes what runs — drops an `errexit`, reorders two
commands, loses a line's `-` prefix — is worse than no tool. So the project
is built to make its claims checkable rather than asserted.

## How the work is constrained

- **The spec is the law.** [`SPEC.md`](../SPEC.md) and the `SPEC.md` of every
  node hold the invariants that must stay true, the tasks still open, and a
  bug record. A change to behaviour starts as a change to a spec row, and
  the commit cites that row.
- **Tests come first.** A failing test is committed before the code that
  makes it pass, and a guard checks the order in history.
- **Every commit says why.** The commit-message hook refuses a body without
  a `Why:` line, so the reasoning behind a change is in the log next to the
  change.
- **The tool is run on itself.** `xnl check` over this repository is a gate
  step: the project may not hold the kind of embed it exists to remove.
- **One gate, everywhere.** [`hk.pkl`](../hk.pkl) defines the checks once; the
  git hooks and CI run the same definition.

[`CONTRIBUTING.md`](CONTRIBUTING.md) describes that loop in full.

## What to check yourself

- **Run the gate.** `nix develop`, then `hk check --all`. If a claim on this
  page is false, that is where it shows.
- **Read the bug records.** Each `§B` row pairs a defect with its cause and
  the rule that now catches it. A record with nothing embarrassing in it
  would be the suspicious one.
- **Try it on your own files.** `xnl check` only reports; `xnl extract`
  without `--write` only prints a diff. Nothing is changed until you ask.
- **Look for rules with no runner.** An invariant that no test, guard or
  gate step executes is exactly the kind of gap this process is meant to
  rule out. Finding one is a useful bug report.

## Responsibility

The maintainer named in [`LICENSE`](../LICENSE) is responsible for this code,
including the parts a model wrote and the mistakes nobody caught. "A model
wrote it" explains where the code came from; it does not move
responsibility anywhere else.

Bug reports are welcome, and unflattering ones are the most useful: see
[`CONTRIBUTING.md`](CONTRIBUTING.md), or [`SECURITY.md`](SECURITY.md) for
anything that should not be public.
