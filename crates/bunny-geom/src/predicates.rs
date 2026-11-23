//! Exact geometric predicates (`bunny-predicates/1`).
//!
//! Q32.32 points are integers on the raw grid, so each predicate's determinant is an integer
//! and its sign is computed exactly, with no rounding, for every representable input. The
//! functions that round return a magnitude and are named `_rounded`; never decide a sign
//! from them. See `docs/topics/predicates/README.md`.

use core::cmp::Ordering;

use bunny_linalg::{FixedVec2, FixedVec3};
use bunny_num::FixedQ32_32;

mod barycentric;
mod wide;

pub use barycentric::{
    barycentric2, barycentric3_coplanar, ExactBarycentric, TriangleEdge, TriangleLocation,
    TriangleVertex,
};
use wide::Wide;

/// The identity string of the exact predicate contract.
pub const PREDICATES_CONTRACT_ID: &str = "bunny-predicates/1";

/// `2^33`: the doubled area in raw squared units over `2^33` is the area in raw units.
const AREA_DIVISOR: wide::Limbs = [1 << 33, 0, 0, 0];

/// `6 2^64`: the sextupled volume in raw cubed units over `6 2^64` is the volume in raw units.
const VOLUME_DIVISOR: wide::Limbs = [0, 6, 0, 0];

/// The winding of three points in Bunny's canonical 2D plane (`+X` right, `+Y` up), the sign
/// of `(b - a) x (c - a)`.
///
/// In a `y`-down frame, such as viewport pixels, the same sign looks clockwise on screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Orientation2 {
    /// The cross product is positive: `c` is left of the line from `a` to `b`.
    CounterClockwise,
    /// The cross product is negative: `c` is right of the line from `a` to `b`.
    Clockwise,
    /// The cross product is exactly zero: the points are on one line, or repeated.
    Collinear,
}

/// The side of the plane through `a`, `b` and `c` that a point `d` is on, the sign of
/// `(d - a) . ((b - a) x (c - a))`.
///
/// The front is the side the right-hand normal `(b - a) x (c - a)` points to: the side from
/// which `a`, `b` and `c` appear counter-clockwise in Bunny's right-handed frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Orientation3 {
    /// The determinant is positive: `d` is on the side the normal points to.
    Front,
    /// The determinant is negative: `d` is on the other side.
    Behind,
    /// The determinant is exactly zero: the four points are on one plane, or degenerate.
    Coplanar,
}

/// The exact integer value of a predicate's determinant: a sign and a magnitude below
/// `2^256`, in raw units (raw squared for a 2D area, raw cubed for a 3D volume).
///
/// It holds a doubled area (`|value| <= (2^64 - 1)^2`), a sextupled volume
/// (`|value| <= 2 (2^64 - 1)^3`) or a barycentric numerator or denominator. Zero is never
/// negative, so equal values compare equal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExactDeterminant(Wide);

impl ExactDeterminant {
    /// The sign: `Less` below zero, `Equal` at zero, `Greater` above.
    #[must_use]
    pub fn sign(&self) -> Ordering {
        if self.0.is_zero() {
            Ordering::Equal
        } else if self.0.is_negative() {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }

    /// Whether the value is exactly zero.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    /// Whether the value is below zero.
    #[must_use]
    pub const fn is_negative(&self) -> bool {
        self.0.is_negative()
    }

    /// The magnitude as four little-endian 64-bit limbs.
    #[must_use]
    pub const fn magnitude(&self) -> [u64; 4] {
        self.0.magnitude()
    }

    /// The value as an `i128`, or `None` when it does not fit.
    #[must_use]
    pub fn to_i128(&self) -> Option<i128> {
        let [low, high, rest @ ..] = self.0.magnitude();
        if rest != [0, 0] {
            return None;
        }
        let magnitude = (u128::from(high) << 64) | u128::from(low);
        if self.0.is_negative() {
            0_i128.checked_sub_unsigned(magnitude)
        } else {
            i128::try_from(magnitude).ok()
        }
    }
}

/// The exact `(b - a) x (c - a)` on raw values.
///
/// Each difference has magnitude at most `2^64 - 1` and each product below `2^128`; the
/// difference of the two products is held in 256 bits.
fn det2(a: FixedVec2, b: FixedVec2, c: FixedVec2) -> Wide {
    let ex = Wide::difference(b.x.raw(), a.x.raw());
    let ey = Wide::difference(b.y.raw(), a.y.raw());
    let fx = Wide::difference(c.x.raw(), a.x.raw());
    let fy = Wide::difference(c.y.raw(), a.y.raw());
    ex.times(fy).minus(ey.times(fx))
}

/// The exact `(d - a) . ((b - a) x (c - a))` on raw values, as the sum of six products of
/// three differences, each below `2^192`.
fn det3(a: FixedVec3, b: FixedVec3, c: FixedVec3, d: FixedVec3) -> Wide {
    let edge = |p: FixedVec3| {
        [
            Wide::difference(p.x.raw(), a.x.raw()),
            Wide::difference(p.y.raw(), a.y.raw()),
            Wide::difference(p.z.raw(), a.z.raw()),
        ]
    };
    let ([ex, ey, ez], [fx, fy, fz], [gx, gy, gz]) = (edge(b), edge(c), edge(d));
    let nx = ey.times(fz).minus(ez.times(fy));
    let ny = ez.times(fx).minus(ex.times(fz));
    let nz = ex.times(fy).minus(ey.times(fx));
    gx.times(nx).plus(gy.times(ny)).plus(gz.times(nz))
}

/// The exact winding of `a`, `b`, `c`: the sign of `(b - a) x (c - a)` on raw values.
///
/// Exact for every representable input, including `i64::MIN` and `i64::MAX` coordinates:
/// nothing is rounded before the sign is read. `Collinear` means exactly zero, so moving any
/// coordinate of three collinear points by one raw unit either gives a definite sign or
/// keeps them on one line.
#[must_use]
pub fn orientation2(a: FixedVec2, b: FixedVec2, c: FixedVec2) -> Orientation2 {
    match doubled_signed_area2(a, b, c).sign() {
        Ordering::Greater => Orientation2::CounterClockwise,
        Ordering::Less => Orientation2::Clockwise,
        Ordering::Equal => Orientation2::Collinear,
    }
}

/// The exact doubled signed area `(b - a) x (c - a)` of the triangle `a`, `b`, `c`, in raw
/// squared units (`2^-64` square units). Positive is counter-clockwise.
///
/// Its magnitude is at most `(2^64 - 1)^2`, reached by a corner triangle of the raw square: a
/// triangle inside a square covers at most half of it. It does not fit Q32.32 in general.
#[must_use]
pub fn doubled_signed_area2(a: FixedVec2, b: FixedVec2, c: FixedVec2) -> ExactDeterminant {
    ExactDeterminant(det2(a, b, c))
}

/// The signed area of `a`, `b`, `c` in Q32.32, rounded once from the exact value: raw
/// `round_half_even(D / 2^33)` with `D` the exact doubled area. `None` when that is outside
/// `i64`, which needs an area of `2^31` square units or more.
///
/// This is a magnitude, not a predicate. Its sign agrees with [`orientation2`] except that a
/// nonzero area of at most half a raw unit (`|D| <= 2^32`) rounds to zero; decide winding
/// with [`orientation2`].
#[must_use]
pub fn signed_area2_rounded(a: FixedVec2, b: FixedVec2, c: FixedVec2) -> Option<FixedQ32_32> {
    det2(a, b, c).round_div(&AREA_DIVISOR).map(FixedQ32_32::from_raw)
}

/// The exact side of the plane through `a`, `b`, `c` that `d` is on: the sign of
/// `(d - a) . ((b - a) x (c - a))` on raw values.
///
/// Exact for every representable input, including `i64::MIN` and `i64::MAX` coordinates:
/// nothing is rounded before the sign is read. `Coplanar` means exactly zero; it is also the
/// answer when `a`, `b`, `c` are collinear or repeated.
#[must_use]
pub fn orientation3(a: FixedVec3, b: FixedVec3, c: FixedVec3, d: FixedVec3) -> Orientation3 {
    match sextupled_signed_volume3(a, b, c, d).sign() {
        Ordering::Greater => Orientation3::Front,
        Ordering::Less => Orientation3::Behind,
        Ordering::Equal => Orientation3::Coplanar,
    }
}

/// The exact sextupled signed volume `(d - a) . ((b - a) x (c - a))` of the tetrahedron
/// `a`, `b`, `c`, `d`, in raw cubed units (`2^-96` cubic units). Positive is
/// [`Orientation3::Front`].
///
/// Its magnitude is at most `2 (2^64 - 1)^3`, reached by the regular tetrahedron on alternate
/// corners of the raw cube (a tetrahedron in a cube has at most a third of its volume).
#[must_use]
pub fn sextupled_signed_volume3(
    a: FixedVec3,
    b: FixedVec3,
    c: FixedVec3,
    d: FixedVec3,
) -> ExactDeterminant {
    ExactDeterminant(det3(a, b, c, d))
}

/// The signed volume of `a`, `b`, `c`, `d` in Q32.32, rounded once from the exact value: raw
/// `round_half_even(D / (6 2^64))` with `D` the exact sextupled volume. `None` when that is
/// outside `i64`.
///
/// This is a magnitude, not a predicate. Its sign agrees with [`orientation3`] except that a
/// nonzero volume of at most half a raw unit (`|D| <= 3 2^64`) rounds to zero; decide the
/// side with [`orientation3`].
#[must_use]
pub fn signed_volume3_rounded(
    a: FixedVec3,
    b: FixedVec3,
    c: FixedVec3,
    d: FixedVec3,
) -> Option<FixedQ32_32> {
    det3(a, b, c, d).round_div(&VOLUME_DIVISOR).map(FixedQ32_32::from_raw)
}
