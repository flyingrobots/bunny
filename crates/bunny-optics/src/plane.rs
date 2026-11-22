//! The image plane: every sample direction as one floored affine function.
//!
//! Vectors here are `i128` triples in fine units of `2^-64`, 32 bits below raw Q32.32. A
//! sample's offset from the image centre is carried doubled, as the raw Q32.32 value of
//! `2 * (x + sub_x) - width`, so it is an exact integer for every pixel and sub-pixel offset.

use bunny_linalg::FixedUnitVec3;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;

use crate::basis::Basis;

type Wide3 = [i128; 3];

/// Fine units are raw Q32.32 shifted left by this many bits.
const FINE_SHIFT: u32 = FixedQ32_32::FRACTIONAL_BITS;
/// `centre << CENTRE_SHIFT` has the units of `offset * step`: fine units times raw, doubled.
const CENTRE_SHIFT: u32 = FINE_SHIFT + 1;
/// Shifting `offset * step` right by this many bits gives raw Q32.32.
const DIRECTION_SHIFT: u32 = CENTRE_SHIFT + FINE_SHIFT;
/// The largest scale shift tried; the centre vector stays exact up to it.
const MAX_SCALE_SHIFT: u32 = FixedQ32_32::FRACTIONAL_BITS;
/// Inclusive bound on the magnitude of a direction component (1.0), as in the trace domain.
const DIRECTION_LIMIT: i128 = 1 << FixedQ32_32::FRACTIONAL_BITS;

/// What the image plane is built from: the basis, the tangent and the image size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lens {
    pub basis: Basis,
    pub tan_raw: i64,
    pub width: u32,
    pub height: u32,
}

/// Doubled raw offsets of a sample from the image centre, in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Offset {
    pub x: i64,
    pub y: i64,
}

/// The direction of every sample, scaled by `2^-shift`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Plane {
    /// `forward * 2^-shift`, exact.
    centre: Wide3,
    /// `right * 2 * tan / (height * 2^shift)`, the change per pixel to the right, floored.
    step_x: Wide3,
    /// `up * 2 * tan / (height * 2^shift)`, the change per pixel up, floored.
    step_y: Wide3,
}

impl Plane {
    /// The smallest scale shift whose image-rectangle corners fit the direction limit, and the
    /// plane at that shift; `None` if no shift fits or a corner does not face forward.
    pub fn fit(lens: &Lens) -> Option<(u32, Self)> {
        (0..=MAX_SCALE_SHIFT)
            .map(|shift| (shift, Self::at(lens, shift)))
            .find(|(_, plane)| plane.corners(lens).iter().flatten().all(|c| fits(*c)))
            .filter(|(_, plane)| plane.faces_forward(lens))
    }

    fn at(lens: &Lens, shift: u32) -> Self {
        let divisor = i128::from(lens.height) << shift;
        let doubled_tan = 2 * i128::from(lens.tan_raw);
        let step = |axis: FixedUnitVec3| wide(axis).map(|c| (doubled_tan * c).div_euclid(divisor));
        Self {
            centre: wide(lens.basis.forward).map(|c| (c << FINE_SHIFT) >> shift),
            step_x: step(lens.basis.right),
            step_y: step(lens.basis.up),
        }
    }

    /// `floor((centre * 2^33 + x * step_x + y * step_y) / 2^65)` per component, in raw Q32.32.
    pub fn direction(&self, offset: Offset) -> Wide3 {
        let (x, y) = (i128::from(offset.x), i128::from(offset.y));
        map3(self.centre, self.step_x, self.step_y, |c, sx, sy| {
            ((c << CENTRE_SHIFT) + x * sx + y * sy) >> DIRECTION_SHIFT
        })
    }

    /// Directions at the four corners of the image rectangle, offsets `+-width` and `+-height`.
    fn corners(&self, lens: &Lens) -> [Wide3; 4] {
        let (w, h) = (i64::from(lens.width) * ONE_RAW, i64::from(lens.height) * ONE_RAW);
        [(-w, -h), (w, -h), (-w, h), (w, h)].map(|(x, y)| self.direction(Offset { x, y }))
    }

    /// Whether every corner direction `d` has `d . forward > 2 * |forward|_1`, which makes
    /// `d . forward` positive for every sample inside the rectangle.
    fn faces_forward(&self, lens: &Lens) -> bool {
        let forward = wide(lens.basis.forward);
        let slack = 2 * forward.iter().map(|c| c.abs()).sum::<i128>();
        self.corners(lens).iter().all(|corner| dot(*corner, forward) > slack)
    }
}

const fn fits(component: i128) -> bool {
    -DIRECTION_LIMIT <= component && component <= DIRECTION_LIMIT
}

fn wide(v: FixedUnitVec3) -> Wide3 {
    let v = v.into_inner();
    [v.x, v.y, v.z].map(|c| i128::from(c.raw()))
}

/// Applies `f` to matching components of three triples.
fn map3(a: Wide3, b: Wide3, c: Wide3, f: impl Fn(i128, i128, i128) -> i128) -> Wide3 {
    let ([a0, a1, a2], [b0, b1, b2], [c0, c1, c2]) = (a, b, c);
    [f(a0, b0, c0), f(a1, b1, c1), f(a2, b2, c2)]
}

const fn dot(lhs: Wide3, rhs: Wide3) -> i128 {
    let ([a0, a1, a2], [b0, b1, b2]) = (lhs, rhs);
    a0 * b0 + a1 * b1 + a2 * b2
}
