# Exact Unsigned Ellipse Classification (`bunny-u32-ellipse-classify/1`)

This chapter describes the current implementation of `bunny-u32-ellipse-classify/1`
in `crates/bunny-num` and `crates/bunny-gpu`.

## Ownership & Scope

- **CPU reference (`crates/bunny-num`):** Pure, `#![deny(unsafe_code)]`, WASM-compatible
  `classify_ellipse_u32(dx, dy, semi_x, semi_y) -> Result<EllipseClass, EllipseDomainError>`,
  plus canonical adapters `classify_ellipse_u32_words(...) -> [u32; 2]` and
  `classify_ellipse_u32_bytes(...) -> [u8; 8]`.
- **GPU shader & equivalence harness (`crates/bunny-gpu`):** Quarantined backend crate
  holding the portable 2-limb `u32` WGSL helper
  (`crates/bunny-gpu/shaders/bunny_u32_ellipse_classify.wgsl`), the compute wrapper
  (`crates/bunny-gpu/shaders/bunny_u32_ellipse_classify_compute.wgsl`), the checked-in
  1,444-vector fixture (`crates/bunny-gpu/fixtures/u32_ellipse_classify_vectors.json`),
  the content-addressed `ImplementationManifest`, and the `wgpu` CPU-vs-GPU equivalence
  harness emitting `KernelConformanceReceipt`.
- **Downstream boundary:** Bunny owns only exact mathematical classification relative to an
  axis-aligned ellipse. Downstream consumers own rasterization, stroke thickness, annulus
  rules, and join masks.
- **Q32.32 non-equivalence:** This bounded unsigned-integer kernel is independent of
  `FixedQ32_32` and makes no claim of Q32.32 GPU conformance.

## Normative Contract

- **Domain:** `dx, dy, semi_x, semi_y` in `0..=46,340` (`floor(sqrt(2^31 - 1))`). Any operand
  above `46,340` returns `Err(EllipseDomainError)` / `[0, 1]`. Host callers MUST reject
  negative, fractional, non-finite, or out-of-range values before casting to `u32`.
- **Degenerate zero-semiaxis rule (precedes the raw polynomial):** When `semi_x == 0 ||
  semi_y == 0`, `(0, 0)` classifies as `OnBoundary` (`[1, 0]`) and every other point
  classifies as `Outside` (`[2, 0]`). Evaluating the raw polynomial without this branch would
  misclassify `(0, dy, 0, b)` and `(dx, 0, a, 0)` as `0 == 0` (`OnBoundary`).
- **Nonzero-semiaxis trichotomy:** Compares `(dx * semi_y)^2 + (dy * semi_x)^2` against
  `(semi_x * semi_y)^2` in exact unsigned 64-bit arithmetic (2-limb `u32` in portable WGSL),
  bounded strictly below `2^63` with zero division, zero rounding, and zero subtraction:
  - `lhs < rhs` -> `Inside` (`0`)
  - `lhs == rhs` -> `OnBoundary` (`1`)
  - `lhs > rhs` -> `Outside` (`2`)
- **Interchange wire format:** 8 little-endian bytes (`class_le4 || status_le4`), where
  `status == 0` is Success (`class` in `{0, 1, 2}`) and `status == 1` is `OutOfDomain` with
  `class == 0`.

## Conformance Evidence

The verification design and implemented test anchors live in [`test-plan.md`](test-plan.md).
The `wgpu` harness reports `Outcome::Unavailable` (never `Outcome::Pass`) when a host has
no compatible GPU or software adapter, and `Outcome::NotRun` when passed an empty vector slice.
