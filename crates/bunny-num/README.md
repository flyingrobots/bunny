# bunny-num

Deterministic numeric profiles and fixed-point math substrates for the Bunny
graphics commons.

This crate provides the core numerical representations used across the Bunny
ecosystem to guarantee bit-level CPU determinism.

## Core Features

* **FixedQ32_32**: A private-field newtype wrapping a signed 64-bit integer
  (`i64`) scaled by $2^{32}$.
* **Validated Float Ingress**: `try_from_f32` rejects non-finite and
  out-of-range float boundaries before they enter canonical fixed math.
* **Deterministic Float Conversions**: Bitwise extraction and parsing of IEEE
  754 float bits map floating-point values without relying on host-specific FPU
  registers.
* **Bankers' Rounding (Ties-to-Even)**: Standard, unbiased rounding implemented
  on division remainders and multiplication products to prevent systematic bias.
* **Saturating Bounds**: Arithmetic operations clamp to `i64::MIN` or
  `i64::MAX` instead of wrapping.
* **Checked Arithmetic**: `checked_add`, `checked_sub`, `checked_neg`,
  `checked_mul`, and `checked_div` return `None` when overflow or division by
  zero would make the result invalid for geometry code.
* **Deterministic Square Root**: Exact integer square root (`sqrt()`) on wide
  integers, with the same bits on every target (the implementation uses core's
  `isqrt`; the older digit-by-digit loop is kept as a test oracle).
* **Exact Lerp and Ratios (`bunny-lerp/1`)**: `FixedQ32_32::lerp(a, b, t)` is
  `a + round_half_even((b - a) * t / 2^32)` from an exact `i128` step, exact at
  `t = 0` and `t = 1.0`, monotone in `t`, extrapolating outside `[0, 1]`, and
  `None` only outside Q32.32; `FixedQ32_32::from_ratio(n, d)` is the nearest
  Q32.32 value to `n / d` for `u32` operands, `None` for `d = 0` or a result
  of `2^31` or more.
* **FixedAngle**: A binary angle (`2^64` units per turn) with wrapping
  arithmetic, exact or symmetric-rounding conversions from turns, degrees and
  radians, and deterministic `sin`, `cos` and `sin_cos` within one raw Q32.32
  unit of the exact value.
* **Zero Dependency & No-Unsafe**: Compiled under `#![deny(unsafe_code)]` with
  zero external runtime dependencies.
* **`no_std`**: With default features off the crate is `#![no_std]` and needs
  only `core` (no `alloc`), with every public item and the same bits.

## Features

| Feature | Default | Effect |
| --- | --- | --- |
| `std` | on | Links `std`. Off, the crate is `no_std`; no public item is removed. |

```toml
[dependencies]
bunny-num = { version = "0.7", default-features = false }
```

See [`docs/topics/no-std/`](../../docs/topics/no-std/README.md) for the gate
that builds it for `thumbv7em-none-eabihf`, a target with no `std`.

## Usage

```rust
use bunny_num::{FixedQ32_32, FloatConversionError};

fn main() -> Result<(), FloatConversionError> {
    let a = FixedQ32_32::try_from_f32(1.5)?;
    let b = FixedQ32_32::try_from_f32(2.5)?;

    let sum = a + b;
    let product = a * b;
    let quotient = b / a;

    assert_eq!(sum.to_f32(), 4.0);
    assert_eq!(product.to_f32(), 3.75);
    let root = FixedQ32_32::try_from_f32(9.0)?.sqrt();
    assert_eq!(root.map(FixedQ32_32::to_f32), Some(3.0));

    Ok(())
}
```

## License

Apache-2.0.
