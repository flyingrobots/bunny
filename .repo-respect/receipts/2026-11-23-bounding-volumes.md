# Receipt: Bounding Volumes

Task:
flyingrobots/bunny#151: box helpers on `FixedAabb3` in `bunny-geom` under the contract identity
`bunny-bounds/1` (`from_points`, `from_triangle`, `merge`, `expand`, `expand_axes`,
`from_sphere`, `bounding_sphere`, with the typed `BoundsError`), `mesh_bounds` in `bunny-mesh`
(with `MeshBoundsError`), and dedicated tests of the existing `union_aabb`,
`TraceSphere::bounds` and `TraceTriangle::bounds` as they behave today. Additive: no existing
public item, output bit, golden, digest or ID changes.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*, docs/NUMERIC_CONSTITUTION.md
.repo-respect/receipts/2026-11-23-transform-composition.md
scripts/publish-crates.sh, crates/*/Cargo.toml (dependency direction)
crates/bunny-geom/src/{lib.rs,conversions.rs}, crates/bunny-geom/tests/geom_tests.rs
crates/bunny-broadphase/src/utils.rs, crates/bunny-broadphase/tests/common/mod.rs
crates/bunny-query/src/trace.rs, trace/{sphere.rs,box_hit.rs}
crates/bunny-query/tests/trace_sphere_tests.rs
crates/bunny-mesh/src/{lib.rs,hash.rs}
crates/bunny-num/src/{lib.rs,fixed_q32_32.rs} (sqrt_u128)
crates/bunny-linalg/tests/transform_tests.rs (seeded-test pattern)
xtask/src/{topic_docs.rs,code_dojo.rs} (ledger rules, file limits)
docs/topics/{transform,lerp}/test-plan.md, docs/topics/ray-tracing/README.md (box inversion)
flyingrobots/bunny#151; open pull requests flyingrobots/bunny#469 and #471 (file lists)

Files edited:
crates/bunny-geom/tests/bounds_tests.rs (new)
crates/bunny-mesh/tests/mesh_bounds_tests.rs (new)
crates/bunny-broadphase/tests/union_aabb_tests.rs (new)
crates/bunny-query/tests/trace_bounds_tests.rs (new)
docs/topics/bounding-volumes/test-plan.md (new: BOUNDS-REQ-001 to -008, BOUNDS-TP-001 to -016)
.repo-respect/receipts/2026-11-23-bounding-volumes.md

Topic docs:
docs/topics/bounding-volumes/ is a new topic with its own BOUNDS- prefix; no other topic uses
it.

Generated artifacts:
None.

Checks run:
First commit (tests and planned cases only): cargo test --locked -p bunny-geom --test
bounds_tests fails to compile (the helpers do not exist); topic-docs and repo-respect check
--branch clean.

Known risks:
None yet; see the pull request.

Human reviewer:
James Ross (maintainer); review pending.
