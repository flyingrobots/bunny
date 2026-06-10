//! Geometry primitives for Bunny graphics contracts.

use bunny_linalg::Vec3;
use bunny_num::Scalar;

/// A 3D ray with finite origin and direction components.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray3 {
    pub origin: Vec3,
    pub direction: Vec3,
}

/// A 3D axis-aligned bounding box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb3 {
    pub min: Vec3,
    pub max: Vec3,
}

/// A 3D sphere.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sphere3 {
    pub center: Vec3,
    pub radius: Scalar,
}
