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
W3C WGSL CRD 2026-11-06, Metal Shading Language spec 4.1, wgpu docs and source (commit bceb82b), rust-gpu source (commit 60d1e25), wgpu CI workflow, GitHub runner image list (all accessed 2026-11-13)

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
Toolchain claims were verified by reading primary sources on 2026-11-13; no shader was
compiled or run and no CI job selected a software adapter. The document lists what remains
unverified. Effort and speedup figures are estimates.

Human reviewer:
James Ross asked for this write-up on 2026-11-13; review pending.
