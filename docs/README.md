# Bunny Documentation

This directory is Bunny's living technical book.

The repository keeps different kinds of documents because they answer different
questions. Current behavior, historical rationale, test design, roadmap state,
and release chronology must not compete with each other.

## Current Truth

Living topic chapters describe how Bunny works now. They are updated in place
when code, public behavior, invariants, or supported usage changes.

| Chapter | Owns |
| --- | --- |
| [`topics/coordinate-law/`](topics/coordinate-law/) | Coordinate spaces, handedness, units, winding, transform naming, and projection reservations. |
| [`NUMERIC_CONSTITUTION.md`](NUMERIC_CONSTITUTION.md) | Q32.32 arithmetic law, construction policy, rounding, overflow, and numeric golden-vector expectations. |
| [`MATH_GEOMETRY_CAPABILITY_MAP.md`](MATH_GEOMETRY_CAPABILITY_MAP.md) | Bunny's durable math/geometry ownership boundary, non-goals, missing surface, and build order. |
| [`CODE_DOJO.md`](CODE_DOJO.md) | Local and CI quality gate behavior. |
| [`TESTING.md`](TESTING.md) | Repository-level test and validation commands. |
| [`PROCESS.md`](PROCESS.md) | Goalpost delivery loop, review discipline, documentation source-of-truth rules, and release publication flow. |

## Topic Folder Standard

Durable concepts that will be changed by more than one pull request should live
under `docs/topics/<topic>/`.

Recommended files:

| File | Required | Role |
| --- | --- | --- |
| `README.md` | Yes | Current behavior and invariants for the topic. |
| `test-plan.md` | Yes | Current verification design: requirements, fixtures, cases, oracles, determinism proof, and gaps. |
| `architecture.md` | Optional | Structure, data flow, and module boundaries when the topic is large enough to need them. |
| `rationale.md` | Optional | Durable tradeoffs and rejected approaches that still help maintainers. |

The topic `README.md` is the chapter. It answers "what is true now?" Historical
design documents, issues, pull requests, and goalpost notes may explain why that
truth changed, but they are not competing current references.

## Historical Records

| Location | Role |
| --- | --- |
| [`design/`](design/) | Historical design decisions and proposal-era notes. |
| [`goalposts/`](goalposts/) | Delivery evidence for completed goalposts. |
| [`../ROADMAP.md`](../ROADMAP.md) | Versioned delivery plan and GitHub issue anchors. |
| [`../CHANGELOG.md`](../CHANGELOG.md) | Release chronology. |

Historical records may be corrected when factually wrong. When a historical
document could mislead readers about current behavior, add a short note pointing
to the relevant living topic chapter instead of rewriting the whole record.

## Design And Test Gate

Before implementing a nontrivial behavior change:

1. Update the living topic chapter or create a new topic folder.
2. Update the topic test plan with requirements, fixtures, oracles, and test
   cases before implementation.
3. Write tests from the plan.
4. Implement the behavior.
5. Update the test-plan matrix with the actual test names.
6. Record release-visible changes in `CHANGELOG.md`.

For small bug fixes, the same discipline can be scaled down, but every behavior
change still needs a test or a written reason why the existing tests already
cover it.

## Test Plan Standard

Topic test plans should cover:

- golden paths
- known failures
- edge cases
- unusual inputs
- stable error kinds
- determinism proofs
- invariants and property tests
- metamorphic or differential checks when no simple oracle exists
- stress, fuzz, and replay strategy
- fixture provenance and regeneration
- public-surface test goals
- open verification gaps

Tests should assert public behavior and stable contract artifacts. They should
not assert private implementation details, scrape human-oriented output, or use
documentation prose as the oracle.

Valid assertions include structured return values, state transitions, error
kinds, raw fixed-point values, canonical bytes, stable hashes, generated DTOs,
and explicitly documented artifacts. Incidental stdout, stderr, log text, and
private helper behavior are not stable test contracts unless an API explicitly
makes them so.
