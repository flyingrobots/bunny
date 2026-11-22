//! The exact barycentric numerators a triangle hit was decided with.

/// The exact barycentric weights of a point on a triangle: integers over a common denominator.
///
/// The weights of the triangle's vertices `v0`, `v1` and `v2` are `(det - u - v) / det`,
/// `u / det` and `v / det`, with `det > 0`.
///
/// [`crate::TriangleHit::barycentric_numerators`] returns the values the exact triangle test
/// compared to decide the hit (`0 <= u`, `0 <= v`, `u + v <= det`, `det > 0`), with no
/// rounding. For a ray and triangle in the trace domain each is below `2^117`: `det` is
/// `e1 . (d x e2)` with direction components at most `2^32 + 1` and edge components below
/// `2^41`, so `|d x e2|` is below `2^75` per component and `det` below
/// `3 * (2^32 + 1) * 2^83`; `u` and `v` are at most `det`. See
/// `docs/topics/ray-tracing/README.md#exact-triangle-test`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BarycentricNumerators {
    u: i128,
    v: i128,
    det: i128,
}

impl BarycentricNumerators {
    /// The numerators `u` and `v` over `det`, or `None` unless `det > 0`, `u >= 0`, `v >= 0`
    /// and `u + v <= det` (the point is on the triangle, edges and corners included).
    #[must_use]
    pub const fn new(u: i128, v: i128, det: i128) -> Option<Self> {
        let on_triangle = match u.checked_add(v) {
            Some(sum) => det > 0 && u >= 0 && v >= 0 && sum <= det,
            None => false,
        };
        if on_triangle {
            Some(Self { u, v, det })
        } else {
            None
        }
    }

    /// The numerators of a hit the triangle test accepted, which satisfy every condition of
    /// [`Self::new`] by construction.
    pub(crate) const fn from_hit(u: i128, v: i128, det: i128) -> Self {
        Self { u, v, det }
    }

    /// The numerator of the weight of `v1`.
    #[must_use]
    pub const fn u(&self) -> i128 {
        self.u
    }

    /// The numerator of the weight of `v2`.
    #[must_use]
    pub const fn v(&self) -> i128 {
        self.v
    }

    /// The common denominator, positive.
    #[must_use]
    pub const fn det(&self) -> i128 {
        self.det
    }
}
