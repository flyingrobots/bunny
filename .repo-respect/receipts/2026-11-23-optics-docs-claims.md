# Receipt: Optics Docs Claims

Task:
`flyingrobots/bunny#299`: correct the optics chapter where it overstates the code (the facing sign, the `SurfaceHit::new` refusal), say what the shading identity inherits without a version, say that the pinhole camera has no identity string, state the rule for sample indices past 1 023, and pin the corrected claims with three tests. Documentation, one rustdoc comment and tests only; no kernel, signature, golden or ID changed.

Files read:
- `docs/topics/optics/README.md`
- `docs/topics/optics/test-plan.md`
- `crates/bunny-optics/src/shading/hit.rs`
- `crates/bunny-optics/src/sampler.rs`
- `crates/bunny-num/src/fixed_angle.rs`
- `crates/bunny-linalg/src/lib.rs`
- `crates/bunny-linalg/src/fixed_vec3.rs`
- `crates/bunny-optics/tests/shading_refusal_tests.rs`

Files edited:
- `docs/topics/optics/README.md`
- `docs/topics/optics/test-plan.md`
- `crates/bunny-optics/src/shading/hit.rs` (a doc comment)
- `crates/bunny-optics/tests/optics_docs_claims_tests.rs`
- `CHANGELOG.md`
- `.repo-respect/receipts/2026-11-23-optics-docs-claims.md`

Topic docs:
- `docs/topics/optics/README.md`: Facing And Secondary Rays, Shading Identity, Ray Generation, Pixel Values, Status.
- `docs/topics/optics/test-plan.md`: OPTICS-TP-270 to OPTICS-TP-272, with their contract-graph cases.

Generated artifacts:
- None.

Checks run:
- Gates are listed in the pull request description.
- Local only, not part of the gates: `OPTICS-TP-272` fails when one sine-table entry is raised by `2^33`, `2^31` or `2^30` (62-bit units); restored afterwards.

Known risks:
- `OPTICS-TP-272` pins a digest of current outputs; it is a regression pin, not an independent oracle.
- Local toolchain 1.100.0, CI pins 1.96.0.

Human reviewer:
@flyingrobots
