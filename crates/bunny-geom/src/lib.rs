#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
// `clippy::nursery` is cherry-picked, not denied as a group (CODE_STANDARDS.md, Rule 10):
// - `missing_const_for_fn` keeps every function that can be `const` usable in constant
//   expressions, as the crate's value constructors and accessors are;
// - `redundant_pub_crate` keeps crate-private items `pub` inside private modules, so the
//   module's own visibility is the one boundary to read;
// - `use_self` keeps `Self` in impls, so a renamed type changes one line.
#![deny(clippy::missing_const_for_fn)]
#![deny(clippy::redundant_pub_crate)]
#![deny(clippy::use_self)]
#![deny(missing_docs)]

//! Geometry primitives for Bunny graphics contracts.
//!
//! The default `std` feature links `std` and turns on the `std` feature of every Bunny
//! dependency. Without it the crate is `no_std` and uses only `core`, with the same public
//! items and the same bits.

use core::fmt;

use bunny_linalg::{FixedUnitVec3, FixedVec3, Vec3};
// dojo: allow float-boundary -- imports the f32 DTO alias for Sphere3
use bunny_num::{is_finite, FixedQ32_32, Scalar};

mod bounds;
mod conversions;
mod plane;
mod predicates;
mod transform;
// The 256-bit integer shared by the predicates and the 2D queries marks its crate-private
// items `pub`, as `clippy::redundant_pub_crate` asks; the workspace's `unreachable_pub` lint
// disagrees, and is allowed on the module, as in `bunny-optics`.
#[allow(unreachable_pub)]
mod wide;

pub use plane::{
    FixedPlane3, FixedSegment3, PlaneSide, PlaneSpan, PolygonClip, PolygonClipError, SegmentClip,
};
pub use transform::{TransformError, TransformedRay3};

pub use bounds::{BoundsError, BOUNDS_CONTRACT_ID};

mod frustum;

pub use frustum::{FixedFrustum, FrustumClass, FrustumSphereError};
pub use predicates::{
    barycentric2, barycentric3_coplanar, doubled_signed_area2, orientation2, orientation3,
    sextupled_signed_volume3, signed_area2_rounded, signed_volume3_rounded, ExactBarycentric,
    ExactDeterminant, Orientation2, Orientation3, TriangleEdge, TriangleLocation, TriangleVertex,
    PREDICATES_CONTRACT_ID,
};

/// Error type for bounding shape constructors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeomError {
    /// AABB min boundary exceeds max boundary.
    InvalidAabbBounds,
    /// A coordinate is not finite.
    NonFiniteCoordinate,
    /// Sphere radius is negative.
    NegativeSphereRadius,
    /// Sphere radius is not finite.
    NonFiniteRadius,
    /// Ray direction normalization failed (zero length or overflow).
    InvalidRayDirection,
    /// A finite float boundary value is outside the Q32.32 fixed-point range.
    FixedValueOutOfRange,
}

impl fmt::Display for GeomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidAabbBounds => "AABB min boundary exceeds max boundary",
            Self::NonFiniteCoordinate => "coordinate is not finite",
            Self::NegativeSphereRadius => "sphere radius is negative",
            Self::NonFiniteRadius => "sphere radius is not finite",
            Self::InvalidRayDirection => "ray direction normalization failed",
            Self::FixedValueOutOfRange => "value is outside the Q32.32 range",
        };
        f.write_str(message)
    }
}

impl core::error::Error for GeomError {}

pub(crate) const fn vec3_is_finite(v: Vec3) -> bool {
    is_finite(v.x) && is_finite(v.y) && is_finite(v.z)
}

/// A 3D ray with finite origin and direction components.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray3 {
    /// The origin of the ray.
    pub origin: Vec3,
    /// The direction vector of the ray.
    pub direction: Vec3,
}

/// A 3D axis-aligned bounding box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb3 {
    /// The minimum corner of the bounding box.
    pub min: Vec3,
    /// The maximum corner of the bounding box.
    pub max: Vec3,
}

// dojo: allow float-boundary -- f32 DTO sphere; converted to FixedSphere3 at the boundary
/// A 3D sphere.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sphere3 {
    /// The center of the sphere.
    pub center: Vec3,
    /// The radius of the sphere.
    pub radius: Scalar,
}

/// A 3D ray with deterministic fixed-point coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedRay3 {
    /// The ray origin.
    pub origin: FixedVec3,
    /// The ray direction vector.
    pub direction: FixedUnitVec3,
}

impl FixedRay3 {
    /// Creates a new deterministic fixed-point ray with a pre-normalized direction.
    #[must_use]
    pub const fn new(origin: FixedVec3, direction: FixedUnitVec3) -> Self {
        Self { origin, direction }
    }

    /// Tries to create a new `FixedRay3` by normalizing the direction.
    ///
    /// # Errors
    /// Returns `GeomError::InvalidRayDirection` if the direction cannot be normalized.
    pub fn try_new(origin: FixedVec3, direction: FixedVec3) -> Result<Self, GeomError> {
        FixedUnitVec3::new(direction)
            .map(|dir| Self::new(origin, dir))
            .ok_or(GeomError::InvalidRayDirection)
    }
}

/// A 3D axis-aligned bounding box with deterministic fixed-point coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedAabb3 {
    /// The minimum corner.
    pub min: FixedVec3,
    /// The maximum corner.
    pub max: FixedVec3,
}

impl FixedAabb3 {
    /// Creates a new deterministic axis-aligned bounding box.
    #[must_use]
    pub const fn new(min: FixedVec3, max: FixedVec3) -> Self {
        Self { min, max }
    }

    /// Tries to create a new `FixedAabb3` with min <= max.
    ///
    /// # Errors
    /// Returns `GeomError::InvalidAabbBounds` if min.x > max.x, min.y > max.y, or min.z > max.z.
    pub fn try_new(min: FixedVec3, max: FixedVec3) -> Result<Self, GeomError> {
        if min.x <= max.x && min.y <= max.y && min.z <= max.z {
            Ok(Self::new(min, max))
        } else {
            Err(GeomError::InvalidAabbBounds)
        }
    }
}

/// A 3D sphere with deterministic fixed-point coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedSphere3 {
    /// The center coordinates.
    pub center: FixedVec3,
    /// The sphere radius.
    pub radius: FixedQ32_32,
}

impl FixedSphere3 {
    /// Creates a new deterministic sphere.
    #[must_use]
    pub const fn new(center: FixedVec3, radius: FixedQ32_32) -> Self {
        Self { center, radius }
    }

    /// Tries to create a new `FixedSphere3` with a non-negative radius.
    ///
    /// # Errors
    /// Returns `GeomError::NegativeSphereRadius` if the radius is negative.
    pub fn try_new(center: FixedVec3, radius: FixedQ32_32) -> Result<Self, GeomError> {
        if radius >= FixedQ32_32::ZERO {
            Ok(Self::new(center, radius))
        } else {
            Err(GeomError::NegativeSphereRadius)
        }
    }
}

mod shapes;

pub use shapes::{
    FixedCapsule3, FixedLine3, FixedObb3, FixedTriangle3, ShapeError, OBB_AXIS_DOT_TOLERANCE_RAW,
    OBB_AXIS_LENGTH_TOLERANCE_RAW,
};

mod planar;

pub use planar::{FixedAabb2, FixedCircle2, FixedRay2, FixedSegment2, PLANAR_CONTRACT_ID};

mod sat;

pub use sat::{
    sat_obb_obb, sat_triangle_aabb, ObbAxis, SatClass, TriangleAabbAxis, SAT_CONTRACT_ID,
};
