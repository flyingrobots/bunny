# bunny-linalg

Deterministic linear algebra primitives for the Bunny graphics commons.

This crate provides 2D and 3D vector representations designed to enforce
bit-level deterministic coordinate math for graphics pipelines.

## Core Features

* **Float Vectors (`Vec2`, `Vec3`)**: Stateless floating-point coordinates
  suitable for network/wire DTO formats.
* **Fixed-Point Vectors (`FixedVec2`, `FixedVec3`)**: Deterministic vector
  primitives backed by `FixedQ32_32` fixed-point coordinates.
* **Unit Vectors (`FixedUnitVec2`, `FixedUnitVec3`)**: Runtime normalization
  through `new`, plus compile-time fixed-unit proofs through `try_from_unit`
  and axis constants such as `UNIT_X` and `NEG_UNIT_Z`.
* **Geometric Operations**: Native dot products, cross products (for
  `FixedVec3`), squared lengths, lengths, and normalization.
* **Arithmetic Operator Overloads**: Complete suite of standard vector
  operations (`Add`, `Sub`, `Neg`, scalar `Mul` / `Div`, and assign variants).
* **Boundary Conversions**: `try_from_float` validates float DTO coordinates
  before fixed-point ingress; `From` remains available only as a saturating
  convenience conversion.
* **Safe & Portable**: Declares `#![deny(unsafe_code)]` and compiles cleanly on
  all platforms including WebAssembly (`wasm32-unknown-unknown`).

## Usage

```rust
use bunny_linalg::{FixedVec3, Vec3};
use bunny_num::FloatConversionError;

fn main() -> Result<(), FloatConversionError> {
    let a = FixedVec3::try_from_float(Vec3::new(1.0, 2.0, 3.0))?;
    let b = FixedVec3::try_from_float(Vec3::new(4.0, 5.0, 6.0))?;

    // Vector operations
    let sum = a + b;
    let dot = a.dot(b);
    let cross = a.cross(b); // Returns [-3.0, 6.0, -3.0]

    // Length and normalization
    let len = a.length();
    let norm = a.normalize();

    assert_eq!(sum.x.to_f32(), 5.0);
    assert_eq!(dot.to_f32(), 32.0);
    assert_eq!(cross.x.to_f32(), -3.0);
    assert!(len.is_some());
    assert!(norm.is_some());

    Ok(())
}
```

## License

Apache-2.0.
