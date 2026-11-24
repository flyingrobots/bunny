# Receipt: Uniform Grid Broadphase

Task:
flyingrobots/bunny#121: a deterministic uniform grid broadphase in `bunny-broadphase`
(`uniform_grid_pairs`, contract identity `bunny-uniform-grid/1`). Cells are identified by their
exact integer coordinates `floor(raw / cell_raw)`; candidates come from sorting (cell, object)
keys in caller scratch, not from hashing. Pairs have the form and order of `sweep_and_prune`.
Additive: no existing public item, output bit, golden, digest or ID changes.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*, docs/CODE_DOJO.md (limits)
.repo-respect/receipts/2026-11-23-unproject-ray.md
crates/bunny-broadphase/{Cargo.toml,README.md,src/lib.rs,src/sweep_and_prune.rs,src/utils.rs}
crates/bunny-broadphase/tests/{determinism.rs,bvh_sah_allocation_tests.rs,common/mod.rs}
crates/bunny-geom/src/{lib.rs,bounds.rs} (FixedAabb3::new/try_new, BoundsError::InvertedBox)
docs/topics/bvh-construction/{README.md,test-plan.md}, docs/README.md
flyingrobots/bunny#485 (crates/bunny-mesh/src/topology/: the caller-scratch model)
xtask/src/{code_dojo.rs,topic_docs.rs} (limits, case statuses)

Files edited:
crates/bunny-broadphase/tests/uniform_grid_tests.rs (new)
crates/bunny-broadphase/tests/uniform_grid_allocation_tests.rs (new)
docs/topics/uniform-grid/test-plan.md (new: GRID-REQ-001 to -009, GRID-TP-001 to -015)
.repo-respect/receipts/2026-11-24-uniform-grid-broadphase.md

Topic docs:
docs/topics/uniform-grid/: a new topic with its own prefix GRID-, starting at 001. No other
ledger uses GRID-.

Generated artifacts:
None.

Checks run:
First commit (tests and planned cases only): the grid tests fail to compile, because
`bunny_broadphase::uniform_grid` does not exist yet.

Known risks:
To be completed with the kernel.

Human reviewer:
James Ross (maintainer); review pending.
