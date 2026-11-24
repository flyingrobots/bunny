//! Box helpers on [`FixedAabb3`], contract `bunny-bounds/1`.
//!
//! Every helper works on raw `i64` coordinates. Minima, maxima and comparisons are exact;
//! margins and radii are added in `i128`, so nothing wraps or saturates, and a result outside
//! the raw range is refused. The one rounding is the bounding sphere's: its centre is rounded
//! to nearest, ties to even, and its radius is rounded up so the sphere contains the box. A
//! box whose minimum is above its maximum on some axis is refused as input, as
//! [`FixedAabb3::try_new`] refuses it. See `docs/topics/bounding-volumes/README.md`.

use core::fmt;

use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;

use crate::{FixedAabb3, FixedSphere3};

/// The contract identity of the box helpers on [`FixedAabb3`].
pub const BOUNDS_CONTRACT_ID: &str = "bunny-bounds/1";

/// Why a box helper refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundsError {
    /// [`FixedAabb3::from_points`] was given no points; an empty set has no bounds.
    NoPoints,
    /// An input box has a minimum above its maximum on some axis.
    InvertedBox,
    /// A negative margin would move a minimum above its maximum.
    ShrinkInvertsBox,
    /// The sphere radius is negative.
    NegativeRadius,
    /// A result coordinate or radius is outside the Q32.32 raw range (`i64`).
    OutOfRange,
}

impl fmt::Display for BoundsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NoPoints => "no points to bound",
            Self::InvertedBox => "box minimum is above its maximum",
            Self::ShrinkInvertsBox => "margin would move a box minimum above its maximum",
            Self::NegativeRadius => "sphere radius is negative",
            Self::OutOfRange => "bounds are outside the Q32.32 range",
        })
    }
}

impl core::error::Error for BoundsError {}

type Raw3 = [i64; 3];

const fn raw3(v: FixedVec3) -> Raw3 {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

const fn vec3(raw: Raw3) -> FixedVec3 {
    let [x, y, z] = raw;
    FixedVec3::new(FixedQ32_32::from_raw(x), FixedQ32_32::from_raw(y), FixedQ32_32::from_raw(z))
}

/// Per-axis `f` of two raw triples.
fn zip3(a: Raw3, b: Raw3, f: impl Fn(i64, i64) -> i64) -> Raw3 {
    let ([a0, a1, a2], [b0, b1, b2]) = (a, b);
    [f(a0, b0), f(a1, b1), f(a2, b2)]
}

/// Per-axis `f` of two raw triples, widened to `i128` first.
fn zip3_wide(a: Raw3, b: Raw3, f: impl Fn(i128, i128) -> i128) -> [i128; 3] {
    let ([a0, a1, a2], [b0, b1, b2]) = (a, b);
    let g = |x: i64, y: i64| f(i128::from(x), i128::from(y));
    [g(a0, b0), g(a1, b1), g(a2, b2)]
}

/// Each `i128` coordinate as `i64`, or `None` when one is outside the raw range.
fn narrow3(wide: [i128; 3]) -> Option<Raw3> {
    let [x, y, z] = wide;
    Some([i64::try_from(x).ok()?, i64::try_from(y).ok()?, i64::try_from(z).ok()?])
}

/// The midpoint of `lo` and `hi`, rounded to nearest, ties to even. It lies in `[lo, hi]`.
fn midpoint(lo: i64, hi: i64) -> i64 {
    let sum = i128::from(lo) + i128::from(hi);
    let floor = sum.div_euclid(2);
    // An odd sum is an exact half: step up from an odd floor to the even neighbour.
    let mid = floor + (sum & floor & 1);
    // `lo <= mid <= hi`, so it fits.
    i64::try_from(mid).unwrap_or(lo)
}

impl FixedAabb3 {
    /// Whether no minimum is above its maximum (the condition [`FixedAabb3::try_new`] checks).
    const fn is_ordered(&self) -> bool {
        let ([lx, ly, lz], [hx, hy, hz]) = (raw3(self.min), raw3(self.max));
        lx <= hx && ly <= hy && lz <= hz
    }

    /// The box grown to include `p`. Exact.
    fn include(self, p: FixedVec3) -> Self {
        let (lo, hi, p) = (raw3(self.min), raw3(self.max), raw3(p));
        Self::new(vec3(zip3(lo, p, i64::min)), vec3(zip3(hi, p, i64::max)))
    }

    /// The smallest box containing every point: each axis's exact minimum and maximum.
    ///
    /// One point gives the degenerate box at that point. The slice is read once; nothing
    /// allocates. The result is always ordered.
    ///
    /// # Errors
    ///
    /// [`BoundsError::NoPoints`] for an empty slice: an empty set has no bounds, and no box
    /// (zeros or an inverted sentinel) stands in for it.
    pub fn from_points(points: &[FixedVec3]) -> Result<Self, BoundsError> {
        let (first, rest) = points.split_first().ok_or(BoundsError::NoPoints)?;
        Ok(rest.iter().fold(Self::new(*first, *first), |b, p| b.include(*p)))
    }

    /// The smallest box containing the triangle: each axis's exact minimum and maximum of
    /// the three vertices. Every input is accepted, degenerate triangles included.
    #[must_use]
    pub fn from_triangle(vertices: [FixedVec3; 3]) -> Self {
        let [a, b, c] = vertices;
        Self::new(a, a).include(b).include(c)
    }

    /// The smallest box containing both boxes: each axis's exact minimum of the minima and
    /// maximum of the maxima.
    ///
    /// # Errors
    ///
    /// [`BoundsError::InvertedBox`] when either box has a minimum above its maximum.
    pub fn merge(&self, other: &Self) -> Result<Self, BoundsError> {
        if !self.is_ordered() || !other.is_ordered() {
            return Err(BoundsError::InvertedBox);
        }
        Ok(self.include(other.min).include(other.max))
    }

    /// The box with every face moved out by `margin` (in by a negative one): `min - margin`
    /// and `max + margin` on each axis. See [`FixedAabb3::expand_axes`].
    ///
    /// # Errors
    ///
    /// As [`FixedAabb3::expand_axes`].
    pub fn expand(&self, margin: FixedQ32_32) -> Result<Self, BoundsError> {
        self.expand_axes(FixedVec3::new(margin, margin, margin))
    }

    /// The box with each axis's faces moved out by that axis's margin: `min - m` and
    /// `max + m`, exact in `i128`. A zero margin returns the box unchanged; a negative one
    /// shrinks it, down to a flat box (minimum equal to maximum) but not past it.
    ///
    /// # Errors
    ///
    /// Checked in this order: [`BoundsError::InvertedBox`] when the input has a minimum above
    /// its maximum; [`BoundsError::ShrinkInvertsBox`] when a result minimum would be above its
    /// maximum; [`BoundsError::OutOfRange`] when a result coordinate is outside `i64`. An
    /// axis that shrinks without inverting stays inside the input, so only a growing axis can
    /// leave the range.
    pub fn expand_axes(&self, margin: FixedVec3) -> Result<Self, BoundsError> {
        if !self.is_ordered() {
            return Err(BoundsError::InvertedBox);
        }
        let (lo, hi, m) = (raw3(self.min), raw3(self.max), raw3(margin));
        let new_lo = zip3_wide(lo, m, |c, m| c - m);
        let new_hi = zip3_wide(hi, m, |c, m| c + m);
        if new_lo.iter().zip(&new_hi).any(|(l, h)| l > h) {
            return Err(BoundsError::ShrinkInvertsBox);
        }
        match (narrow3(new_lo), narrow3(new_hi)) {
            (Some(lo), Some(hi)) => Ok(Self::new(vec3(lo), vec3(hi))),
            _ => Err(BoundsError::OutOfRange),
        }
    }

    /// The smallest box containing the sphere: `centre - r` to `centre + r` on each axis,
    /// exact. A radius of zero gives the degenerate box at the centre.
    ///
    /// # Errors
    ///
    /// [`BoundsError::NegativeRadius`] for a negative radius (which
    /// [`FixedSphere3::try_new`] refuses); [`BoundsError::OutOfRange`] when a coordinate is
    /// outside `i64`.
    pub fn from_sphere(sphere: &FixedSphere3) -> Result<Self, BoundsError> {
        if sphere.radius < FixedQ32_32::ZERO {
            return Err(BoundsError::NegativeRadius);
        }
        // A point box grown by a non-negative margin is never inverted.
        Self::new(sphere.center, sphere.center).expand(sphere.radius)
    }

    /// A sphere containing the box, every corner included exactly.
    ///
    /// The centre is each axis's midpoint `(min + max) / 2`, rounded once to nearest, ties to
    /// even (within 1/2 raw of the true midpoint). The radius is the smallest raw value whose
    /// square is at least the exact squared distance from that centre to the farthest
    /// corner: the ceiling of the exact root, never rounded down, so no corner is outside.
    /// Each per-axis distance is at most `2^63`, so the squared distance is below `3 * 2^126`
    /// and fits `u128`.
    ///
    /// # Errors
    ///
    /// [`BoundsError::InvertedBox`] when the box has a minimum above its maximum;
    /// [`BoundsError::OutOfRange`] when the radius is above `i64::MAX` raw.
    pub fn bounding_sphere(&self) -> Result<FixedSphere3, BoundsError> {
        if !self.is_ordered() {
            return Err(BoundsError::InvertedBox);
        }
        let (lo, hi) = (raw3(self.min), raw3(self.max));
        let centre = zip3(lo, hi, midpoint);
        let below = zip3_wide(centre, lo, |c, l| c - l);
        let above = zip3_wide(hi, centre, |h, c| h - c);
        // Both are at least zero, as the centre lies in `[min, max]`.
        let far_sq: u128 = below
            .iter()
            .zip(&above)
            .map(|(b, a)| b.unsigned_abs().max(a.unsigned_abs()).pow(2))
            .sum();
        let root = FixedQ32_32::sqrt_u128(far_sq);
        let radius = if root * root == far_sq { root } else { root + 1 };
        let radius = i64::try_from(radius).map_err(|_| BoundsError::OutOfRange)?;
        Ok(FixedSphere3::new(vec3(centre), FixedQ32_32::from_raw(radius)))
    }
}
