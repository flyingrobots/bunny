//! The box of a mesh's triangles, part of `bunny-bounds/1`.

use core::fmt;

use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;

use crate::IndexBufferLayout;

/// Why [`mesh_bounds`] refused a mesh.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshBoundsError {
    /// The index buffer has no triangles, so no vertex is used and there are no bounds.
    EmptyMesh,
    /// A triangle has an index at or past the vertex count; `triangle` is the first such
    /// triangle's position in the index buffer.
    IndexOutOfRange {
        /// Position of the first bad triangle in the index buffer.
        triangle: usize,
    },
}

impl fmt::Display for MeshBoundsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyMesh => f.write_str("mesh has no triangles"),
            Self::IndexOutOfRange { triangle } => {
                write!(f, "triangle {triangle} has an index past the vertex buffer")
            }
        }
    }
}

impl std::error::Error for MeshBoundsError {}

/// The three vertex positions at `indices`, or `None` when an index is out of range.
fn corners(vertices: &[FixedVec3], indices: [usize; 3]) -> Option<[FixedVec3; 3]> {
    let [a, b, c] = indices;
    Some([*vertices.get(a)?, *vertices.get(b)?, *vertices.get(c)?])
}

/// The indices of triangle `i` as `usize`; a `u32` index too wide for `usize` maps to
/// `usize::MAX`, which no vertex buffer reaches.
fn face(indices: IndexBufferLayout<'_>, i: usize) -> Option<[usize; 3]> {
    let wide = |index: u32| usize::try_from(index).unwrap_or(usize::MAX);
    match indices {
        IndexBufferLayout::Width16(faces) => {
            faces.get(i).map(|f| [f.v0, f.v1, f.v2].map(usize::from))
        }
        IndexBufferLayout::Width32(faces) => faces.get(i).map(|f| [f.v0, f.v1, f.v2].map(wide)),
    }
}

/// The smallest box containing every triangle of the mesh: each axis's exact minimum and
/// maximum over the vertices the triangles use.
///
/// A vertex no triangle uses is not covered: the bounds are those of the surface, and
/// `FixedAabb3::from_points(vertices)` bounds the whole buffer. Triangles are read once, in
/// order; nothing allocates.
///
/// # Errors
///
/// [`MeshBoundsError::EmptyMesh`] when the index buffer has no triangles;
/// [`MeshBoundsError::IndexOutOfRange`] naming the first triangle with an index at or past
/// `vertices.len()`, the condition `IndexBufferLayout::is_valid` checks.
pub fn mesh_bounds(
    vertices: &[FixedVec3],
    indices: IndexBufferLayout<'_>,
) -> Result<FixedAabb3, MeshBoundsError> {
    let mut bounds: Option<FixedAabb3> = None;
    for triangle in 0..indices.len() {
        let points = face(indices, triangle).and_then(|f| corners(vertices, f));
        let points = points.ok_or(MeshBoundsError::IndexOutOfRange { triangle })?;
        let tri = FixedAabb3::from_triangle(points);
        bounds = Some(bounds.map_or(tri, |b| hull(&b, &tri)));
    }
    bounds.ok_or(MeshBoundsError::EmptyMesh)
}

/// The smallest box containing two ordered boxes: the bounds of their four corners. Exact.
fn hull(a: &FixedAabb3, b: &FixedAabb3) -> FixedAabb3 {
    let first = FixedAabb3::from_triangle([a.min, a.max, b.min]);
    FixedAabb3::from_triangle([first.min, first.max, b.max])
}
