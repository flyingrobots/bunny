# Bunny - Bearing

This is a short repository signpost, not a branch, PR, or CI tracker. Deeper
history belongs in `ROADMAP.md`, `CHANGELOG.md`, and goalpost documents; live
work state belongs in GitHub issues and pull requests.

## Current Position

| Field | State |
| --- | --- |
| Release focus | `v0.5.0`: standards, numeric law, and release hygiene |
| Previous release baseline | `v0.4.0` |
| Current quality gate | Code Dojo, release archive verification, and GitHub Actions |
| Next feature train | `v0.6.0` Math Foundations |

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
* The Rust Code Standards Editor's Edition and Code Dojo enforcement layer are
  installed as the active repository standards and quality gate.
* PR #184 landed the numeric law cleanup: private `FixedQ32_32` raw storage,
  canonical `raw()` access, validating float ingress, `Hash`, and geometry
  out-of-Q32.32 rejection.
* The `v0.5.0` release candidate promotes the standards and numeric-law work into
  a publishable workspace version.

## Release Sequence

1. Merge the `v0.5.0` release-prep PR after review and CI are clean.
2. Tag the merge commit as `v0.5.0`.
3. Publish the GitHub Release so the release workflow can verify and publish the
   public crates.

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
snapshot, not a replacement checklist. The active local quality gate is:

```bash
cargo run --locked -p xtask -- code-dojo --all
```

The standards-alignment goalpost is complete. Evidence is recorded in
`docs/goalposts/post-v0.4.0-standards-alignment.md`; the release candidate also
passes `RELEASE_TAG=v0.5.0 scripts/publish-crates.sh verify`.
