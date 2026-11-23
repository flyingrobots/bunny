# Receipt: Plane Shape Classification

Task:
flyingrobots/bunny#162, the remainder: classify a segment, a triangle and an AABB against a
`FixedPlane3`. New type `PlaneSpan` (Front, Behind, Straddling, Coplanar) and three methods,
`classify_segment`, `classify_triangle` and `classify_aabb`, each defined by the existing point
rule (`FixedPlane3::classify`) at the endpoints, vertices or eight corners. Additive: no existing
public item, output bit, golden, digest or ID changes.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*
.repo-respect/receipts/2026-11-23-rotation.md
crates/bunny-geom/{Cargo.toml,README.md,src/lib.rs,src/plane.rs,src/plane/polygon.rs}
crates/bunny-geom/tests/{plane_clip_tests.rs,polygon_clip_tests.rs}
crates/bunny-num/src/fixed_q32_32.rs (checked_mul, checked_add, checked_sub)
docs/topics/plane-clipping/{README.md,test-plan.md}
docs/topics/ray-tracing/README.md and test-plan.md (inverted-box approach, flyingrobots/bunny#295)
docs/topics/rotation/test-plan.md (mutation ledger format)
xtask/src/code_dojo.rs (category limits), xtask/src/topic_docs.rs (case rules)
flyingrobots/bunny#162 and flyingrobots/bunny#295

Files edited:
- `crates/bunny-geom/tests/plane_span_sweep_tests.rs`
- `crates/bunny-geom/tests/plane_span_tests.rs`
- `docs/topics/plane-clipping/test-plan.md`

Topic docs:
docs/topics/plane-clipping/test-plan.md: planned PC-REQ-014 to -019 and PC-TP-025 to -037, a free
contiguous range above the existing PC-REQ-013 and PC-TP-024.

Generated artifacts:
None.

Checks run:
First commit (tests and planned cases only): cargo test --locked -p bunny-geom --test
plane_span_tests --test plane_span_sweep_tests fails to compile (PlaneSpan and the three methods
do not exist); cargo run --locked -p xtask -- topic-docs clean (13 planned, 0 duplicate IDs);
cargo run --locked -p xtask -- repo-respect check --branch.

Known risks:
None yet.

Human reviewer:
James Ross (maintainer); review pending.
