//! Convex polygon clipping against a fixed plane.

use core::fmt;

use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;

use super::{edge_cut, keeps};
use super::FixedPlane3;

/// The result of clipping a polygon against a plane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolygonClip {
    /// The clipped polygon occupies the first `len` slots of the output buffer.
    /// `len` is at least three.
    Kept(usize),
    /// Nothing with three or more vertices survives: the polygon is behind the
    /// plane, or only a vertex or an edge of it touches the plane.
    Outside,
}

/// Why a polygon could not be clipped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolygonClipError {
    /// The input has fewer than three vertices.
    TooFewVertices,
    /// A signed distance or an intersection overflowed.
    Overflow,
    /// The output buffer cannot hold the clipped polygon.
    OutputTooSmall,
}

impl fmt::Display for PolygonClipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::TooFewVertices => "polygon has fewer than three vertices",
            Self::Overflow => "plane clipping overflowed",
            Self::OutputTooSmall => "output buffer is too small for the clipped polygon",
        };
        f.write_str(message)
    }
}

impl std::error::Error for PolygonClipError {}

/// Appends points to a caller-provided buffer without indexing.
struct Sink<'a> {
    out: &'a mut [FixedVec3],
    len: usize,
}

impl Sink<'_> {
    fn push(&mut self, point: FixedVec3) -> Result<(), PolygonClipError> {
        let slot = self.out.get_mut(self.len).ok_or(PolygonClipError::OutputTooSmall)?;
        *slot = point;
        self.len += 1;
        Ok(())
    }
}

/// Whether an edge needs a cut: exactly one end is kept and neither is on the plane.
const fn needs_cut(from: FixedQ32_32, to: FixedQ32_32) -> bool {
    keeps(from) != keeps(to) && from.raw() != 0 && to.raw() != 0
}

impl FixedPlane3 {
    /// Clips a convex polygon to the closed front half-space.
    ///
    /// `polygon` lists the vertices in order; the winding is preserved. The
    /// result is written to the start of `out`; on success the slots after it are
    /// left unchanged. For a convex polygon with `n`
    /// vertices at most `n + 1` are written, so `out.len() >= n + 1` always
    /// suffices. Concave input is not rejected and gives an unspecified but
    /// memory-safe result.
    ///
    /// Output order starts at the first input vertex that is kept, then follows
    /// the input order. A cut point is emitted where an edge leaves or enters
    /// the front, between its kept neighbors. A vertex exactly on the plane is
    /// kept and never duplicated by a cut. A polygon wholly on the plane, or
    /// wholly in front, is returned unchanged. If fewer than three vertices
    /// remain, the result is [`PolygonClip::Outside`]. Input vertices are never
    /// merged, so repeated input vertices stay repeated.
    ///
    /// A cut point is computed in the canonical direction of its edge
    /// (ascending raw `(x, y, z)`), so two polygons sharing an edge get the same
    /// point whichever way each traverses it. See
    /// `docs/topics/plane-clipping/README.md`.
    ///
    /// # Errors
    ///
    /// [`PolygonClipError::TooFewVertices`] for fewer than three input vertices,
    /// [`PolygonClipError::Overflow`] when a distance or cut overflows, and
    /// [`PolygonClipError::OutputTooSmall`] when the result does not fit in
    /// `out`. After an error the contents of `out` are unspecified.
    pub fn clip_polygon(
        self,
        polygon: &[FixedVec3],
        out: &mut [FixedVec3],
    ) -> Result<PolygonClip, PolygonClipError> {
        let count = polygon.len();
        if count < 3 {
            return Err(PolygonClipError::TooFewVertices);
        }
        let Some(first) = self.first_kept(polygon)? else {
            return Ok(PolygonClip::Outside);
        };
        let mut sink = Sink { out, len: 0 };
        let from = polygon.iter().cycle().skip(first).take(count);
        let to = polygon.iter().cycle().skip(first + 1).take(count);
        for (cur, next) in from.zip(to) {
            self.clip_edge(*cur, *next, &mut sink)?;
        }
        Ok(if sink.len >= 3 { PolygonClip::Kept(sink.len) } else { PolygonClip::Outside })
    }

    fn first_kept(self, polygon: &[FixedVec3]) -> Result<Option<usize>, PolygonClipError> {
        for (index, point) in polygon.iter().enumerate() {
            if keeps(self.signed_distance(*point).ok_or(PolygonClipError::Overflow)?) {
                return Ok(Some(index));
            }
        }
        Ok(None)
    }

    fn clip_edge(
        self,
        cur: FixedVec3,
        next: FixedVec3,
        sink: &mut Sink<'_>,
    ) -> Result<(), PolygonClipError> {
        let from = self.signed_distance(cur).ok_or(PolygonClipError::Overflow)?;
        let to = self.signed_distance(next).ok_or(PolygonClipError::Overflow)?;
        if keeps(from) {
            sink.push(cur)?;
        }
        if needs_cut(from, to) {
            sink.push(edge_cut(self, cur, next).ok_or(PolygonClipError::Overflow)?)?;
        }
        Ok(())
    }
}
