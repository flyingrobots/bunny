# Receipt: GPU Backend Proposal

Task:
Document what it would take for Bunny to run its deterministic kernels on a GPU with
bit-identical results: why integer GPU arithmetic can be exact, the 64/128-bit word-size
obstacle, a quarantined `bunny-gpu` crate, shader-route options, CI testing with software
adapters, milestones, effort estimates and risks.

Files read:
CODE_STANDARDS.md
README.md
docs/README.md
docs/design/0001-bunny-graphics-commons.md

Files edited:
docs/design/0005-gpu-compute-backend.md
.repo-respect/receipts/2026-11-13-gpu-backend-proposal.md

Topic docs:
None. This is a proposal under docs/design/, not current behaviour.

Generated artifacts:
None.

Checks run:
cargo run --locked -p xtask -- code-dojo --all

Known risks:
Statements about WGSL 64-bit support, wgpu's SHADER_INT64 feature, rust-gpu capabilities
and software adapters on hosted CI runners are from memory and are marked "verify first"
in the document. Effort and speedup figures are estimates.

Human reviewer:
James Ross asked for this write-up on 2026-11-13; review pending.
