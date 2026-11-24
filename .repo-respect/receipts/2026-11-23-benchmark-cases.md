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
docs/topics/performance/README.md (Adding a case: the copied harness, `BASE_CRATES`, the
groups outside `default`, the new cases; Continuous integration: required checks, one-sided
cases)
docs/topics/performance/test-plan.md (scope, fixtures, PERF-REQ-015 and -016, PERF-TP-070 to
-086)
CHANGELOG.md
.repo-respect/receipts/2026-11-23-benchmark-cases.md

Topic docs:
docs/topics/performance/: new IDs PERF-REQ-015 and PERF-REQ-016, PERF-TP-070 to PERF-TP-086,
contiguous above the highest in use on `main` (PERF-REQ-014, PERF-TP-069). `topic-docs`
reports zero duplicate IDs. No ledger entry or measured number is added or changed.

Generated artifacts:
None.

Checks run:
First commit: the new determinism tests fail on purpose (digests not yet recorded); every
other new test passes (`cargo test --locked -p bunny-perf --all-features`: 36 passed, 5 failed).
Second commit, each its own command on the host (local Rust 1.100.0; CI pins 1.96.0): cargo fmt
--all --check; cargo test --locked --workspace --all-targets --all-features (142 test result
lines, none failed); cargo test -p bunny-perf with all features (41 passed) and with default
features (30 passed); cargo run --locked -p xtask -- code-dojo-generated; code-dojo-rust --all;
code-dojo-determinism --enforce; code-dojo-perf-groups (all 21 groups alone and together);
topic-docs (zero duplicate IDs); repo-respect check --branch. cargo clippy --locked -p bunny-perf
-p xtask --all-targets --all-features -D warnings: clean in every added or changed line; the
newer toolchain's lints remain in untouched lines (`memory.rs:57`, several `assert!(x.is_empty())`
in xtask tests). A standalone copy of the harness, with the manifest `cargo xtask perf` writes,
was built in release mode with all 21 groups to confirm the copy builds; the binary was not run.
No timing measurement was run: measurements go through the guarded container runner, and
none was taken.
Not run locally: cargo deny and markdownlint (CI's Dojo job), the pinned 1.96.0 clippy, wasm.

Known risks:
The new cases run in the pull-request perf job on both sides (the crates exist on `main`), so
they enter the 20 % gate on this pull request with identical code on both sides.

Human reviewer:
James Ross (maintainer); review pending.
