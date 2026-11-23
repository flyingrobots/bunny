# bunny-linalg

Deterministic linear algebra primitives for the Bunny graphics commons.

This crate provides 2D and 3D vector representations plus fixed matrix and
affine transform types designed to enforce bit-level deterministic coordinate
math for graphics pipelines.

## Core Features

* **Float Vectors (`Vec2`, `Vec3`)**: Stateless floating-point coordinates
  suitable for network/wire DTO formats.
* **Fixed-Point Vectors (`FixedVec2`, `FixedVec3`)**: Deterministic vector
  primitives backed by `FixedQ32_32` fixed-point coordinates.
* **Unit Vectors (`FixedUnitVec2`, `FixedUnitVec3`)**: Runtime normalization
  through `new`, plus compile-time fixed-unit proofs through `try_from_unit`
  and axis constants such as `UNIT_X` and `NEG_UNIT_Z`.
* **Fixed-Point Matrices (`FixedMat2`, `FixedMat3`, `FixedMat4`)**:
  Row-major deterministic matrix primitives with column-vector multiplication
  semantics.
* **Affine Transforms (`FixedAffine2`, `FixedAffine3`)**: Checked transform
  wrappers that translate points, preserve vector translation-invariance, and
  compose right-to-left.
* **Projection Matrices**: `FixedMat4::try_orthographic` and
  `try_perspective` build `clip_from_view` matrices (NDC depth `[0, 1]`, camera
  looks down `-Z`) from a tangent, with no trigonometry and typed
  `ProjectionError`s.
* **Viewport Mapping**: `Viewport::try_ndc_to_viewport` and
  `try_viewport_to_ndc` convert between NDC and continuous top-left, y-down
  viewport coordinates with floored raw rounding and typed `ViewportError`s.
* **Look-At View Matrix**: `FixedMat4::try_look_at(eye, target, up)` builds a
  right-handed `view_from_world` matrix (camera looks down `-Z`). The basis is
  orthonormal within 2 raw units; degenerate input, including an `up` that is
  exactly parallel or antiparallel to the view direction, returns a typed
  `LookAtError` (`ParallelUp`). A translation entry whose finished value is
  representable is never refused because an intermediate term overflowed.
* **Look Rotation**: `FixedMat3::try_look_rotation(forward, up)` builds the
  rotation that maps local `-Z` onto `forward` (columns right, true up, back),
  with the look-at basis, rounding and `LookAtError` refusals. Columns are
  orthonormal within 2 raw units, rows within 3 (length) and 4 (dot).
* **Rotations (`bunny-rotation/1`)**: `FixedMat3::rotation_x`, `rotation_y`
  and `rotation_z` copy `FixedAngle::sin_cos` into the axis matrices (exact at
  quarter turns), and `rotation_axis_angle(axis, angle)` evaluates Rodrigues'
  formula on a `FixedUnitVec3` axis with one ties-to-even rounding per entry.
  Right-hand rule, column vectors; bounds in `docs/topics/rotation/`.
* **Transform Composition (`bunny-transform/1`)**:
  `FixedAffine3::from_trs(translation, rotation, scale)` is `T * R * S` (scale,
  then rotate, then translate), and `rotate_about_point` and
  `scale_about_point` are `T(p) * R * T(-p)` and `T(p) * S * T(-p)`, the pivot
  fixed exactly. One ties-to-even rounding per product; `None` on overflow;
  bounds in `docs/topics/transform/`.
* **Geometric Operations**: Native dot products, cross products (for
  `FixedVec3`), squared lengths, lengths, and normalization.
* **Exact Lerp (`bunny-lerp/1`)**: `FixedVec3::lerp(a, b, t)` and
  `lerp_channels(a, b, t)` for `[FixedQ32_32; 3]` colour channels apply
  `FixedQ32_32::lerp` to each component; `None` when any component is out of
  range. No colour space is assumed.
* **Cubic Bezier Curves (`bunny-curve/1`)**: `CubicBezier3` with `at(t)` by
  de Casteljau over `FixedVec3::lerp` (within 3/2 raw units of the exact
  cubic, exact at the ends), `derivative_at(t)` (within 3 raw units, `None` on
  overflow) and `split(t)` into the de Casteljau halves; `None` for `t`
  outside `[0, 1]`.
* **Arc Length (`bunny-curve/1`, part 3)**: `ArcLengthTable::build(curve,
  segments_per_span, storage)` over any `Curve3` (implemented by
  `CubicBezier3`) stores cumulative chord lengths in caller storage, each chord
  kept to `2^-16` raw and each entry rounded once; `param_at_distance(s)` finds
  the parameter by binary search and one rounded interpolation, exact at 0 and
  at `total()`, non-decreasing, `None` outside `[0, total]`. Typed refusals in
  `ArcLengthError`; the bounds and measured errors are in
  `docs/topics/curve/README.md#arc-length`.
* **Checked Matrix Operations**: Matrix-vector and matrix-matrix
  multiplication, determinant, and inverse APIs return `None` for singular or
  overflowing cases; transpose is infallible and returns a matrix directly.
* **Arithmetic Operator Overloads**: Complete suite of standard vector
  operations (`Add`, `Sub`, `Neg`, scalar `Mul` / `Div`, and assign variants).
* **Boundary Conversions**: `try_from_float` validates float DTO coordinates
  before fixed-point ingress; `From` remains available only as a saturating
  convenience conversion.
* **Safe & Portable**: Declares `#![deny(unsafe_code)]` and compiles cleanly on
  all platforms including WebAssembly (`wasm32-unknown-unknown`).

## Usage

```rust
use bunny_linalg::{FixedMat3, FixedVec3, Vec3};
use bunny_num::FloatConversionError;

fn main() -> Result<(), FloatConversionError> {
    let a = FixedVec3::try_from_float(Vec3::new(1.0, 2.0, 3.0))?;
    let b = FixedVec3::try_from_float(Vec3::new(4.0, 5.0, 6.0))?;

    // Vector operations
    let sum = a + b;
    let dot = a.dot(b);
    let cross = a.cross(b); // Returns [-3.0, 6.0, -3.0]
    let identity = FixedMat3::IDENTITY;
    let transformed = identity.checked_mul_vec3(a);

    // Length and normalization
    let len = a.length();
    let norm = a.normalize();

    assert_eq!(sum.x.to_f32(), 5.0);
    assert_eq!(dot.to_f32(), 32.0);
    assert_eq!(cross.x.to_f32(), -3.0);
    assert_eq!(transformed, Some(a));
    assert!(len.is_some());
    assert!(norm.is_some());

    Ok(())
}
```

## License

Apache-2.0.
