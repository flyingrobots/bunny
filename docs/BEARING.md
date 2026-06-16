# Bunny - Bearing

This is the living signpost for what the repo is doing now. It should be short,
current, and honest. Deeper history belongs in `ROADMAP.md`, `CHANGELOG.md`, and
goalpost documents.

## Current Position

| Field | State |
| --- | --- |
| Active release | `v0.4.0` - Quantized Meshes & Codecs |
| Active branch | `main` |
| Open PR | None |
| Current gate | `v0.4.0` release verification and publication |
| Next goalpost | Post-`v0.4.0` sync and `v0.5.0` planning |

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
* PR #105 merged GP3 into `main` in merge commit
  `2227825ab83d7aa53de3ec0f5d538d2542ac4cf3`.
* PR #104 merged GP2 into `main`. The GP2 goalpost now includes a captured
  witness table with repo-truth anchors for each completed implementation claim.
* Codec ingress now rejects non-finite vertex coordinates and out-of-bounds PLY
  face indices before returning borrowed mesh views.
* CI is pinned to Rust 1.96.0 and runs native workspace tests plus headless
  `wasm-pack test --node` for every WASM-compatible library crate.
* Release publication is now gated by `.github/workflows/release.yml`, which
  packages and publishes the public Bunny crates to crates.io in dependency
  order after the GitHub Release is published.

## Immediate Next Work

1. Verify the release archive gate and required local quality gates.
2. Push the release-prep commit to `main`.
3. Tag `v0.4.0`, publish the GitHub Release, and confirm the crates.io workflow
   publishes every public Bunny crate.
4. Start the next roadmap branch only after release publication is confirmed.

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
snapshot, not a replacement checklist. The `v0.4.0` release candidate must be
verified with:

```bash
cargo +1.96.0 fmt --all -- --check
git diff --check
cargo +1.96.0 clippy --locked --workspace --all-targets -- -D warnings
cargo +1.96.0 test --locked --workspace --all-targets
cargo +1.96.0 check --locked -p bunny-num -p bunny-linalg -p bunny-geom \
  -p bunny-contract -p bunny-query -p bunny-broadphase -p bunny-mesh \
  -p bunny-codec --target wasm32-unknown-unknown
RUSTUP_TOOLCHAIN=1.96.0 wasm-pack test --node crates/bunny-codec --locked
scripts/publish-crates.sh verify
```

Documentation changes also ran Markdown lint over the touched Markdown files.
The full local WASM loop used the explicit `RUSTUP_TOOLCHAIN=1.96.0
wasm-pack test --node <crate> --locked` commands listed in
`docs/TESTING.md#webassembly-gates` for all eight WASM-compatible library
crates: `bunny-num`, `bunny-linalg`, `bunny-geom`, `bunny-contract`,
`bunny-query`, `bunny-broadphase`, `bunny-mesh`, and `bunny-codec`.
