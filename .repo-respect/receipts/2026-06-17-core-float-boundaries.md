# Receipt: Core Float Boundary Cleanup

Task:
Resolve the self-discovered PR #106 full-gate failure posted at
`https://github.com/flyingrobots/bunny/pull/106#issuecomment-4734005573`.
The tightened Code Dojo float-boundary rule exposed remaining inferred float
literals in deterministic core code.

Files read:

* `crates/bunny-broadphase/src/bvh/split.rs`
* `crates/bunny-broadphase/src/utils.rs`
* `crates/bunny-geom/src/conversions.rs`
* `crates/bunny-num/src/fixed_q32_32/conversions.rs`
* `crates/bunny-num/src/fixed_q32_32.rs`
* `crates/bunny-num/src/lib.rs`

Files edited:

* `.repo-respect/receipts/2026-06-17-core-float-boundaries.md`
* `crates/bunny-broadphase/src/bvh/split.rs`
* `crates/bunny-broadphase/src/utils.rs`
* `crates/bunny-geom/src/conversions.rs`
* `crates/bunny-num/src/fixed_q32_32/conversions.rs`

Checks run:

* `cargo run --locked -p xtask -- code-dojo --all` (red before fix)
* `cargo run --locked -p xtask -- code-dojo --all` (green after fix)
* `git diff --check`

Known risks:

* The radius validation now spells positive zero as `Scalar::from_bits(0)`,
  preserving the old `radius < +0.0` behavior, including accepting `-0.0`.

Human reviewer:
James
