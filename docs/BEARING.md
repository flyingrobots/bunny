# Bunny - Bearing

This is the living signpost for what the repo is doing now. It should be short,
current, and honest. Deeper history belongs in `ROADMAP.md`, `CHANGELOG.md`, and
goalpost documents.

## Current Position

| Field | State |
| --- | --- |
| Active release | Post-`v0.4.0` standards alignment |
| Active branch | `backlog-source-of-truth-guard` |
| Open PR | #106 |
| Current gate | Standards alignment complete locally |
| Next goalpost | Prepare the standards alignment branch for review |

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
* The local standards-alignment gate is clean: Code Dojo, headless Node.js WASM
  tests for all WASM-compatible library crates, and release archive verification
  all pass.

## Immediate Next Work

1. Commit the local standards-alignment work.
2. Reconcile the existing branch PR scope or open a dedicated PR from a clean
   standards-alignment branch.
3. Run CI and resolve any review findings without weakening the standards gate.

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
python3 scripts/code-dojo/dojo.py --all
```

The current standards-alignment goalpost is complete locally. Evidence is
recorded in `docs/goalposts/post-v0.4.0-standards-alignment.md`.
