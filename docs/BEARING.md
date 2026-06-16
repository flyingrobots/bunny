# Bunny - Bearing

This is the living signpost for what the repo is doing now. It should be short,
current, and honest. Deeper history belongs in `ROADMAP.md`, `CHANGELOG.md`, and
goalpost documents.

## Current Position

| Field | State |
| --- | --- |
| Active release | `v0.4.0` - Quantized Meshes & Codecs |
| Active branch | `goalpost/v0.4.0-gp3` |
| Open PR | `#105` - GP3 compressed mesh decoder |
| Current gate | PR #105 review, GitHub CI, and CodeRabbit credit status |
| Next goalpost | `v0.4.0-GP3` merge gate |

## Recent Truth

* The Pre-GP2 Completion Integrity Gate is complete. Previously overclaimed
  roadmap items were fact-checked, finished, and documented with evidence.
* GP1 shipped `bunny-mesh` compressed mesh layouts: 16-bit vertex quantization,
  stable 16-bit and 32-bit triangle buffers, and SHA-256 mesh hashing.
* GP2 adds `bunny-codec`, including zero-copy binary PLY and OBJ parser
  contracts, Stanford Bunny-derived fixtures, native zero-allocation witnesses,
  and native/WASM regression tests.
* GP3 now adds the Bunny-native compressed mesh decoder profile, a borrowed
  zero-allocation compressed view, checked typed accessors, committed fixture
  bytes, malformed-input corpus tests, and allocation evidence.
* PR #104 merged GP2 into `main`. The GP2 goalpost now includes a captured
  witness table with repo-truth anchors for each completed implementation claim.
* Codec ingress now rejects non-finite vertex coordinates and out-of-bounds PLY
  face indices before returning borrowed mesh views.
* CI is pinned to Rust 1.96.0 and runs native workspace tests plus headless
  `wasm-pack test --node` for every WASM-compatible library crate.

## Immediate Next Work

1. Push GP3 review-fix commits to PR #105.
2. Confirm GitHub CI is green and unresolved PR review threads are zero.
3. Account for CodeRabbit's insufficient-credits status before merge.
4. Keep GP3 scoped to compression decoders; do not add new external file-format
   profiles in this goalpost.

## Watchpoints

* Do not mark a roadmap slice done unless the implementation, tests, and docs
  all support the claim.
* Keep host-side tooling (`bunny-wesley`, `xtask`) distinct from
  WASM-compatible library crates in docs and CI claims.
* Keep GP2 codec parser zero-copy claims intact. GP3 decoder claims are limited
  to borrowed raw payload sections plus typed checked accessors; no typed slice
  reinterpretation is claimed.
* Matrix and quaternion profiles are still absent from `bunny-linalg`; future
  transform work must either add them or explicitly stay out of that scope.

## Last Known Local Verification

The canonical checklist lives in `docs/TESTING.md`; this section is a status
snapshot, not a replacement checklist. The GP3 branch was verified with:

```bash
cargo +1.96.0 fmt --all -- --check
git diff --check
cargo +1.96.0 clippy --locked --workspace --all-targets -- -D warnings
cargo +1.96.0 test --locked --workspace --all-targets
cargo +1.96.0 check --locked -p bunny-num -p bunny-linalg -p bunny-geom \
  -p bunny-contract -p bunny-query -p bunny-broadphase -p bunny-mesh \
  -p bunny-codec --target wasm32-unknown-unknown
RUSTUP_TOOLCHAIN=1.96.0 wasm-pack test --node crates/bunny-codec --locked
```

Documentation changes also ran Markdown lint over the touched Markdown files.
The full local WASM loop used the explicit `RUSTUP_TOOLCHAIN=1.96.0
wasm-pack test --node <crate> --locked` commands listed in
`docs/TESTING.md#webassembly-gates` for all eight WASM-compatible library
crates: `bunny-num`, `bunny-linalg`, `bunny-geom`, `bunny-contract`,
`bunny-query`, `bunny-broadphase`, `bunny-mesh`, and `bunny-codec`.
