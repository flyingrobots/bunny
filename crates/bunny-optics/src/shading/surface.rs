//! A triangle prepared for shading: its exact plane, unit normal and frame.

use bunny_query::TraceTriangle;

use super::frame::ShadingFrame;
use super::wide::{cross, dot, sub, unit, wide, Wide};

/// A triangle's geometric normal, frame and exact plane, computed once per triangle.
///
/// The plane normal is the exact `i128` cross product `(v1 - v0) x (v2 - v0)`, the
/// right-handed normal of the coordinate law, so the triangle's front face (the side from which
/// its vertices appear counter-clockwise, where `TriangleHit::front_face` is `true`) is the side
/// it points to. The unit normal is that vector normalized: each component within one raw unit
/// of the exact unit normal. The frame is [`ShadingFrame::new`] of the unit normal.
///
/// Building one costs two square roots and six divisions (three for the normal, three for the
/// frame), so a renderer builds it once per triangle, not once per hit. See
/// `docs/topics/optics/README.md#geometric-normal`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TriangleSurface {
    plane: Wide,
    offset: i128,
    frame: ShadingFrame,
}

impl TriangleSurface {
    /// Prepares `triangle` for shading.
    ///
    /// Returns `None` for a zero-area triangle (duplicate or collinear vertices: the cross
    /// product is exactly zero), which the exact triangle test never reports a hit on.
    #[must_use]
    pub fn new(triangle: &TraceTriangle) -> Option<Self> {
        let [v0, v1, v2] = triangle.vertices().map(wide);
        // Edges are below 2^41 raw, so the cross product is below 2^83 and its dot product with
        // a vertex below 2^125: exact in i128.
        let plane = cross(sub(v1, v0), sub(v2, v0));
        let normal = unit(plane)?;
        Some(Self { plane, offset: dot(plane, v0), frame: ShadingFrame::new(normal) })
    }

    /// The frame around the front-face unit normal.
    #[must_use]
    pub const fn frame(&self) -> ShadingFrame {
        self.frame
    }

    /// The frame whose normal faces a ray that hit the given face: [`Self::frame`] for the front
    /// face and its [`ShadingFrame::flipped`] for the back face.
    #[must_use]
    pub fn facing_frame(&self, front_face: bool) -> ShadingFrame {
        if front_face {
            self.frame
        } else {
            self.frame.flipped()
        }
    }

    /// The exact plane of the face a ray hit: the normal `(v1 - v0) x (v2 - v0)` (negated for the
    /// back face) and its dot product with `v0`. A point `p` is strictly on the arrival side
    /// when `normal . p > offset`.
    pub(crate) fn facing_plane(&self, front_face: bool) -> (Wide, i128) {
        if front_face {
            (self.plane, self.offset)
        } else {
            (self.plane.map(|c| -c), -self.offset)
        }
    }
}
