#![deny(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(missing_docs)]

//! Geometry primitives for Bunny graphics contracts.

use bunny_linalg::{FixedUnitVec3, FixedVec3, Vec3};
use bunny_num::{is_finite, FixedQ32_32, Scalar};

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
}

const fn vec3_is_finite(v: Vec3) -> bool {
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

impl TryFrom<Ray3> for FixedRay3 {
    type Error = GeomError;

    fn try_from(r: Ray3) -> Result<Self, Self::Error> {
        if !vec3_is_finite(r.origin) || !vec3_is_finite(r.direction) {
            return Err(GeomError::NonFiniteCoordinate);
        }
        Self::try_new(FixedVec3::from(r.origin), FixedVec3::from(r.direction))
    }
}

impl From<FixedRay3> for Ray3 {
    fn from(r: FixedRay3) -> Self {
        Self {
            origin: Vec3::from(r.origin),
            direction: Vec3::from(r.direction.into_inner()),
        }
    }
}

impl TryFrom<Aabb3> for FixedAabb3 {
    type Error = GeomError;

    fn try_from(a: Aabb3) -> Result<Self, Self::Error> {
        if !vec3_is_finite(a.min) || !vec3_is_finite(a.max) {
            return Err(GeomError::NonFiniteCoordinate);
        }
        if a.min.x > a.max.x || a.min.y > a.max.y || a.min.z > a.max.z {
            return Err(GeomError::InvalidAabbBounds);
        }
        Self::try_new(FixedVec3::from(a.min), FixedVec3::from(a.max))
    }
}

impl From<FixedAabb3> for Aabb3 {
    fn from(a: FixedAabb3) -> Self {
        Self {
            min: Vec3::from(a.min),
            max: Vec3::from(a.max),
        }
    }
}

impl TryFrom<Sphere3> for FixedSphere3 {
    type Error = GeomError;

    fn try_from(s: Sphere3) -> Result<Self, Self::Error> {
        if !vec3_is_finite(s.center) {
            return Err(GeomError::NonFiniteCoordinate);
        }
        if !is_finite(s.radius) {
            return Err(GeomError::NonFiniteRadius);
        }
        if s.radius < 0.0 {
            return Err(GeomError::NegativeSphereRadius);
        }
        Self::try_new(FixedVec3::from(s.center), FixedQ32_32::from_f32(s.radius))
    }
}

impl From<FixedSphere3> for Sphere3 {
    fn from(s: FixedSphere3) -> Self {
        Self {
            center: Vec3::from(s.center),
            radius: s.radius.to_f32(),
        }
    }
}
