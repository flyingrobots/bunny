# Bunny - Bearing

This is the living signpost for what the repo is doing now. It should be short,
current, and honest. Deeper history belongs in `ROADMAP.md`, `CHANGELOG.md`, and
goalpost documents.

## Current Position

| Field | State |
| --- | --- |
| Active release | `v0.4.0` - Quantized Meshes & Codecs |
| Active branch | `goalpost/v0.4.0-gp2` |
| Open PR | #104 - Complete v0.4.0 GP2 file format adapters |
| Current gate | CI and review for GP2 |
| Next goalpost | `v0.4.0-GP3` compression decoders |

## Recent Truth

* The Pre-GP2 Completion Integrity Gate is complete. Previously overclaimed
  roadmap items were fact-checked, finished, and documented with evidence.
* GP1 shipped `bunny-mesh` compressed mesh layouts: 16-bit vertex quantization,
  stable 16-bit and 32-bit triangle buffers, and SHA-256 mesh hashing.
* GP2 adds `bunny-codec`, including zero-copy binary PLY and OBJ parser
  contracts, Stanford Bunny-derived fixtures, native zero-allocation witnesses,
  and native/WASM regression tests.
* Codec ingress now rejects non-finite vertex coordinates and out-of-bounds PLY
  face indices before returning borrowed mesh views.
* CI is pinned to Rust 1.96.0 and runs native workspace tests plus headless
  `wasm-pack test --node` for every WASM-compatible library crate.

## Immediate Next Work

1. Let PR #104 finish CI and review.
2. Resolve any review threads without weakening the GP2 contract.
3. Merge GP2 only after checks are green and review is clean.
4. Sync `main`, then open the GP3 branch for compression decoders.

## Watchpoints

* Do not mark a roadmap slice done unless the implementation, tests, and docs
  all support the claim.
* Keep host-side tooling (`bunny-wesley`, `xtask`) distinct from
  WASM-compatible library crates in docs and CI claims.
* Keep codec parsers zero-copy on accepted paths while still validating payload
  structure before returning borrowed views.
* Matrix and quaternion profiles are still absent from `bunny-linalg`; future
  transform work must either add them or explicitly stay out of that scope.

## Last Known Local Verification

The GP2 branch was verified with:

```bash
cargo +1.96.0 fmt --all -- --check
cargo +1.96.0 clippy --locked --workspace --all-targets -- -D warnings
cargo +1.96.0 test --locked --workspace --all-targets
cargo +1.96.0 check --locked -p bunny-num -p bunny-linalg -p bunny-geom \
  -p bunny-contract -p bunny-query -p bunny-broadphase -p bunny-mesh \
  -p bunny-codec --target wasm32-unknown-unknown
```

The full local WASM loop also passed for `bunny-num`, `bunny-linalg`,
`bunny-geom`, `bunny-contract`, `bunny-query`, `bunny-broadphase`,
`bunny-mesh`, and `bunny-codec` with `wasm-pack test --node --locked`.
