//! Helper utilities for spatial queries and broadphase systems.

use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;

/// Computes the union bounding box of two AABBs.
#[must_use]
pub fn union_aabb(a: FixedAabb3, b: FixedAabb3) -> FixedAabb3 {
    FixedAabb3::new(
        FixedVec3::new(
            std::cmp::min(a.min.x, b.min.x),
            std::cmp::min(a.min.y, b.min.y),
            std::cmp::min(a.min.z, b.min.z),
        ),
        FixedVec3::new(
            std::cmp::max(a.max.x, b.max.x),
            std::cmp::max(a.max.y, b.max.y),
            std::cmp::max(a.max.z, b.max.z),
        ),
    )
}

/// Computes the surface area of an AABB.
#[must_use]
pub fn surface_area(aabb: &FixedAabb3) -> FixedQ32_32 {
    let size_x = if aabb.max.x > aabb.min.x { aabb.max.x - aabb.min.x } else { FixedQ32_32::ZERO };
    let size_y = if aabb.max.y > aabb.min.y { aabb.max.y - aabb.min.y } else { FixedQ32_32::ZERO };
    let size_z = if aabb.max.z > aabb.min.z { aabb.max.z - aabb.min.z } else { FixedQ32_32::ZERO };

    let two = FixedQ32_32::ONE + FixedQ32_32::ONE;
    two * (size_x * size_y + size_y * size_z + size_z * size_x)
}

/// Computes the centroid of an AABB.
#[must_use]
pub fn get_centroid(aabb: &FixedAabb3) -> FixedVec3 {
    let two = FixedQ32_32::ONE + FixedQ32_32::ONE;
    FixedVec3::new(
        (aabb.min.x + aabb.max.x).checked_div(two).unwrap_or(FixedQ32_32::ZERO),
        (aabb.min.y + aabb.max.y).checked_div(two).unwrap_or(FixedQ32_32::ZERO),
        (aabb.min.z + aabb.max.z).checked_div(two).unwrap_or(FixedQ32_32::ZERO),
    )
}

/// Gets the coordinate value of a vector along a specific axis (0 for X, 1 for Y, 2 for Z).
#[must_use]
pub const fn get_axis_val(v: &FixedVec3, axis: usize) -> FixedQ32_32 {
    match axis {
        0 => v.x,
        1 => v.y,
        _ => v.z,
    }
}

/// Computes the element-wise minimum of two vectors.
#[must_use]
pub fn min_vec(a: FixedVec3, b: FixedVec3) -> FixedVec3 {
    FixedVec3::new(std::cmp::min(a.x, b.x), std::cmp::min(a.y, b.y), std::cmp::min(a.z, b.z))
}

/// Computes the element-wise maximum of two vectors.
#[must_use]
pub fn max_vec(a: FixedVec3, b: FixedVec3) -> FixedVec3 {
    FixedVec3::new(std::cmp::max(a.x, b.x), std::cmp::max(a.y, b.y), std::cmp::max(a.z, b.z))
}

/// Checks if two AABBs overlap.
#[must_use]
pub fn aabbs_overlap(a: &FixedAabb3, b: &FixedAabb3) -> bool {
    a.min.x <= b.max.x
        && a.max.x >= b.min.x
        && a.min.y <= b.max.y
        && a.max.y >= b.min.y
        && a.min.z <= b.max.z
        && a.max.z >= b.min.z
}
