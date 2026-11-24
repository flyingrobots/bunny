# Receipt: Degeneracy Inventory

Task:
flyingrobots/bunny#116: an inventory of what every public shape constructor and query in
`bunny-geom`, `bunny-query`, `bunny-broadphase` and `bunny-mesh` does today with each degenerate
input, each cell pinned by a characterization test, published as the `degeneracy` topic. The
uniform policy is a proposal for the owner in the pull request, not a ruling. No existing
function's behaviour, signature, golden, digest, requirement ID or case ID changes.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md (Rule 7), .githooks/*
.repo-respect/receipts/2026-11-24-uniform-grid-broadphase.md
crates/bunny-geom/src/{lib.rs,bounds.rs,plane.rs,plane/span.rs,plane/polygon.rs,frustum.rs,
frustum/sphere.rs,transform.rs,shapes.rs,shapes/*.rs,predicates/barycentric.rs}
crates/bunny-query/src/{lib.rs,ray.rs,closest.rs,trace.rs,trace/*.rs,frame.rs}
crates/bunny-broadphase/src/{lib.rs,utils.rs,sweep_and_prune.rs,bvh.rs,bvh/build.rs,traversal.rs,
refit.rs,compact.rs,closest_hit.rs,uniform_grid.rs,uniform_grid/*.rs}
crates/bunny-mesh/src/{lib.rs,bounds.rs,hash.rs,topology.rs,topology/finding.rs}
crates/bunny-broadphase/tests/union_aabb_tests.rs, docs/topics/uniform-grid/test-plan.md
flyingrobots/bunny#477, #483, #472 and the branches of pull requests #495, #496 and #498

Files edited:
crates/bunny-geom/tests/degeneracy_characterization_tests.rs (new)
.repo-respect/receipts/2026-11-24-degeneracy-inventory.md

Topic docs:
None yet.

Generated artifacts:
None.

Checks run:
cargo test --locked -p bunny-geom --test degeneracy_characterization_tests: 14 passed.

Known risks:
Characterization tests pin current behaviour, including behaviour the inventory reports as
inconsistent; they are labelled as such and are not an endorsed contract.

Human reviewer:
James Ross (maintainer); review pending.
