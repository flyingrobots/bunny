# bunny-num

Deterministic numeric profiles and fixed-point math substrates for the Bunny graphics commons.

This crate provides the core numerical representations used across the Bunny ecosystem to guarantee bit-level CPU determinism.

## Core Features

* **FixedQ32_32**: A type-safe newtype wrapper wrapping a signed 64-bit integer (`i64`) scaled by $2^{32}$. It ensures that arithmetic calculations are identical across all target platforms (x86_64, ARM64, WASM), compilers, and optimization levels.
* **Deterministic Float Conversions**: Bitwise extraction and parsing of IEEE 754 float bits to map floating-point values to and from fixed-point representation without relying on host-specific FPU registers.
* **Bankers' Rounding (Ties-to-Even)**: Standard, unbiased rounding implemented on division remainders and multiplication products to prevent systematic bias accumulation.
* **Saturating Bounds**: Gracefully clamps values to `i64::MIN` and `i64::MAX` rather than wrapping around during arithmetic overflows.
* **Deterministic Square Root**: Fast integer square root algorithm (`sqrt()`) implemented via digit-by-digit calculations on wide integers.
* **Zero Dependency & No-Unsafe**: Compiled under `#![deny(unsafe_code)]` with zero external runtime dependencies.

## Usage

```rust
use bunny_num::FixedQ32_32;

fn main() {
    let a = FixedQ32_32::from_f32(1.5);
    let b = FixedQ32_32::from_f32(2.5);

    // Deterministic arithmetic operations
    let sum = a + b;
    let product = a * b;
    let quotient = b / a;

    assert_eq!(sum.to_f32(), 4.0);
    assert_eq!(product.to_f32(), 3.75);
    
    // Deterministic square root
    let root = FixedQ32_32::from_f32(9.0).sqrt().unwrap();
    assert_eq!(root.to_f32(), 3.0);
}
```

## License

Apache-2.0.
