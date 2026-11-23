# Receipt: Exact Geometric Predicates

Task:
flyingrobots/bunny#145, "Math: Add robust fixed geometric predicates", under the contract
identity `bunny-predicates/1` (`PREDICATES_CONTRACT_ID`) in `bunny-geom`: exact 2D and 3D
orientation (the sign of the exact determinant for every representable input), the exact
doubled signed area and sextupled signed volume with separately named forms rounded once to
Q32.32, exact barycentric numerators over a positive denominator for a point and a 2D
triangle (and a point exactly in the plane of a 3D triangle), the exact location
classification with its precedence, closed and open containment, and the weights rounded
once each. Additive: no existing public item, output bit, golden, digest or ID changes.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*
.repo-respect/receipts/2026-11-23-unproject-ray.md
crates/bunny-geom/{Cargo.toml,README.md,src/lib.rs,src/plane.rs}
crates/bunny-geom/tests/{determinism.rs,plane_span_sweep_tests.rs}
crates/bunny-linalg/{src/lib.rs,src/fixed_vec2.rs,src/unproject/wide.rs}
crates/bunny-query/src/{lib.rs,barycentric.rs}
crates/bunny-num/tests/lerp_tests.rs
docs/topics/coordinate-law/README.md (canonical 3D frame, canonical 2D plane, viewport)
docs/topics/plane-clipping/{README.md,test-plan.md}
docs/topics/ray-tracing/README.md (exact triangle test)
docs/topics/lerp/{README.md,test-plan.md}
xtask/src/topic_docs.rs
flyingrobots/bunny#145, #424

Files edited:
docs/topics/predicates/test-plan.md (new: PRED-REQ-001 to -009, PRED-TP-001 to -012)
crates/bunny-geom/tests/predicates_oracle/mod.rs (new: independent big-integer oracle)
crates/bunny-geom/tests/predicate_orientation_tests.rs (new)
crates/bunny-geom/tests/predicate_barycentric_tests.rs (new)
crates/bunny-geom/tests/predicate_sweep_tests.rs (new)
.repo-respect/receipts/2026-11-23-geometric-predicates.md

Topic docs:
docs/topics/predicates/: new topic with its own prefix, PRED-REQ-001 to PRED-REQ-009 and
PRED-TP-001 to PRED-TP-012. No other ledger uses the PRED- prefix.

Generated artifacts:
None.

Checks run:
First commit (tests and planned ledger only): the predicate tests fail to compile (the API
does not exist). cargo fmt --all; topic-docs clean (12 planned cases, zero duplicate IDs).

Known risks:
Pending implementation.

Human reviewer:
James Ross (maintainer); review pending.
