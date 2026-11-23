//! The ray footprint at a triangle hit: how far the texture coordinates change across one
//! pixel to the right and one pixel down, `bunny-ray-footprint/1`.

mod wide;

use core::fmt;

use bunny_linalg::FixedVec2;
use bunny_num::FixedQ32_32;
use bunny_query::TraceTriangle;

use crate::plane::{dot, Wide3};
use wide::{rounded_quotient, I320};

/// The versioned name of [`crate::PinholeCamera::footprint`] and
/// [`crate::ThinLensCamera::footprint`]. Any change to an output bit is a new version.
pub const RAY_FOOTPRINT_ID: &str = "bunny-ray-footprint/1";

/// A trace triangle with a texture coordinate at each vertex, refused when it has no plane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TexturedTriangle {
    triangle: TraceTriangle,
    uvs: [FixedVec2; 3],
}

impl TexturedTriangle {
    /// The triangle `v0`, `v1`, `v2` with the texture coordinates `uvs` in the same order.
    ///
    /// # Errors
    /// [`RayFootprintError::DegenerateTriangle`] when `(v1 - v0) x (v2 - v0)` is exactly zero:
    /// the vertices are collinear and span no plane.
    pub fn new(triangle: TraceTriangle, uvs: [FixedVec2; 3]) -> Result<Self, RayFootprintError> {
        let [v0, v1, v2] = triangle.vertices().map(raw3);
        if cross(sub(v1, v0), sub(v2, v0)) == [0; 3] {
            return Err(RayFootprintError::DegenerateTriangle);
        }
        Ok(Self { triangle, uvs })
    }

    /// The triangle.
    #[must_use]
    pub const fn triangle(&self) -> TraceTriangle {
        self.triangle
    }

    /// The texture coordinates of `v0`, `v1` and `v2`.
    #[must_use]
    pub const fn uvs(&self) -> [FixedVec2; 3] {
        self.uvs
    }
}

/// The change of the texture coordinates `(u, v)` across one pixel, raw Q32.32 per pixel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextureDerivatives {
    dx: FixedVec2,
    dy: FixedVec2,
}

impl TextureDerivatives {
    /// The derivatives `dx = (du/dx, dv/dx)` and `dy = (du/dy, dv/dy)`, for a caller that
    /// filters with a footprint it already has.
    #[must_use]
    pub const fn new(dx: FixedVec2, dy: FixedVec2) -> Self {
        Self { dx, dy }
    }

    /// `(du/dx, dv/dx)`: one pixel to the right, toward growing columns.
    #[must_use]
    pub const fn dx(&self) -> FixedVec2 {
        self.dx
    }

    /// `(du/dy, dv/dy)`: one pixel down, toward growing rows.
    #[must_use]
    pub const fn dy(&self) -> FixedVec2 {
        self.dy
    }
}

/// Which neighbouring ray made a footprint unbounded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FootprintAxis {
    /// The ray one pixel to the right, giving `dx`.
    Right,
    /// The ray one pixel down, giving `dy`.
    Down,
}

/// Why a footprint has no finite Q32.32 value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UnboundedReason {
    /// The neighbouring ray is parallel to the triangle's plane: `D' . N` is exactly zero.
    Parallel,
    /// The neighbouring ray's line meets the plane behind the ray origin: `D' . N` has the
    /// sign opposite to the hit ray's.
    Behind,
    /// The neighbouring ray meets the plane ahead, but a derivative rounds outside the `i64`
    /// range of raw Q32.32.
    TooLarge,
}

/// The footprint of a pixel at a triangle hit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RayFootprint {
    /// Both neighbouring rays meet the plane ahead and every derivative is representable.
    Bounded(TextureDerivatives),
    /// The first neighbouring ray, in the order right then down, for which the footprint has
    /// no value, and why. A caller filters with the widest footprint it has.
    Unbounded {
        /// The neighbouring ray.
        axis: FootprintAxis,
        /// Why.
        reason: UnboundedReason,
    },
}

/// Why a footprint could not be measured.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RayFootprintError {
    /// The pixel sample is outside the image: the camera would give it no ray.
    SampleOutsideImage,
    /// A lens-sample coordinate of a thin-lens camera is outside `[0, 1)`.
    LensSampleOutsideUnitSquare,
    /// The triangle's vertices are collinear, so it has no plane.
    DegenerateTriangle,
    /// The sample's own ray is parallel to the triangle's plane: `D . N` is exactly zero.
    RayParallelToPlane,
    /// The sample's own ray meets the plane at or behind its origin (`t <= 0`, exact).
    PlaneNotAhead,
}

impl fmt::Display for RayFootprintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::SampleOutsideImage => "pixel sample is outside the image",
            Self::LensSampleOutsideUnitSquare => "lens sample is outside the unit square",
            Self::DegenerateTriangle => "triangle vertices are collinear",
            Self::RayParallelToPlane => "the sample's ray is parallel to the triangle's plane",
            Self::PlaneNotAhead => "the triangle's plane is not ahead of the sample's ray",
        })
    }
}

impl std::error::Error for RayFootprintError {}

/// One ray against the triangle: `D . A`, `D . B` and `D . C`. The barycentric weights of the
/// point where its line meets the plane are `u = (D . A) / (D . C)` and `v = (D . B) / (D . C)`.
#[derive(Clone, Copy)]
struct Projection {
    a: i128,
    b: i128,
    c: i128,
}

/// The footprint of three rays from `origin`: the hit ray and its right and down neighbours
/// (raw directions, each component below `2^34` in magnitude).
///
/// With `w = O - v0`, `A = e2 x w`, `B = w x e1` and `C = e2 x e1 = -N`, every value up to the
/// projections is exact in `i128`. Each derivative is
/// `(t1 - t0) PA + (t2 - t0) PB` over `(D' . C) (D . C)`, with
/// `PA = (D' . A)(D . C) - (D . A)(D' . C)` and `PB` the same with `B`, in exact 320-bit
/// integers, rounded once to nearest raw unit, ties to even.
pub fn measure(
    origin: Wide3,
    rays: [Wide3; 3],
    surface: &TexturedTriangle,
) -> Result<RayFootprint, RayFootprintError> {
    let [v0, v1, v2] = surface.triangle.vertices().map(raw3);
    let (w, e1, e2) = (sub(origin, v0), sub(v1, v0), sub(v2, v0));
    let (a, b, c) = (cross(e2, w), cross(w, e1), cross(e2, e1));
    let [hit, right, down] = rays.map(|d| Projection { a: dot(d, a), b: dot(d, b), c: dot(d, c) });
    check_hit(hit.c, dot(w, c))?;
    let deltas = differences(surface.uvs);
    let axis = |axis, neighbour| derivatives(hit, neighbour, deltas).map_err(|r| (axis, r));
    let footprint = axis(FootprintAxis::Right, right).and_then(|dx| {
        let dy = axis(FootprintAxis::Down, down)?;
        Ok(TextureDerivatives { dx, dy })
    });
    Ok(footprint.map_or_else(
        |(axis, reason)| RayFootprint::Unbounded { axis, reason },
        RayFootprint::Bounded,
    ))
}

/// Refuses a hit ray with `D . C` exactly zero (parallel to the plane) or whose distance
/// `t = -(w . C) / (D . C)` is not positive: `w . C` zero (the origin is on the plane) or of the
/// same sign as `D . C`.
const fn check_hit(along: i128, plane: i128) -> Result<(), RayFootprintError> {
    if along == 0 {
        Err(RayFootprintError::RayParallelToPlane)
    } else if plane == 0 || plane.signum() == along.signum() {
        Err(RayFootprintError::PlaneNotAhead)
    } else {
        Ok(())
    }
}

/// `(du, dv)` from the hit ray to one neighbour, or why there is none.
fn derivatives(
    hit: Projection,
    neighbour: Projection,
    deltas: [[i128; 2]; 2],
) -> Result<FixedVec2, UnboundedReason> {
    if neighbour.c == 0 {
        return Err(UnboundedReason::Parallel);
    }
    if neighbour.c.signum() != hit.c.signum() {
        return Err(UnboundedReason::Behind);
    }
    let pa = I320::product(neighbour.a, hit.c).minus(I320::product(hit.a, neighbour.c));
    let pb = I320::product(neighbour.b, hit.c).minus(I320::product(hit.b, neighbour.c));
    // Positive: the two factors have the same sign.
    let denominator = I320::product(neighbour.c, hit.c);
    let [u, v] = deltas.map(|[d1, d2]| {
        let numerator = pa.times(I320::from_i128(d1)).plus(pb.times(I320::from_i128(d2)));
        rounded_quotient(numerator, denominator).map(FixedQ32_32::from_raw)
    });
    Ok(FixedVec2::new(u.ok_or(UnboundedReason::TooLarge)?, v.ok_or(UnboundedReason::TooLarge)?))
}

/// `[[u1 - u0, u2 - u0], [v1 - v0, v2 - v0]]`, raw, each below `2^64` in magnitude.
fn differences([t0, t1, t2]: [FixedVec2; 3]) -> [[i128; 2]; 2] {
    let raw = |c: FixedQ32_32| i128::from(c.raw());
    [[raw(t1.x) - raw(t0.x), raw(t2.x) - raw(t0.x)], [raw(t1.y) - raw(t0.y), raw(t2.y) - raw(t0.y)]]
}

fn raw3(v: bunny_linalg::FixedVec3) -> Wide3 {
    [v.x, v.y, v.z].map(|c| i128::from(c.raw()))
}

const fn sub(a: Wide3, b: Wide3) -> Wide3 {
    let ([a0, a1, a2], [b0, b1, b2]) = (a, b);
    [a0 - b0, a1 - b1, a2 - b2]
}

const fn cross(a: Wide3, b: Wide3) -> Wide3 {
    let ([a0, a1, a2], [b0, b1, b2]) = (a, b);
    [a1 * b2 - a2 * b1, a2 * b0 - a0 * b2, a0 * b1 - a1 * b0]
}
