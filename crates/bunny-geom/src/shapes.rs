//! Geometry-level shape types: lines, triangles, capsules and oriented boxes.
//!
//! Each type is a representation with stated invariants and degenerate cases, not a query.
//! See `docs/topics/shape-library/README.md`.

use core::fmt;

mod capsule;
mod float;
mod line;
mod obb;
mod triangle;

pub use capsule::FixedCapsule3;
pub use line::FixedLine3;
pub use obb::{FixedObb3, OBB_AXIS_DOT_TOLERANCE_RAW, OBB_AXIS_LENGTH_TOLERANCE_RAW};
pub use triangle::FixedTriangle3;

/// Why a line, triangle, capsule or oriented box constructor refused its input.
///
/// The float boundary constructors of these shapes, and of [`crate::FixedSegment3`] and
/// [`crate::FixedPlane3`], also report float refusals here. New variants may be added as the
/// shape library grows, so matches need a wildcard arm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShapeError {
    /// The line direction is refused by `FixedUnitVec3::new`, the ray's rule: it is zero, its
    /// length overflows, or its normalized length is outside the unit band.
    InvalidLineDirection,
    /// The exact cross product of the triangle's two edges is zero: its vertices are
    /// collinear or coincident.
    DegenerateTriangle,
    /// The capsule radius is negative.
    NegativeCapsuleRadius,
    /// The half extent on this box axis (0, 1 or 2) is negative.
    NegativeHalfExtent {
        /// The axis index.
        axis: usize,
    },
    /// The length of this box axis (column 0, 1 or 2) is more than
    /// [`OBB_AXIS_LENGTH_TOLERANCE_RAW`] raw units from one.
    AxisLengthOutOfTolerance {
        /// The column index.
        column: usize,
    },
    /// The dot product of these two box axes is more than [`OBB_AXIS_DOT_TOLERANCE_RAW`] raw
    /// units from zero.
    AxesNotOrthogonal {
        /// The lower column index.
        first: usize,
        /// The higher column index.
        second: usize,
    },
    /// A float boundary coordinate is `NaN` or infinite.
    NonFiniteCoordinate,
    /// A float boundary radius is `NaN` or infinite.
    NonFiniteRadius,
    /// A finite float boundary value is outside the Q32.32 range after rounding.
    FixedValueOutOfRange,
}

impl fmt::Display for ShapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLineDirection => f.write_str("line direction normalization failed"),
            Self::DegenerateTriangle => f.write_str("triangle vertices are collinear"),
            Self::NegativeCapsuleRadius => f.write_str("capsule radius is negative"),
            Self::NegativeHalfExtent { axis } => {
                write!(f, "box half extent {axis} is negative")
            }
            Self::AxisLengthOutOfTolerance { column } => {
                write!(f, "box axis {column} is not unit length within tolerance")
            }
            Self::AxesNotOrthogonal { first, second } => {
                write!(f, "box axes {first} and {second} are not orthogonal within tolerance")
            }
            Self::NonFiniteCoordinate => f.write_str("coordinate is not finite"),
            Self::NonFiniteRadius => f.write_str("radius is not finite"),
            Self::FixedValueOutOfRange => f.write_str("value is outside the Q32.32 range"),
        }
    }
}

impl core::error::Error for ShapeError {}
