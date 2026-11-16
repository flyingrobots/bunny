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
    /// A component of the ray direction in object coordinates exceeds
    /// [`TRACE_UNIT_DIRECTION_LIMIT_RAW`] in magnitude, or every component rounds to zero.
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

impl std::error::Error for ShapeFrameError {}

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

    /// The ray in object coordinates.
    ///
    /// The origin is `(ray.origin - origin) . axis` and the direction `ray.direction . axis`
    /// for each axis, each exact dot product rounded once to Q32.32, ties to even. The
    /// direction is not renormalized, so a distance along the object ray is a distance along
    /// the world ray to within that rounding.
    ///
    /// # Errors
    ///
    /// [`ShapeFrameError::RayOutsideTraceDomain`] when an object origin coordinate is outside
    /// the trace domain, and [`ShapeFrameError::DirectionOutsideTraceDomain`] when an object
    /// direction component exceeds [`TRACE_UNIT_DIRECTION_LIMIT_RAW`] in magnitude or every
    /// component is zero.
    pub fn to_object(&self, ray: &TraceRay) -> Result<TraceRay, ShapeFrameError> {
        let offset = sub3(raw3(ray.origin()), self.origin);
        let origin = self.project(offset).ok_or(ShapeFrameError::RayOutsideTraceDomain)?;
        if !origin.iter().all(|&c| in_trace_domain(c)) {
            return Err(ShapeFrameError::RayOutsideTraceDomain);
        }
        let direction = self.project(wide3(raw3(ray.direction())));
        let limit = TRACE_UNIT_DIRECTION_LIMIT_RAW.unsigned_abs();
        direction
            .filter(|d| d.iter().all(|c| c.unsigned_abs() <= limit))
            .and_then(|d| TraceRay::prepare(vec3(origin), vec3(d), TRACE_UNIT_DIRECTION_LIMIT_RAW))
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
        let [x, y, z] = self.axes.map(wide3);
        let column = |i: usize| [x[i], y[i], z[i]];
        let world = [0, 1, 2].map(|i| round_q64(dot(column(i), [nx, ny, nz])));
        match world {
            [Some(wx), Some(wy), Some(wz)] => Ok(vec3([wx, wy, wz])),
            _ => Err(ShapeFrameError::NormalOutOfRange),
        }
    }

    /// `v . axis` for each axis, rounded to Q32.32, or `None` when one does not fit `i64`.
    fn project(&self, v: Wide3) -> Option<[i64; 3]> {
        let [x, y, z] = self.axes.map(|axis| round_q64(dot(wide3(axis), v)));
        Some([x?, y?, z?])
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

/// A Q64.64 value rounded to Q32.32, ties to even, or `None` when it does not fit `i64`.
fn round_q64(value: i128) -> Option<i64> {
    const HALF: i128 = 1 << 31;
    let floor = value >> 32;
    let rest = value - (floor << 32);
    let up = rest > HALF || (rest == HALF && floor & 1 == 1);
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
