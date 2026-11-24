# Receipt: Benchmark Cases for Conversions, Mesh, Codecs and Sweep-and-Prune

Task:
flyingrobots/bunny#130, the remainder. `bunny-perf` gains cases for the areas the issue names
that had none on `main`: Q32.32 conversions (`num/try_from_f32`, `num/to_f32`), mesh
quantization and the mesh hash (`mesh/quantize_vertex`, `mesh/dequantize_vertex`, `mesh/hash`),
decoding each `bunny-codec` format (`codec/ply_ascii_decode`, `codec/ply_binary_decode`,
`codec/obj_decode`, `codec/compressed_decode`) and `broadphase/sweep_and_prune`. Harness only:
no Bunny kernel, existing case, gate or threshold changes.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*, .repo-respect/receipts/2026-11-23-unproject-ray.md
crates/bunny-perf/{Cargo.toml,src/*.rs}
xtask/src/{perf.rs,perf_report.rs,feature_configs.rs,code_dojo.rs}
.github/workflows/perf.yml
crates/bunny-mesh/src/{lib.rs,hash.rs}, crates/bunny-mesh/Cargo.toml
crates/bunny-codec/src/{lib.rs,ply.rs,ply/header.rs,ply_ascii.rs,obj.rs,compressed.rs,
compressed/read.rs,compressed/view.rs}, crates/bunny-codec/tests/{compressed_tests.rs,
fixtures/*}
crates/bunny-broadphase/src/{sweep_and_prune.rs,utils.rs}
crates/bunny-num/src/fixed_q32_32.rs
docs/topics/performance/{README.md,test-plan.md}
flyingrobots/bunny#130, #458, #473, #489

Files edited:
crates/bunny-perf/Cargo.toml (groups `convert`, `sweep`, `mesh`, `codec`; optional `bunny-mesh`
and `bunny-codec`)
crates/bunny-perf/src/{main.rs,cases.rs} (module and case registration)
crates/bunny-perf/src/{grid.rs,mesh.rs,codec.rs,sweep.rs,convert.rs} (new)
xtask/src/perf.rs (`BASE_CRATES` gains `bunny-codec` and `bunny-mesh`; one new test)
Cargo.lock (the `bunny-perf` entry's dependency list)
.repo-respect/receipts/2026-11-23-benchmark-cases.md

Topic docs:
Pending in a later commit on this branch.

Generated artifacts:
None.

Checks run:
First commit: the new determinism tests fail on purpose (digests not yet recorded); every
other new test passes (`cargo test --locked -p bunny-perf --all-features`: 36 passed, 5 failed).
No timing measurement was run.

Known risks:
The new cases run in the pull-request perf job on both sides (the crates exist on `main`), so
they enter the 20 % gate on this pull request with identical code on both sides.

Human reviewer:
James Ross (maintainer); review pending.
