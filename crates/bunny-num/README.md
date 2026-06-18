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
* **Deterministic Square Root**: Integer square root (`sqrt()`) implemented via
  digit-by-digit calculations on wide integers.
* **Zero Dependency & No-Unsafe**: Compiled under `#![deny(unsafe_code)]` with
  zero external runtime dependencies.

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
