//! The geometry-level triangle and its exact degeneracy test.

use bunny_linalg::FixedVec3;

use super::ShapeError;

/// A 3D triangle with fixed-point vertices `v0`, `v1`, `v2` over the full Q32.32 range.
///
/// Vertex order is preserved and significant: it is the winding, so `(v0, v1, v2)` and
/// `(v1, v0, v2)` are different triangles under `==`, though both are degenerate or neither
/// is. A degenerate triangle (collinear or coincident vertices) is a valid value of this type;
/// [`Self::is_degenerate`] says which it is and [`Self::try_new`] refuses one.
///
/// This is not the trace-domain `TraceTriangle` of `bunny-query`, whose vertices must lie in
/// `abs(raw) < 2^40`; any `TraceTriangle` vertices also make a `FixedTriangle3`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedTriangle3 {
    /// The first vertex.
    pub v0: FixedVec3,
    /// The second vertex.
    pub v1: FixedVec3,
    /// The third vertex.
    pub v2: FixedVec3,
}

impl FixedTriangle3 {
    /// Creates a triangle from three vertices, in order. Degenerate triangles are accepted.
    #[must_use]
    pub const fn new(v0: FixedVec3, v1: FixedVec3, v2: FixedVec3) -> Self {
        Self { v0, v1, v2 }
    }

    /// Creates a triangle, refusing a degenerate one.
    ///
    /// # Errors
    /// Returns [`ShapeError::DegenerateTriangle`] when [`Self::is_degenerate`] is true.
    pub fn try_new(v0: FixedVec3, v1: FixedVec3, v2: FixedVec3) -> Result<Self, ShapeError> {
        let triangle = Self::new(v0, v1, v2);
        if triangle.is_degenerate() {
            Err(ShapeError::DegenerateTriangle)
        } else {
            Ok(triangle)
        }
    }

    /// The three vertices, in order.
    #[must_use]
    pub const fn vertices(&self) -> [FixedVec3; 3] {
        [self.v0, self.v1, self.v2]
    }

    /// Whether the exact cross product `(v1 - v0) x (v2 - v0)` of the raw vertices is zero.
    ///
    /// Exact, not rounded: each edge difference has up to 65 bits (held in `i128`, magnitude
    /// below `2^64`), and each product of two of them has up to 129 bits (held as a sign and a
    /// `u128` magnitude below `2^128`). A cross component is the difference of two such
    /// products and needs 130 bits, so it is never formed: it is zero exactly when the two
    /// products are equal, which is compared sign and magnitude. The result does not depend
    /// on vertex order.
    #[must_use]
    pub fn is_degenerate(&self) -> bool {
        let [ax, ay, az] = edge(self.v0, self.v1);
        let [bx, by, bz] = edge(self.v0, self.v2);
        same_product(ay, bz, az, by) && same_product(az, bx, ax, bz) && same_product(ax, by, ay, bx)
    }
}

/// The exact raw difference `b - a`; every component has magnitude below `2^64`.
fn edge(a: FixedVec3, b: FixedVec3) -> [i128; 3] {
    let diff = |a: bunny_num::FixedQ32_32, b: bunny_num::FixedQ32_32| {
        i128::from(b.raw()) - i128::from(a.raw())
    };
    [diff(a.x, b.x), diff(a.y, b.y), diff(a.z, b.z)]
}

/// Whether `a * b == c * d` exactly, for operands of magnitude below `2^64`.
fn same_product(a: i128, b: i128, c: i128, d: i128) -> bool {
    product(a, b) == product(c, d)
}

/// The exact product as `(negative, magnitude)`, with zero never negative. Both magnitudes are
/// below `2^64`, so their product is below `2^128` and fits `u128`.
const fn product(a: i128, b: i128) -> (bool, u128) {
    let magnitude = a.unsigned_abs() * b.unsigned_abs();
    (magnitude != 0 && (a < 0) != (b < 0), magnitude)
}
