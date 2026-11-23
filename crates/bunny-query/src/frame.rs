//! The rigid frame a shape is placed with (`bunny-shape-frame/1`).
//!
//! A [`ShapeFrame`] holds three unit axes and an origin, all in world coordinates. It maps a
//! [`TraceRay`] into the shape's object frame and a normal found there back to world space.
//! Each output component is one exact integer dot product rounded once to Q32.32, ties to
//! even. See `docs/topics/shape-frame/README.md`.

use core::fmt;

use crate::{TraceRay, TRACE_COORD_LIMIT_RAW, TRACE_UNIT_DIRECTION_LIMIT_RAW};
use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::FixedQ32_32;

/// The versioned name of [`ShapeFrame::to_object`] and [`ShapeFrame::to_world`]. Any change
/// to an output bit of either is a new version.
pub const SHAPE_FRAME_ID: &str = "bunny-shape-frame/1";

/// Inclusive bound on the magnitude of the exact dot product of two axes of a
/// [`ShapeFrame`], in units of `2^-64`: `2^40`, so the cosine of the angle between two axes
/// is at most `2^-24`.
pub const SHAPE_FRAME_AXIS_DOT_LIMIT_RAW: i128 = 1 << 40;

/// The largest direction shift [`ShapeFrame::to_object`] applies (see its documentation).
pub const SHAPE_FRAME_MAX_DIRECTION_SHIFT: u32 = 1;
const MAX_DIRECTION_SHIFT: u32 = SHAPE_FRAME_MAX_DIRECTION_SHIFT;

type Wide3 = [i128; 3];

/// Why a [`ShapeFrame`] could not be built or could not map a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeFrameError {
    /// An origin coordinate of the frame is outside the trace domain: `abs(raw) >= 2^40`.
    OriginOutsideTraceDomain,
    /// Two axes are not orthogonal: the magnitude of their exact dot product exceeds
    /// [`SHAPE_FRAME_AXIS_DOT_LIMIT_RAW`].
    AxesNotOrthogonal,
    /// The axes are left-handed: `(x_axis x y_axis) . z_axis` is not positive.
    AxesLeftHanded,
    /// The ray origin in object coordinates is outside the trace domain.
    RayOutsideTraceDomain,
    /// Every component of the ray direction in object coordinates rounds to zero. Defensive:
    /// no ray a [`TraceRay`] accepts reaches it.
    DirectionOutsideTraceDomain,
    /// A component of the normal in world coordinates does not fit Q32.32.
    NormalOutOfRange,
}

impl fmt::Display for ShapeFrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::OriginOutsideTraceDomain => "frame origin is outside the trace domain",
            Self::AxesNotOrthogonal => "frame axes are not orthogonal",
            Self::AxesLeftHanded => "frame axes are left-handed",
            Self::RayOutsideTraceDomain => "ray origin in object space is outside the trace domain",
            Self::DirectionOutsideTraceDomain => {
                "ray direction in object space is outside the trace domain"
            }
            Self::NormalOutOfRange => "normal in world space does not fit Q32.32",
        })
    }
}

impl core::error::Error for ShapeFrameError {}

/// A rigid placement: right-handed, orthogonal unit axes and an origin in world space.
///
/// The world point `p` has object coordinates `((p - origin) . x_axis, (p - origin) . y_axis,
/// (p - origin) . z_axis)`. There is no scale or shear; a shape carries its own dimensions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShapeFrame {
    /// The x, y and z axes, one row each.
    axes: [[i64; 3]; 3],
    origin: [i64; 3],
}

impl ShapeFrame {
    /// The frame whose axes are the world axes and whose origin is `origin`.
    ///
    /// # Errors
    ///
    /// [`ShapeFrameError::OriginOutsideTraceDomain`] when an origin coordinate is outside the
    /// trace domain.
    pub fn translation(origin: FixedVec3) -> Result<Self, ShapeFrameError> {
        let axes = [FixedUnitVec3::UNIT_X, FixedUnitVec3::UNIT_Y, FixedUnitVec3::UNIT_Z];
        Self::new(axes, origin)
    }

    /// Builds a frame from its `[x, y, z]` axes and its origin, all in world coordinates.
    ///
    /// # Errors
    ///
    /// [`ShapeFrameError::OriginOutsideTraceDomain`] for an origin outside the trace domain,
    /// [`ShapeFrameError::AxesNotOrthogonal`] when the exact dot product of two axes exceeds
    /// [`SHAPE_FRAME_AXIS_DOT_LIMIT_RAW`] in magnitude, and
    /// [`ShapeFrameError::AxesLeftHanded`] when `(x x y) . z` is not positive.
    pub fn new(axes: [FixedUnitVec3; 3], origin: FixedVec3) -> Result<Self, ShapeFrameError> {
        let origin = raw3(origin);
        if !origin.iter().all(|&c| in_trace_domain(c)) {
            return Err(ShapeFrameError::OriginOutsideTraceDomain);
        }
        let axes = axes.map(|axis| raw3(axis.into_inner()));
        let [x, y, z] = axes.map(wide3);
        let orthogonal = [(x, y), (y, z), (z, x)]
            .iter()
            .all(|&(a, b)| dot(a, b).abs() <= SHAPE_FRAME_AXIS_DOT_LIMIT_RAW);
        if !orthogonal {
            return Err(ShapeFrameError::AxesNotOrthogonal);
        }
        if dot(cross(x, y), z) <= 0 {
            return Err(ShapeFrameError::AxesLeftHanded);
        }
        Ok(Self { axes, origin })
    }

    /// The `[x, y, z]` axes in world coordinates.
    #[must_use]
    pub fn axes(&self) -> [FixedVec3; 3] {
        self.axes.map(vec3)
    }

    /// The origin in world coordinates.
    #[must_use]
    pub const fn origin(&self) -> FixedVec3 {
        vec3(self.origin)
    }

    /// The ray in object coordinates, with its direction scaled to fit the trace domain.
    ///
    /// The origin is `(ray.origin - origin) . axis` for each axis, each exact dot product
    /// rounded once to Q32.32, ties to even. The direction is `(ray.direction . axis) / 2^k`,
    /// each exact dot product divided by `2^k` and rounded once the same way, for the smallest
    /// `k` that keeps every component within [`TRACE_UNIT_DIRECTION_LIMIT_RAW`]. Every
    /// direction a [`TraceRay`] holds is shorter than `sqrt(3) * (1 + 2^-32)`, and every axis
    /// shorter than `1 + 2^-31`, so each exact component is below 2.0 and `k` is 0 or 1
    /// ([`ObjectRay::direction_shift`]). Use [`ObjectRay::world_distance`] to map a distance
    /// along the object ray back to the world ray.
    ///
    /// # Errors
    ///
    /// [`ShapeFrameError::RayOutsideTraceDomain`] when an object origin coordinate is outside
    /// the trace domain, and [`ShapeFrameError::DirectionOutsideTraceDomain`] when every
    /// object direction component rounds to zero (a defensive refusal: a nonzero direction of
    /// a `TraceRay` has a component of at least one raw unit, so no accepted input reaches it).
    pub fn to_object(&self, ray: &TraceRay) -> Result<ObjectRay, ShapeFrameError> {
        let offset = sub3(raw3(ray.origin()), self.origin);
        let origin = self.project(offset, 0).ok_or(ShapeFrameError::RayOutsideTraceDomain)?;
        if !origin.iter().all(|&c| in_trace_domain(c)) {
            return Err(ShapeFrameError::RayOutsideTraceDomain);
        }
        let world = wide3(raw3(ray.direction()));
        let limit = TRACE_UNIT_DIRECTION_LIMIT_RAW.unsigned_abs();
        let fits = |d: &[i64; 3]| d.iter().all(|c| c.unsigned_abs() <= limit);
        let (direction, shift) = (0..=MAX_DIRECTION_SHIFT)
            .find_map(|k| self.project(world, k).filter(fits).map(|d| (d, k)))
            .ok_or(ShapeFrameError::DirectionOutsideTraceDomain)?;
        TraceRay::prepare(vec3(origin), vec3(direction), TRACE_UNIT_DIRECTION_LIMIT_RAW)
            .map(|ray| ObjectRay { ray, shift })
            .ok_or(ShapeFrameError::DirectionOutsideTraceDomain)
    }

    /// A normal (or any direction) given in object coordinates, in world coordinates.
    ///
    /// Each world component is `normal.x * x_axis + normal.y * y_axis + normal.z * z_axis` on
    /// that coordinate, the exact sum rounded once to Q32.32, ties to even. The result is not
    /// renormalized.
    ///
    /// # Errors
    ///
    /// [`ShapeFrameError::NormalOutOfRange`] when a world component does not fit Q32.32.
    pub fn to_world(&self, normal: FixedVec3) -> Result<FixedVec3, ShapeFrameError> {
        let [nx, ny, nz] = wide3(raw3(normal));
        let [[x0, x1, x2], [y0, y1, y2], [z0, z1, z2]] = self.axes.map(wide3);
        let columns = [[x0, y0, z0], [x1, y1, z1], [x2, y2, z2]];
        let world = columns.map(|column| round_q64(dot(column, [nx, ny, nz]), 0));
        match world {
            [Some(wx), Some(wy), Some(wz)] => Ok(vec3([wx, wy, wz])),
            _ => Err(ShapeFrameError::NormalOutOfRange),
        }
    }

    /// `(v . axis) / 2^shift` for each axis, rounded to Q32.32, or `None` when one does not
    /// fit `i64`.
    fn project(&self, v: Wide3, shift: u32) -> Option<[i64; 3]> {
        let [x, y, z] = self.axes.map(|axis| round_q64(dot(wide3(axis), v), shift));
        Some([x?, y?, z?])
    }
}

/// A ray mapped into a shape's object frame by [`ShapeFrame::to_object`].
///
/// The object direction is the world direction rotated and divided by `2^k`, `k` being
/// [`Self::direction_shift`], so the object point at distance `t` is the world point at
/// distance `t / 2^k` along the world ray (to within the rounding of the mapped ray).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectRay {
    ray: TraceRay,
    shift: u32,
}

impl ObjectRay {
    /// The ray in object coordinates, for the shape kernels.
    #[must_use]
    pub const fn ray(&self) -> &TraceRay {
        &self.ray
    }

    /// `k`: the object direction is the rotated world direction divided by `2^k`. It is 0 or
    /// 1 ([`SHAPE_FRAME_MAX_DIRECTION_SHIFT`]).
    #[must_use]
    pub const fn direction_shift(&self) -> u32 {
        self.shift
    }

    /// The world-ray distance of an object-ray distance: `floor(t / 2^k)` in raw units.
    ///
    /// A kernel's distance is the floor of the true object distance, and the floor of a
    /// floor divided by `2^k` is the floor of the true value divided by `2^k`, so the result
    /// is the floor of the exact world distance of the object hit, the same rounding as a
    /// triangle hit on the world ray. Exact, never fails.
    #[must_use]
    pub const fn world_distance(&self, object_distance: FixedQ32_32) -> FixedQ32_32 {
        FixedQ32_32::from_raw(object_distance.raw() >> self.shift)
    }

    /// The object-ray distance of a world-ray distance limit: `t * 2^k`, saturating at the
    /// Q32.32 range. A saturated limit is beyond every distance a kernel reports in the
    /// trace domain.
    #[must_use]
    pub const fn object_distance(&self, world_distance: FixedQ32_32) -> FixedQ32_32 {
        let raw = world_distance.raw();
        let scaled = raw.saturating_mul(1 << self.shift);
        FixedQ32_32::from_raw(scaled)
    }
}

const fn in_trace_domain(raw: i64) -> bool {
    raw.unsigned_abs() < TRACE_COORD_LIMIT_RAW.unsigned_abs()
}

const fn raw3(v: FixedVec3) -> [i64; 3] {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

const fn vec3(raw: [i64; 3]) -> FixedVec3 {
    let [x, y, z] = raw;
    FixedVec3::new(FixedQ32_32::from_raw(x), FixedQ32_32::from_raw(y), FixedQ32_32::from_raw(z))
}

/// A Q64.64 value divided by `2^shift` and rounded to Q32.32, ties to even, or `None` when
/// it does not fit `i64`.
fn round_q64(value: i128, shift: u32) -> Option<i64> {
    let bits = FixedQ32_32::FRACTIONAL_BITS + shift;
    let half = 1_i128 << (bits - 1);
    let floor = value >> bits;
    let rest = value - (floor << bits);
    let up = rest > half || (rest == half && floor & 1 == 1);
    i64::try_from(floor + i128::from(up)).ok()
}

fn wide3(v: [i64; 3]) -> Wide3 {
    v.map(i128::from)
}

fn sub3(lhs: [i64; 3], rhs: [i64; 3]) -> Wide3 {
    let ([a0, a1, a2], [b0, b1, b2]) = (wide3(lhs), wide3(rhs));
    [a0 - b0, a1 - b1, a2 - b2]
}

const fn cross(lhs: Wide3, rhs: Wide3) -> Wide3 {
    let ([a0, a1, a2], [b0, b1, b2]) = (lhs, rhs);
    [a1 * b2 - a2 * b1, a2 * b0 - a0 * b2, a0 * b1 - a1 * b0]
}

const fn dot(lhs: Wide3, rhs: Wide3) -> i128 {
    let ([a0, a1, a2], [b0, b1, b2]) = (lhs, rhs);
    a0 * b0 + a1 * b1 + a2 * b2
}
