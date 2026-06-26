# Bunny - Bearing

This is a short repository signpost, not a branch, PR, or CI tracker. Deeper
history belongs in `ROADMAP.md`, `CHANGELOG.md`, and goalpost documents; live
work state belongs in GitHub issues and pull requests.

## Current Position

| Field | State |
| --- | --- |
| Release focus | `v0.7.0`: Geometry Law and remaining frame math |
| Previous release baseline | `v0.6.0`, cut from the 2026-06-26 release prep |
| Current quality gate | Code Dojo, release archive verification, and GitHub Actions |
| Next release step | Start the `v0.7.0` goalposts and keep moved frame-math slices explicit |
| Next feature train | `v0.7.0` Geometry Law and Primitive Coverage |

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
* PR #185 merged the `v0.5.0` release prep into `main` in merge commit
  `5c15363b6e6da609df76bd37db8bc3b41215ff05`.
* `v0.5.0` promotes the standards and numeric-law work into a publishable
  workspace version.
* `v0.5.0` was tagged and published through the GitHub Release workflow on
  2026-06-19.
* PR #189 merged the first `v0.6.0` Math Foundations goalpost into `main`,
  covering coordinate law, numeric preconditions, and deterministic property
  corpora.
* PR #191 merged deterministic matrix and affine transform primitives into
  `main`.
* PR #195 merged deterministic contract-profile witnesses for `bunny-wesley`
  and `bunny-contract`.
* `v0.6.0` publishes the landed frame-commons surface: coordinate law, numeric
  preconditions, property corpora, matrices, affine transforms, and deterministic
  contract-profile witnesses. Projection, quaternion, angle, interpolation,
  curve, and transform-aware bounds work moved forward to the next train.

## Release Sequence

1. Complete the `v0.7.0` remaining frame-math and geometry-law goalposts.
2. Open a release-prep pull request that bumps the workspace version to
   `0.7.0`, updates release notes and signposts, and passes Code Dojo plus
   release archive verification.
3. Merge the verified release-prep pull request to `main`.
4. Tag the verified `main` tip as `v0.7.0`.
5. Publish the GitHub Release so the release workflow can verify and publish the
   public crates.
6. Confirm crates.io visibility for every published Bunny crate.

## Watchpoints

* Do not mark a roadmap slice done unless the implementation, tests, and docs
  all support the claim.
* Keep host-side tooling (`bunny-wesley`, `xtask`) distinct from
  WASM-compatible library crates in docs and CI claims.
* Keep GP2 codec parser zero-copy claims intact. GP3 decoder claims are limited
  to borrowed raw payload sections plus typed checked accessors; no typed slice
  reinterpretation is claimed.
* Quaternion, projection, viewport, interpolation, curve, and transform-aware
  bounds APIs are still absent; future frame-math work must either add them or
  explicitly stay out of that scope.

## Last Known Local Verification

The canonical checklist lives in `docs/TESTING.md`; this section is a status
snapshot, not a replacement checklist. The active local quality gate is:

```bash
cargo run --locked -p xtask -- code-dojo --all
```

The standards-alignment goalpost is complete. Evidence is recorded in
`docs/goalposts/post-v0.4.0-standards-alignment.md`; the `v0.6.0` release prep
must pass `RELEASE_TAG=v0.6.0 scripts/publish-crates.sh verify`, full Code Dojo,
GitHub Actions, and CodeRabbit review before tagging.
