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
crates/bunny-query/tests/degeneracy_characterization_tests.rs (new)
crates/bunny-broadphase/tests/degeneracy_characterization_tests.rs (new)
crates/bunny-mesh/tests/degeneracy_characterization_tests.rs (new)
crates/bunny-linalg/tests/degeneracy_characterization_tests.rs (new, confirms flyingrobots/bunny#483)
docs/topics/degeneracy/README.md (new chapter: the inventory of current behaviour)
docs/topics/degeneracy/test-plan.md (new: DEGEN-REQ-001 to -010, DEGEN-TP-001 to -080)
docs/README.md (one topic row, sorted position), CHANGELOG.md (one bullet)
.repo-respect/receipts/2026-11-24-degeneracy-inventory.md

Topic docs:
docs/topics/degeneracy/: a new topic with its own prefix DEGEN-, starting at 001. No other
ledger uses DEGEN-. The in-review shapes of pull requests #495, #496 and #498 are inventoried
in the pull request and on the issue, not in the chapter, which describes the default branch.

Generated artifacts:
None.

Checks run:
Each its own command on the host (local Rust 1.100.0; CI uses the pinned 1.96.0):
cargo fmt --all --check; cargo test --locked --workspace --all-targets --all-features (171 test
result lines, all ok); cargo run --locked -p xtask -- code-dojo-generated; code-dojo-rust --all;
code-dojo-determinism --enforce; code-dojo-perf-groups; topic-docs (zero duplicate IDs);
repo-respect check --branch; git diff --check. cargo clippy --locked --keep-going on the five
touched crates --all-targets --all-features -D warnings: nothing in the new files (the only
reports are newer-toolchain lints in crates/bunny-geom/tests/polygon_clip_tests.rs).
In-review branches: scratch probes (not committed) run against the heads of #495, #496 and #498.
Not run locally: wasm-pack (CI's WebAssembly job), cargo deny and markdownlint-cli2 (CI's Dojo
job), the pinned 1.96.0 clippy. No mutation checks: no kernel is added or changed.

Known risks:
Characterization tests pin current behaviour, including behaviour the inventory reports as
inconsistent; they are labelled as such and are not an endorsed contract.

Human reviewer:
James Ross (maintainer); review pending.
