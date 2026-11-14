# Receipt: bunny-codec Readers Without crates.io Dependencies

Task:
Implement flyingrobots/bunny#291. Make the `bunny-codec` readers usable without
`sha2` and its crates.io tree. Only `bunny_mesh::compute_mesh_hash` uses `sha2`, so
`sha2` becomes optional behind a default-on `mesh-hash` feature of `bunny-mesh`, and
`bunny-codec` depends on `bunny-mesh` with `default-features = false` and forwards the
feature as its own default-on `mesh-hash`. The full gate checks the no-default
configuration of both crates.

Files read:
In progress.

Files edited:
In progress.

Topic docs:
docs/topics/ascii-ply/test-plan.md (APLY-REQ-012, APLY-REQ-013; APLY-TP-052 to -054)

Generated artifacts:
None.

Checks run:
In progress.

Known risks:
In progress.

Human reviewer:
James Ross (maintainer); review pending.
