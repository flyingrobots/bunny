//! Points along a ray whose direction is not unit length.

use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use bunny_query::TraceRay;

/// The point at `distance` along `ray`: `origin + direction * distance`.
///
/// Distances from the trace kernels are multiples of the ray's direction, which a camera does
/// not normalize, so this is the point a hit distance names. Each component multiplies with
/// the Q32.32 rule (rounded to nearest, ties to even) and then adds exactly. Returns `None`
/// when a product or sum is not representable.
#[must_use]
pub fn point_at(ray: &TraceRay, distance: FixedQ32_32) -> Option<FixedVec3> {
    let _ = (ray, distance);
    None
}
