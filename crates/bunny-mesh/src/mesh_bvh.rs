//! A borrowed triangle mesh, its BVH, and the ray and closest-point searches over both.
//!
//! [`MeshView`] checks a vertex buffer and a [`IndexBufferLayout`] once; triangle `i` of the
//! index buffer is BVH primitive `i`. [`MeshView::write_bounds`] fills the boxes a BVH builder
//! takes, and [`MeshBvh`] joins the mesh to the built tree. See
//! `docs/topics/mesh-queries/README.md#mesh-view`.

use bunny_broadphase::TraversalError;
use bunny_broadphase::{closest_hit, closest_point, BvhView, ClosestHit, ClosestPoint};
use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use bunny_query::{PointQuery, TraceRay, TraceTriangle, TRACE_COORD_LIMIT_RAW};

use crate::IndexBufferLayout;

/// Why a mesh or a mesh BVH was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshError {
    /// The index buffer has no triangles.
    EmptyMesh,
    /// The index buffer has more than `u32::MAX` triangles, more than BVH primitive indices
    /// can name.
    TooManyTriangles,
    /// A vertex coordinate is outside the trace domain (`abs(raw) < 2^40`); `vertex` is the
    /// first such vertex.
    VertexOutsideTraceDomain {
        /// Index of the vertex in the vertex buffer.
        vertex: usize,
    },
    /// A triangle names a vertex past the end of the vertex buffer; `triangle` is the first.
    IndexOutOfRange {
        /// Index of the triangle in the index buffer.
        triangle: usize,
    },
    /// The output buffer of [`MeshView::write_bounds`] is shorter than the triangle count.
    BoundsBufferTooSmall,
    /// An entry of the BVH's primitive-index buffer names no triangle of the mesh.
    PrimitiveOutOfRange {
        /// Position of the entry in the primitive-index buffer.
        entry: usize,
    },
}

/// A triangle mesh borrowed from a vertex buffer and an index buffer, checked once.
///
/// Every vertex is inside the trace domain and every index names a vertex, so every
/// triangle can be prepared as a [`TraceTriangle`]. Triangles with collinear or coincident
/// vertices are kept: the ray test misses them and the closest-point test treats them as the
/// segment or point they span.
#[derive(Clone, Copy, Debug)]
pub struct MeshView<'a> {
    positions: &'a [FixedVec3],
    indices: IndexBufferLayout<'a>,
}

impl<'a> MeshView<'a> {
    /// Checks and borrows a mesh.
    ///
    /// # Errors
    /// In this order: [`MeshError::EmptyMesh`] for an index buffer with no triangles,
    /// [`MeshError::TooManyTriangles`] past `u32::MAX` triangles,
    /// [`MeshError::VertexOutsideTraceDomain`] for the first vertex with a coordinate outside
    /// the trace domain (unused vertices included), and [`MeshError::IndexOutOfRange`] for the
    /// first triangle that names a missing vertex.
    pub fn new(
        positions: &'a [FixedVec3],
        indices: IndexBufferLayout<'a>,
    ) -> Result<Self, MeshError> {
        if indices.is_empty() {
            return Err(MeshError::EmptyMesh);
        }
        u32::try_from(indices.len()).map_err(|_| MeshError::TooManyTriangles)?;
        if let Some(vertex) = positions.iter().position(|&v| !in_trace_domain(v)) {
            return Err(MeshError::VertexOutsideTraceDomain { vertex });
        }
        let mesh = Self { positions, indices };
        match (0..indices.len()).find(|&triangle| mesh.corners(triangle).is_none()) {
            Some(triangle) => Err(MeshError::IndexOutOfRange { triangle }),
            None => Ok(mesh),
        }
    }

    /// The vertex buffer.
    #[must_use]
    pub const fn positions(&self) -> &'a [FixedVec3] {
        self.positions
    }

    /// The index buffer.
    #[must_use]
    pub const fn indices(&self) -> IndexBufferLayout<'a> {
        self.indices
    }

    /// The number of triangles, at least one.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // `new` refused more than `u32::MAX`.
    pub const fn triangle_count(&self) -> u32 {
        self.indices.len() as u32
    }

    /// Triangle `index` of the index buffer, prepared for the trace and closest-point
    /// kernels, or `None` past the last triangle.
    #[must_use]
    pub fn triangle(&self, index: u32) -> Option<TraceTriangle> {
        let [a, b, c] = self.corners(index as usize)?;
        TraceTriangle::new(a, b, c)
    }

    /// Writes the bounds of triangle `i` to `out[i]`, the primitive boxes for
    /// `bunny_broadphase::build_bvh_sah` or `build_bvh`, and returns the triangle count.
    ///
    /// # Errors
    /// [`MeshError::BoundsBufferTooSmall`] when `out` is shorter than the triangle count;
    /// nothing is written.
    pub fn write_bounds(&self, out: &mut [FixedAabb3]) -> Result<usize, MeshError> {
        let out = out.get_mut(..self.indices.len()).ok_or(MeshError::BoundsBufferTooSmall)?;
        for (index, slot) in (0..).zip(out.iter_mut()) {
            if let Some(triangle) = self.triangle(index) {
                *slot = triangle.bounds();
            }
        }
        Ok(out.len())
    }

    /// The three vertices of triangle `index`, or `None` if it or a vertex is missing.
    fn corners(&self, index: usize) -> Option<[FixedVec3; 3]> {
        let named = match self.indices {
            IndexBufferLayout::Width16(faces) => {
                faces.get(index).map(|f| [f.v0, f.v1, f.v2].map(usize::from))
            }
            IndexBufferLayout::Width32(faces) => faces.get(index).and_then(|f| {
                let [a, b, c] = [f.v0, f.v1, f.v2].map(usize::try_from);
                Some([a.ok()?, b.ok()?, c.ok()?])
            }),
        }?;
        let [a, b, c] = named.map(|vertex| self.positions.get(vertex).copied());
        Some([a?, b?, c?])
    }
}

/// A [`MeshView`] joined to a BVH built over its triangle bounds.
///
/// ```
/// use bunny_broadphase::{build_bvh_sah, BvhNode, BvhView};
/// use bunny_geom::FixedAabb3;
/// use bunny_linalg::FixedVec3;
/// use bunny_mesh::{IndexBufferLayout, MeshBvh, MeshView, Triangle16};
/// use bunny_num::{fixed_q32_32::ONE_RAW, FixedQ32_32};
/// use bunny_query::{PointQuery, TraceRay};
///
/// let units = |n: i64| FixedQ32_32::from_raw(n * ONE_RAW);
/// let at = |x, y, z| FixedVec3::new(units(x), units(y), units(z));
/// // A unit square in the plane z = 0, as two triangles sharing the diagonal.
/// let positions = [at(0, 0, 0), at(1, 0, 0), at(1, 1, 0), at(0, 1, 0)];
/// let faces = [Triangle16::new(0, 1, 2), Triangle16::new(0, 2, 3)];
/// let mesh = MeshView::new(&positions, IndexBufferLayout::Width16(&faces)).expect("valid");
///
/// let zero = FixedAabb3::new(at(0, 0, 0), at(0, 0, 0));
/// let mut boxes = [zero; 2];
/// mesh.write_bounds(&mut boxes).expect("room for both");
/// let empty = BvhNode { bounds: zero, first_child_or_prim_idx: 0, prim_count: 0 };
/// let (mut nodes, mut prim_indices) = ([empty; 3], [0; 2]);
/// let used = build_bvh_sah(&mut nodes, &mut prim_indices, &boxes).expect("buffers fit");
/// let bvh = MeshBvh::new(mesh, BvhView::new(&nodes[..used], &prim_indices)).expect("valid");
///
/// // Straight down onto the diagonal: both triangles are hit at 2, the lower index wins.
/// let ray = TraceRay::new(at(0, 0, 2), at(0, 0, -1)).expect("in domain");
/// let hit = bvh.closest_hit(&ray, units(10)).expect("well formed").expect("hit");
/// assert_eq!((hit.primitive, hit.distance), (0, units(2)));
///
/// // Above the far corner (1, 1): the corner itself, 3 units up, on both triangles.
/// let query = PointQuery::new(at(1, 1, 3)).expect("in domain");
/// let nearest = bvh.closest_point(&query).expect("well formed").expect("found");
/// assert_eq!((nearest.primitive, nearest.point), (0, at(1, 1, 0)));
/// assert_eq!(nearest.distance_squared, units(9));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct MeshBvh<'a> {
    mesh: MeshView<'a>,
    bvh: BvhView<'a>,
}

impl<'a> MeshBvh<'a> {
    /// Joins a mesh to a BVH whose primitive `i` is the mesh's triangle `i`.
    ///
    /// The node buffer is checked by the searches, as `closest_hit` checks it; this checks
    /// only that every primitive index names a triangle.
    ///
    /// # Errors
    /// [`MeshError::PrimitiveOutOfRange`] for the first primitive-index entry at or past the
    /// triangle count.
    pub fn new(mesh: MeshView<'a>, bvh: BvhView<'a>) -> Result<Self, MeshError> {
        let count = mesh.triangle_count();
        match bvh.prim_indices.iter().position(|&primitive| primitive >= count) {
            Some(entry) => Err(MeshError::PrimitiveOutOfRange { entry }),
            None => Ok(Self { mesh, bvh }),
        }
    }

    /// The mesh.
    #[must_use]
    pub const fn mesh(&self) -> MeshView<'a> {
        self.mesh
    }

    /// The BVH.
    #[must_use]
    pub const fn bvh(&self) -> BvhView<'a> {
        self.bvh
    }

    /// `bunny_broadphase::closest_hit` over the mesh's triangles with
    /// [`TraceRay::hit_triangle`]: the nearest triangle the ray hits within `0..=t_max`, the
    /// lowest index of equally near ones.
    ///
    /// # Errors
    /// The errors of `closest_hit` for a malformed node buffer.
    pub fn closest_hit(
        &self,
        ray: &TraceRay,
        t_max: FixedQ32_32,
    ) -> Result<Option<ClosestHit>, TraversalError> {
        closest_hit(self.bvh, ray, t_max, |primitive, limit| {
            let triangle = self.mesh.triangle(primitive)?;
            ray.hit_triangle(&triangle, FixedQ32_32::ZERO, limit).map(|hit| hit.distance)
        })
    }

    /// `bunny_broadphase::closest_point` over the mesh's triangles with
    /// [`PointQuery::closest_on_triangle`]: the triangle whose exact closest point is nearest,
    /// the lowest index of equally near ones.
    ///
    /// # Errors
    /// The errors of `closest_point` for a malformed node buffer.
    pub fn closest_point(
        &self,
        query: &PointQuery,
    ) -> Result<Option<ClosestPoint>, TraversalError> {
        closest_point(self.bvh, query, |primitive| {
            self.mesh.triangle(primitive).map(|triangle| query.closest_on_triangle(&triangle))
        })
    }
}

fn in_trace_domain(v: FixedVec3) -> bool {
    [v.x, v.y, v.z].iter().all(|c| c.raw().unsigned_abs() < TRACE_COORD_LIMIT_RAW.unsigned_abs())
}
