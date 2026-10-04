# Built by an LLM, deliberately and in the open

<!-- hallucinogen:tending-disclaimer start -->
**Tended by an autonomous loop running Codex with GPT-5.6 luna at low
reasoning effort.** When Codex is unavailable the loop falls back to Claude,
then to a local model. The loop opens pull requests, reviews them itself
and merges them once they are green, without a human reading the diff: the
merge gate is this repository's own checks plus that automated review, not
human approval. On top of that, the maintainer runs periodic meta-reviews
with agents and corrects drift or bugs they find.

What keeps that checkable is mechanical rather than a matter of trust:
the loop generates tests and linter configurations, so its gates give
the same answer every time, and it builds command-line tools, with clear
documentation, that a person can use to inspect the same state the loop sees.

Some classes of change are held for a human by design: releases, anything
touching the loop's own safety rails, and anything that could publish to
a package registry. Everything else is not.

**Origin.** This project was built in spec-driven development sessions with
Claude Opus 5.5, with a human reviewing every change. The text below records
that period: what it says about human review holds for the code written then,
not for the merges the loop makes now.

---
<!-- hallucinogen:tending-disclaimer end -->

This repository — Rust, shell, Nix, fixtures, specs and this prose — was
written by [Claude Code](https://claude.com/claude-code) running Anthropic's
**Claude Opus** models. Commits a model wrote carry a
`Co-Authored-By: Claude …` trailer naming the model; the current ratio is
whatever these two commands say, which is the point of not writing it down
here:

```sh
git log --format=%B | grep -c 'Co-Authored-By: Claude'
git rev-list --count HEAD
```

A human owns every decision, reviews every diff, and is accountable for what
ships.

That is the disclaimer. The rest of this file is why it is stated as a design
note rather than as an apology, and what a reader can check for themselves.

## Why say it at all

A model writes plausible code, and plausible is not correct. A reader who does
not know how a repository was produced cannot calibrate how hard to look at it.

Two reasons, and only the first is the obvious one.

The second is specific to this tool. xenolith rewrites other people's files:
it moves a script out of a Nix attribute, a justfile recipe or an hk step into
a file of its own, and replaces it with a line that runs that file. An
extraction that quietly changes what runs — drops an `errexit`, reorders two
commands, loses a recipe line's `-` prefix — is worse than no tool at all. A
repository built by a model, arguing that every embed should be checked
rather than trusted, has to hold itself to that first. Everything below is an
attempt to make the provenance checkable instead of merely disclosed.

## The method is spec-driven development, federated

[`SPEC.md`](../SPEC.md) is the law rather than a description written
afterwards, and it is not one file: every directory that owns something has
its own `SPEC.md` with its constraints, invariants, tasks and a bug log, and
the root's `§F` table names the nodes. The `federated nodes` badge in the
[README](../README.md) counts them, generated rather than typed.

A behaviour change starts as a spec row, lands as a failing test, and only
then as code — a guard checks that order in the history. A rule and its
checker land in the same commit, because a rule with no runner is a comment.

## The guardrails are git hooks that also run on CI

Entering the dev shell (`nix develop`, or `direnv allow`) installs the hooks,
which run [hk](https://github.com/jdx/hk) against one definition of the gate
in [`hk.pkl`](../hk.pkl) — the fast set on commit, everything on push.
[`ci.yml`](../.github/workflows/ci.yml) calls that same definition on three
platforms, so a laptop and a runner cannot disagree.
[`INTEGRATION.md`](INTEGRATION.md) has the full flow.

The badges in the README are generated from the files that own each number
— `Cargo.toml`, `hk.pkl`, `.coverage`, `.lint-debt`, `flake.lock`, `ci.yml`
— by `xenolith-dev`, this repository's own unpublished tooling, and the gate
fails when one drifts. A number typed into prose is true the day it is
written and quietly wrong after; this document follows the same rule and
states no counts of its own.

## The record is deliberately unflattering

`nix:B2` records that the flake's own clippy check was a three-line shell
script inside a Nix string — and that the repository's own `xnl check`
flagged it as a `sequence`. **The repository held the exact embed it exists
to forbid**, in the file that defines how it is checked.

`src/registry:B11` records that the shell host shipped and was never
registered, so every `.sh` file in the tree went unclaimed and unscanned —
and the dogfood step stayed green over scripts it had never read. A gate that
passes by not looking is the failure this project is built to catch in other
people's repositories.

Both are here rather than in a footnote because a tool that claims to find
what hides inside files is worth exactly as much as its record of what hid
inside its own.

## What a reader should actually check

In the order it matters:

1. **Does the gate run for you?** `nix develop` then `hk check --all`. If a
  claim on this page is false, that is where it shows.
2. **Do the `§B` rows look real or curated?** `nix:B2` and
  `src/registry:B11` are above, unedited. Judge the rest by them.
3. **Do the invariants have runners?** An invariant that no test, guard or
  gate step executes is a comment with a number on it. Finding one is a
  useful bug report.
4. **Try it on your own files.** `xnl check` only reports; `xnl extract`
  without `--write` only prints a diff. Nothing changes until you ask.

## Accountability

The human named in [`LICENSE`](../LICENSE) is responsible for this code,
including the parts a model wrote and the parts nobody caught. "The LLM wrote
it" is an explanation of provenance, never a transfer of responsibility.

Bug reports are welcome and unflattering ones are more useful — see
[`SECURITY.md`](SECURITY.md) for the ones that should not be public, and
[`CONTRIBUTING.md`](CONTRIBUTING.md) for everything else.

## Deeper

[`AGENTS.md`](../AGENTS.md) is the working guide ·
[`CONTRIBUTING.md`](CONTRIBUTING.md) is the loop ·
[`INTEGRATION.md`](INTEGRATION.md) is the gate and its known gaps ·
[`SPEC.md`](../SPEC.md) is the law and the backlog.
