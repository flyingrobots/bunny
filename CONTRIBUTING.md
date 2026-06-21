# Contributing to Bunny

Bunny is a deterministic Rust library, and the contribution process treats
documentation as part of the contract. The goal is not to collect design notes
forever. The goal is to keep a reader, a reviewer, and the test suite looking
at the same truth.

The short version is:

```text
current truth -> planned verification -> executable evidence -> historical reasoning
```

If you are new to the project, read this as Bunny's documentation technique.
Every artifact has one job, and a pull request should leave those artifacts in
agreement.

## The Mental Model

Think of Bunny's docs as a contract graph.

A feature or concept has current behavior, requirements, planned test cases,
fixtures, executable tests, and historical design decisions. Those pieces are
connected with stable names and IDs so maintainers can answer a simple question:
"What claim is this code making, and what evidence proves it?"

That means Bunny separates four kinds of information:

| Artifact | Job |
| --- | --- |
| Current truth | Describe what is true in `HEAD` right now. |
| Planned verification | Describe how behavior will be tested before it is implemented. |
| Executable evidence | Tests, doctests, fixtures, static checks, or generated artifacts that prove the behavior. |
| Historical reasoning | Explain why a decision was made without pretending to be the current reference. |

The most important rule is that current truth must not describe future behavior.
If a feature is planned but not implemented, it belongs in a test plan, issue,
design note, roadmap slice, or pull request. The living reference only changes
after the behavior and evidence exist.

## Where Documentation Lives

Durable concepts that will evolve across more than one pull request should use a
topic folder under `docs/topics/<topic>/`. The folder is the shelf for that
concept, like a chapter in a technical book.

| Path | Use it for |
| --- | --- |
| `docs/topics/<topic>/README.md` | Current behavior, invariants, public contract, and supported usage. |
| `docs/topics/<topic>/test-plan.md` | Requirements, planned cases, implemented evidence, fixtures, oracles, and known gaps. |
| `docs/topics/<topic>/architecture.md` | Optional structure, data flow, and module boundaries when the topic is large enough. |
| `docs/topics/<topic>/rationale.md` | Optional still-relevant tradeoffs and rejected alternatives. |
| `docs/design/` | Historical proposal-era design documents. |
| `docs/goalposts/` | Delivery evidence for completed goalposts. |
| `docs/README.md` | The documentation spine and topic index. |
| `docs/PROCESS.md` | The delivery loop, review discipline, and release workflow. |
| `ROADMAP.md` | Release train, goalposts, and GitHub issue anchors. |
| `CHANGELOG.md` | Release-visible changes. |
| `docs/BEARING.md` | Durable release posture and watchpoints. |

For example, `docs/topics/coordinate-law/README.md` states Bunny's current
coordinate conventions. Its sibling `test-plan.md` records the requirements,
test cases, oracles, and evidence that keep those conventions from drifting.

## How To Change Behavior

For meaningful behavior changes, use this sequence:

1. Write or update a proposal, design note, or rationale page if the change
   needs real design discussion.
2. Update the topic `test-plan.md` with planned cases before implementation.
   Each planned case should have a stable case ID, requirement IDs, an explicit
   oracle, an evidence type, and a status.
3. Write the smallest deterministic executable evidence that fails for the
   missing behavior.
4. Implement the behavior.
5. Update the topic `README.md` so it describes the behavior that now exists in
   `HEAD`.
6. Mark the planned cases as implemented and record the actual test names,
   fixture paths, doctests, or artifact anchors.
7. Update `CHANGELOG.md`, `ROADMAP.md`, or `docs/BEARING.md` when the change is
   release-visible or changes the project posture.

Small fixes can scale this down, but they should still preserve the same shape:
make the claim clear, add or identify evidence, implement the change, and keep
the current reference honest.

## Test Plans Are Contracts

Topic test plans are written for people and checked by tools.

The prose explains the intent, edge cases, determinism obligations, fixtures,
and known gaps. Fenced `toml` metadata blocks form the machine-readable contract
graph that `xtask` validates. Do not parse visual Markdown tables as data; tables
are for readers.

Each planned or implemented case should answer:

* Which requirement does this case cover?
* What exact behavior or invariant is being checked?
* What is the oracle?
* What kind of evidence proves it?
* Is the case planned, implemented, blocked, or retired?
* If implemented, what test, fixture, doctest, static check, or artifact is the
  evidence?

Good evidence asserts stable behavior: structured return values, state
transitions, error kinds, raw fixed-point values, canonical bytes, stable hashes,
generated DTOs, or documented artifacts. Avoid treating implementation details,
incidental log text, or documentation prose as the oracle.

## What Not To Do

Do not create a second current reference for the same topic. If a concept has a
topic folder, update that folder instead of scattering durable truth into a new
one-off document.

Do not update a living `README.md` to describe intended behavior before code and
tests exist. That turns the current reference into a proposal and makes the docs
lie on `main`.

Do not bury durable behavior only in an issue, RFC, pull request, or goalpost
note. Those are useful history, but they are not the current contract.

Do not leave planned or blocked test cases vague. A future maintainer should be
able to tell what evidence would close the gap.

Do not regenerate golden fixtures casually. Golden changes should be deliberate,
reviewable, and tied to a clear contract change.

## Local Checks

Run focused checks first, then broaden as the change becomes ready for handoff.
For documentation-only changes, start with:

```bash
markdownlint-cli2 CONTRIBUTING.md CHANGELOG.md docs/**/*.md README.md ROADMAP.md
git diff --check
cargo run --locked -p xtask -- topic-docs
```

Before opening or updating a goalpost pull request, run the full local gate:

```bash
cargo run --locked -p xtask -- code-dojo --all
```

Install the repo-local hooks once per clone:

```bash
bash scripts/install-githooks.sh
```

The hooks are guardrails. The commands above are the local contract, and GitHub
Actions is the merge gate.
