# Numeric Constitution — Deterministic Q32.32 Math

This document defines the arithmetic law for Bunny's canonical fixed-point math.
When this document and implementation disagree, one of them is wrong. Usually both need tests.

`docs/COORDINATE_LAW.md` defines what canonical numbers mean when they are used
as coordinates, vectors, directions, normals, transforms, projections, or
viewport values.

## Canonical Type

Use a private newtype:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FixedQ32_32(i64);
```

Required associated constants:

```rust
impl FixedQ32_32 {
    pub const FRACTIONAL_BITS: u32 = 32;
    pub const SCALE: i128 = 1_i128 << Self::FRACTIONAL_BITS;
    pub const MIN_RAW: i64 = i64::MIN;
    pub const MAX_RAW: i64 = i64::MAX;

    pub const fn from_raw(raw: i64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> i64 {
        self.0
    }
}
```

Do not expose the field publicly. A compatibility accessor such as `to_raw` may
exist, but `raw` is the canonical name in this document.

## Raw Representation

- Signed two's-complement `i64` raw value.
- Lower 32 bits are fractional.
- Upper signed bits represent the integer component.
- Serialization writes the raw `i64` in the project-selected byte order.
- Textual debug formatting is not canonical serialization.

## Construction Policy

Construction must be explicit:

- `from_raw(i64)` preserves exact raw bits.
- `from_i32`, `from_i64`, or similar integer constructors must check or document range.
- Validated float ingress is a boundary operation that must reject `NaN`, `+∞`,
  `-∞`, and values outside representable range.
- Float ingress must document rounding mode and must not be used inside core canonical algorithms.
- Saturating float conversion helpers may exist only when their names or docs
  make the saturation policy explicit. They must not be used where API
  validation claims rejection.

## Rounding Policy

Default rounding is **Banker's Rounding / ties-to-even**.

At a discarded-bit boundary:

1. If the discarded value is less than half, round toward zero-magnitude truncation result.
2. If the discarded value is greater than half, round away from the truncation result.
3. If exactly half, choose the result whose least significant retained bit is even.
4. Negative values use the same mathematical ties-to-even rule, not ad hoc sign hacks.

Required tests:

- positive less-than-half
- positive greater-than-half
- positive exact-half to even
- positive exact-half to odd
- negative less-than-half
- negative greater-than-half
- negative exact-half to even
- negative exact-half to odd

## Intermediate Width

All multiplication and division implementations must use `i128` intermediates unless a local proof demonstrates no overflow before the final policy step.

## Overflow Policy

Every arithmetic operation must declare one of these policies:

### Checked

Returns an error or `Option::None` when the exact mathematical result is not representable.

Use for:

- geometry predicate inputs
- construction from external data
- transformations where overflow invalidates the result

### Saturating

Clamps to `MIN_RAW` or `MAX_RAW`.

Use only when the API explicitly defines saturation as meaningful. Saturation is deterministic, but it can turn invalid geometry into plausible garbage.

### Proven-Infallible

The function documents a local invariant proving overflow cannot occur.

Use for:

- bounded loops
- normalized values
- operations on prevalidated ranges

Proof must be close to the code. Distant tribal knowledge is not a proof.

## Division Policy

Division by zero is domain-invalid. It must not panic.

Allowed outcomes:

- `Result<T, NumericError::DivisionByZero>`
- `Option::None`
- documented invalid construction rejection

Saturating division by zero is banned in core geometry unless an algorithm-specific document proves it is the canonical rule.

## Multiplication Policy

Multiplication algorithm:

1. Promote operands to `i128`.
2. Multiply exact raw values.
3. Round from Q64.64 back to Q32.32 using ties-to-even.
4. Apply the declared overflow policy.

The shift and rounding order is part of the API contract. Do not "optimize" it into a different result.

## Equality and Ordering

Equality and ordering compare raw canonical values.

There is no epsilon equality in canonical fixed-point math. Epsilon belongs in lossy boundary adapters or visualization, not in the core truth model.

## Deterministic Test Vectors

Every numeric crate should contain a golden-vector test file covering:

- raw round trips
- integer construction boundaries
- float ingress rejection cases, if float ingress exists
- addition boundaries
- subtraction boundaries
- multiplication rounding
- division rounding
- negative ties
- min/max saturation or checked overflow
- canonical serialization

Recommended location:

```text
crates/<crate>/tests/golden_vectors.rs
```

## Reference Implementation Requirement

Optimized arithmetic must have a simple reference implementation or externally generated golden vectors.

The optimized implementation may be clever. The reference must be boring.

**Sensei's Wisdom™**: One clever implementation is a magic trick. Two equivalent implementations plus golden vectors is engineering.
