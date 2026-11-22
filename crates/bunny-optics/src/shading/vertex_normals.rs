//! Area-weighted vertex normals of an indexed triangle mesh: `bunny-vertex-normals/1`.

use core::fmt;

use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::FixedQ32_32;
use bunny_query::TRACE_COORD_LIMIT_RAW;

use super::wide::{cross, sub, unit, wide, Wide};

#[cfg(test)]
mod tests;

/// The versioned name of the rule [`VertexNormals::new`] computes. Any change to an output bit
/// is a new version.
pub const VERTEX_NORMALS_ID: &str = "bunny-vertex-normals/1";

/// The most triangles [`VertexNormals::new`] accepts: `2^32`.
///
/// Each face's cross product has components below `2^83` raw squared units, so a vertex that
/// belongs to every triangle sums to below `2^115`, well inside `i128`.
pub const MAX_NORMAL_TRIANGLES: u64 = 1 << 32;

/// One unit normal per vertex of an indexed triangle mesh, from the faces around it.
///
/// See `docs/topics/optics/README.md#vertex-normals`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VertexNormals {
    normals: Vec<Option<FixedUnitVec3>>,
}

impl VertexNormals {
    /// The area-weighted normal of every vertex of the mesh with these `positions` and
    /// `triangles` (three indices into `positions` each).
    ///
    /// Each triangle `[a, b, c]` contributes its exact `i128` cross product
    /// `(p[b] - p[a]) x (p[c] - p[a])`, twice its area along its right-handed normal, to each
    /// of its three vertices. A vertex's normal is its exact sum normalized as
    /// `TriangleSurface::new` normalizes a face normal: scaled by a power of two to a largest
    /// component magnitude in `[2^61, 2^62)` (floored when scaling down), then
    /// `FixedUnitVec3::new`. Each component is within one raw unit of the exact unit vector.
    ///
    /// The sums are exact integer sums, so the result depends only on the multiset of faces:
    /// not on their order, nor on which corner a face lists first. A vertex with no face, or
    /// whose faces sum to exactly zero (only zero-area faces, or faces that cancel), has
    /// `None`. Vertices are never welded: two vertices at the same position get the normals
    /// of their own faces.
    ///
    /// # Errors
    ///
    /// Checked in this order: [`VertexNormalError::TooManyTriangles`] for more than
    /// [`MAX_NORMAL_TRIANGLES`] triangles; [`VertexNormalError::PositionOutsideTraceDomain`]
    /// for the first position with a coordinate magnitude of at least `TRACE_COORD_LIMIT_RAW`;
    /// [`VertexNormalError::IndexOutOfRange`] for the first triangle, and its first corner,
    /// whose index is not below `positions.len()`.
    pub fn new(positions: &[FixedVec3], triangles: &[[u32; 3]]) -> Result<Self, VertexNormalError> {
        check_triangle_count(u64::try_from(triangles.len()).unwrap_or(u64::MAX))?;
        check_positions(positions)?;
        let mut sums = vec![[0_i128; 3]; positions.len()];
        for (triangle, indices) in triangles.iter().enumerate() {
            let corners = corners(positions, triangle, *indices)?;
            let [a, b, c] = corners.map(|(_, point)| point);
            let face = cross(sub(b, a), sub(c, a));
            for (vertex, _) in corners {
                if let Some(sum) = sums.get_mut(vertex) {
                    *sum = add(*sum, face);
                }
            }
        }
        Ok(Self { normals: sums.into_iter().map(unit).collect() })
    }

    /// The normal of each vertex, in the order of the positions given to [`Self::new`]:
    /// `None` for a vertex with no face or a zero sum.
    #[must_use]
    pub fn normals(&self) -> &[Option<FixedUnitVec3>] {
        &self.normals
    }
}

/// Refuses a mesh with more than [`MAX_NORMAL_TRIANGLES`] triangles.
const fn check_triangle_count(count: u64) -> Result<(), VertexNormalError> {
    if count > MAX_NORMAL_TRIANGLES {
        Err(VertexNormalError::TooManyTriangles { count })
    } else {
        Ok(())
    }
}

/// Refuses the first position with a coordinate outside the trace domain. Inside it every edge
/// component is below `2^41` raw, so each cross product component is below `2^83`.
fn check_positions(positions: &[FixedVec3]) -> Result<(), VertexNormalError> {
    let limit = TRACE_COORD_LIMIT_RAW.unsigned_abs();
    let outside = |p: &FixedVec3| [p.x, p.y, p.z].iter().any(|c| c.raw().unsigned_abs() >= limit);
    match positions.iter().enumerate().find(|(_, p)| outside(p)) {
        Some((vertex, &position)) => {
            Err(VertexNormalError::PositionOutsideTraceDomain { vertex, position })
        }
        None => Ok(()),
    }
}

/// The vertex index and widened position of each corner of `indices`, or the first index that
/// is not below `positions.len()`.
fn corners(
    positions: &[FixedVec3],
    triangle: usize,
    indices: [u32; 3],
) -> Result<[(usize, Wide); 3], VertexNormalError> {
    let corner = |index: u32| {
        let vertex = usize::try_from(index).ok()?;
        positions.get(vertex).map(|point| (vertex, wide(*point)))
    };
    let refuse = |index| VertexNormalError::IndexOutOfRange {
        triangle,
        index,
        vertex_count: positions.len(),
    };
    let [a, b, c] = indices;
    Ok([
        corner(a).ok_or_else(|| refuse(a))?,
        corner(b).ok_or_else(|| refuse(b))?,
        corner(c).ok_or_else(|| refuse(c))?,
    ])
}

/// The exact sum. Every partial sum is below `2^115` in magnitude ([`MAX_NORMAL_TRIANGLES`]
/// faces of components below `2^83`), so it cannot overflow.
const fn add(a: Wide, b: Wide) -> Wide {
    let ([ax, ay, az], [bx, by, bz]) = (a, b);
    [ax + bx, ay + by, az + bz]
}

/// Why [`VertexNormals::new`] refused a mesh.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum VertexNormalError {
    /// The mesh has more than [`MAX_NORMAL_TRIANGLES`] triangles.
    TooManyTriangles {
        /// The number of triangles given.
        count: u64,
    },
    /// A position has a coordinate magnitude of at least `TRACE_COORD_LIMIT_RAW` (256 units):
    /// it cannot be the vertex of a trace triangle.
    PositionOutsideTraceDomain {
        /// The index of the first such position.
        vertex: usize,
        /// That position.
        position: FixedVec3,
    },
    /// A triangle lists a vertex index that is not below the number of positions.
    IndexOutOfRange {
        /// The index of the first such triangle.
        triangle: usize,
        /// Its first index that is out of range.
        index: u32,
        /// The number of positions.
        vertex_count: usize,
    },
}

impl fmt::Display for VertexNormalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyTriangles { count } => {
                write!(f, "mesh has {count} triangles, more than {MAX_NORMAL_TRIANGLES}")
            }
            Self::PositionOutsideTraceDomain { vertex, position } => {
                let [x, y, z] = [position.x, position.y, position.z].map(FixedQ32_32::raw);
                write!(f, "vertex {vertex} at raw ({x}, {y}, {z}) is outside the trace domain")
            }
            Self::IndexOutOfRange { triangle, index, vertex_count } => write!(
                f,
                "triangle {triangle} lists vertex {index}, but the mesh has {vertex_count} vertices"
            ),
        }
    }
}

impl std::error::Error for VertexNormalError {}
