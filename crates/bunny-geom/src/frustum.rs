//! Six-plane frusta and conservative culling of points, boxes and spheres.
//!
//! See `docs/topics/frustum/README.md` for the orientation, the combination rule, the
//! rounding of the matrix extraction and what the six-plane test does not prove.

use core::fmt;

use bunny_linalg::{FixedMat4, FixedMat4Row, FixedVec3};
use bunny_num::FixedQ32_32;

use crate::{FixedAabb3, FixedPlane3, FixedSphere3, PlaneSide, PlaneSpan};

mod sphere;

/// Where a point, box or sphere lies relative to a frustum, by the six-plane test.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrustumClass {
    /// Every plane has the whole shape in its closed front half-space. Exact in the
    /// rounded signed distances: the shape touching a face from inside is `Inside`.
    Inside,
    /// Not proven inside or outside: no plane has the shape behind it and at least one
    /// plane is straddled. Conservative: it includes shapes that meet no point of the
    /// frustum, near an edge or a corner.
    Intersecting,
    /// Some plane has the whole shape behind it, touching it from behind included.
    /// Certain, whatever the other planes report, overflow included.
    Outside,
}

/// Error returned by [`FixedFrustum::classify_sphere`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrustumSphereError {
    /// The radius is negative, which [`FixedSphere3::try_new`] refuses.
    NegativeRadius,
    /// A signed distance overflowed and no plane had the sphere behind it.
    Overflow,
}

impl fmt::Display for FrustumSphereError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NegativeRadius => "sphere radius is negative",
            Self::Overflow => "frustum signed distance is outside the Q32.32 range",
        })
    }
}

impl core::error::Error for FrustumSphereError {}

/// A frustum as six planes, each with its front facing into the frustum.
///
/// The inside is the closed set of points in front of or on all six planes, so a point
/// on a face is inside. The planes are held as given: nothing checks that they bound a
/// non-empty or a bounded region, and an empty or unbounded set of six planes is a valid
/// value whose classifications follow the same rule. Planes need not be unit length.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedFrustum {
    /// The left plane.
    pub left: FixedPlane3,
    /// The right plane.
    pub right: FixedPlane3,
    /// The bottom plane.
    pub bottom: FixedPlane3,
    /// The top plane.
    pub top: FixedPlane3,
    /// The near plane.
    pub near: FixedPlane3,
    /// The far plane.
    pub far: FixedPlane3,
}

impl FixedFrustum {
    /// Creates a frustum from its planes in the order left, right, bottom, top, near, far.
    #[must_use]
    pub const fn new(planes: [FixedPlane3; 6]) -> Self {
        let [left, right, bottom, top, near, far] = planes;
        Self { left, right, bottom, top, near, far }
    }

    /// The planes in the order left, right, bottom, top, near, far.
    #[must_use]
    pub const fn planes(self) -> [FixedPlane3; 6] {
        [self.left, self.right, self.bottom, self.top, self.near, self.far]
    }

    /// Extracts the frustum of a `clip_from_space` matrix, in that space.
    ///
    /// With rows `r0` to `r3` and Bunny's depth range `0..=1` (coordinate law), the planes
    /// are `r3 + r0`, `r3 - r0`, `r3 + r1`, `r3 - r1`, `r2` and `r3 - r2`, each row
    /// `(a, b, c, d)` read as the plane `a x + b y + c z + d >= 0` (normal `(a, b, c)`,
    /// offset `-d`). Every coefficient is an exact sum of two entries: nothing is rounded
    /// and nothing is normalised. Returns `None` when a sum or `-d` overflows.
    #[must_use]
    pub fn from_clip_matrix(clip_from_space: FixedMat4) -> Option<Self> {
        let m = clip_from_space;
        let (r0, r1, r2, r3) = (m.row0(), m.row1(), m.row2(), m.row3());
        Some(Self::new([
            row_plane(combine_rows(r3, r0, false)?)?,
            row_plane(combine_rows(r3, r0, true)?)?,
            row_plane(combine_rows(r3, r1, false)?)?,
            row_plane(combine_rows(r3, r1, true)?)?,
            row_plane(r2)?,
            row_plane(combine_rows(r3, r2, true)?)?,
        ]))
    }

    /// Whether a point is inside the closed frustum, by [`FixedPlane3::classify`].
    ///
    /// `Some(false)` when any plane has the point `Behind`, even if another plane's
    /// distance overflows; otherwise `None` when one overflows.
    #[must_use]
    pub fn contains_point(self, point: FixedVec3) -> Option<bool> {
        let spans = self.planes().map(|plane| plane.classify(point).map(point_span));
        combine(spans).map(|class| class == FrustumClass::Inside)
    }

    /// Classifies a box by [`FixedPlane3::classify_aabb`] against each plane.
    ///
    /// `Outside` when some plane has the box `Behind`, even if another plane overflows;
    /// otherwise `None` when a plane overflows; `Inside` when every plane has it `Front`
    /// or `Coplanar`; `Intersecting` otherwise. `Outside` and `Inside` are exact in the
    /// rounded distances; `Intersecting` is conservative. An inverted box is the box with
    /// the same eight corners, as for `classify_aabb`.
    #[must_use]
    pub fn classify_aabb(self, aabb: FixedAabb3) -> Option<FrustumClass> {
        combine(self.planes().map(|plane| plane.classify_aabb(aabb)))
    }

    /// Classifies a sphere against each plane, exactly in the rounded centre distance.
    ///
    /// Against one plane, with `d` the rounded signed distance of the centre, `r` the
    /// radius and `n` the normal, the sphere is behind when `d < 0` and
    /// `d^2 >= r^2 |n|^2`, in front when `d >= 0` and `d^2 >= r^2 |n|^2`, and straddling
    /// otherwise, compared exactly in wide integers with no square root. A sphere
    /// touching a plane takes the side of its other points. Planes combine as for
    /// [`Self::classify_aabb`]. A zero radius gives the point rule.
    ///
    /// # Errors
    /// [`FrustumSphereError::NegativeRadius`] when the radius is negative, and
    /// [`FrustumSphereError::Overflow`] when a centre distance overflows and no plane has
    /// the sphere behind it.
    pub fn classify_sphere(self, sphere: FixedSphere3) -> Result<FrustumClass, FrustumSphereError> {
        let Some(radius) = u64::try_from(sphere.radius.raw()).ok() else {
            return Err(FrustumSphereError::NegativeRadius);
        };
        let spans = self.planes().map(|plane| sphere::span(plane, sphere.center, radius));
        combine(spans).ok_or(FrustumSphereError::Overflow)
    }
}

const fn point_span(side: PlaneSide) -> PlaneSpan {
    match side {
        PlaneSide::Front => PlaneSpan::Front,
        PlaneSide::Behind => PlaneSpan::Behind,
        PlaneSide::On => PlaneSpan::Coplanar,
    }
}

/// The frustum rule: a `Behind` plane wins over everything, then overflow, then a
/// straddled plane; `Front` and `Coplanar` keep the shape inside.
fn combine(spans: [Option<PlaneSpan>; 6]) -> Option<FrustumClass> {
    if spans.contains(&Some(PlaneSpan::Behind)) {
        return Some(FrustumClass::Outside);
    }
    let mut class = FrustumClass::Inside;
    for span in spans {
        if span? == PlaneSpan::Straddling {
            class = FrustumClass::Intersecting;
        }
    }
    Some(class)
}

/// `w + other` or, with `subtract`, `w - other`, entry by entry, checked.
fn combine_rows(w: FixedMat4Row, other: FixedMat4Row, subtract: bool) -> Option<FixedMat4Row> {
    let entry = |a: FixedQ32_32, b: FixedQ32_32| {
        if subtract {
            a.checked_sub(b)
        } else {
            a.checked_add(b)
        }
    };
    Some((entry(w.0, other.0)?, entry(w.1, other.1)?, entry(w.2, other.2)?, entry(w.3, other.3)?))
}

/// The plane `a x + b y + c z + d >= 0` of the row `(a, b, c, d)`.
fn row_plane(row: FixedMat4Row) -> Option<FixedPlane3> {
    Some(FixedPlane3::new(FixedVec3::new(row.0, row.1, row.2), row.3.checked_neg()?))
}
