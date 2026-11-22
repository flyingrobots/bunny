# Unsigned 32-Bit Ties-to-Even Division (`bunny-u32-div-round-even/1`)

This chapter describes the current implementation of `bunny-u32-div-round-even/1`
in `crates/bunny-num` and `crates/bunny-gpu`.

## Ownership & Scope

- **CPU reference (`crates/bunny-num`):** Pure, `#![deny(unsafe_code)]`, WASM-compatible
  `div_round_even_u32(n, d) -> Result<u32, DivisionByZero>`, plus canonical adapters
  `div_round_even_u32_words(n, d) -> [u32; 2]` and `div_round_even_u32_bytes(n, d) -> [u8; 8]`.
- **GPU shader & equivalence harness (`crates/bunny-gpu`):** Quarantined backend crate
  holding the portable WGSL helper (`crates/bunny-gpu/shaders/bunny_u32_div_round_even.wgsl`),
  the compute wrapper (`crates/bunny-gpu/shaders/bunny_u32_div_round_even_compute.wgsl`),
  the checked-in 1,720-vector fixture (`crates/bunny-gpu/fixtures/u32_div_round_even_vectors.json`),
  the content-addressed `ImplementationManifest`, and the `wgpu` CPU-vs-GPU equivalence harness
  emitting `KernelConformanceReceipt`.
- **Q32.32 non-equivalence:** This bounded unsigned-integer kernel is independent of
  `FixedQ32_32` and makes no claim of Q32.32 GPU conformance.

## Normative Contract

- **Domain:** `n, d` in `0..=4,294,967,295` (`u32`). Host callers MUST reject negative,
  fractional, non-finite, or out-of-range values before casting to `u32`.
- **Zero divisor:** `d == 0` returns `Err(DivisionByZero)` / `[0, 1]` (including `0 / 0`),
  checked by a control-flow branch before `/` or `%`.
- **Rounding:** Exact rational `n / d` rounded to nearest `u32`, exact halfway ties to the
  even integer, computed without `2 * r` overflow via `s = d - r` and
  `(r > s) || ((r == s) && ((q & 1) != 0))`.
- **Interchange wire format:** 8 little-endian bytes (`value_le4 || status_le4`), where
  `status == 0` is Success and `status == 1` is `DivisionByZero` with `value == 0`.

## Conformance Evidence

The verification design and implemented test anchors live in [`test-plan.md`](test-plan.md).
The `wgpu` harness reports `Outcome::Unavailable` (never `Outcome::Pass`) when a host has
no compatible GPU or software adapter.
