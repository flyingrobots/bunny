//! Topology validation for quantized triangle meshes (`bunny-mesh-topology/1`).
//!
//! [`validate_topology`] reports index-degenerate and geometrically degenerate triangles,
//! exact and flipped duplicate faces, and boundary, winding-inconsistent and non-manifold
//! edges, as [`TopologyFinding`] records in one total order. It does not allocate: the caller
//! provides the scratch ([`TopologyScratch`]) and the output slice.

mod edges;
mod error;
mod faces;
mod finding;
mod scratch;
mod triangles;

pub use error::TopologyError;
pub use finding::{TopologyFinding, TopologyKind, TopologyReport, TopologySeverity};
pub use scratch::{edge_scratch_len, face_scratch_len, EdgeSlot, FaceSlot, TopologyScratch};

use crate::{IndexBufferLayout, QuantizedVertex};
use finding::Sink;

/// The contract identity of the topology findings, their definitions and their order.
pub const MESH_TOPOLOGY_CONTRACT_ID: &str = "bunny-mesh-topology/1";

/// Validates the topology of a triangle mesh and writes its findings in record order.
///
/// Findings are, in record order (the derived `Ord` of [`TopologyFinding`]: kind first, then
/// the fields in declaration order):
///
/// 1. [`TopologyFinding::IndexDegenerateTriangle`]: two or three corner indices are equal.
/// 2. [`TopologyFinding::GeometricallyDegenerateTriangle`]: three distinct indices whose
///    positions are collinear or coincident, decided exactly: the cross product of the two
///    edges from corner 0 is zero in `i64` (edge components fit 17 signed bits; each product of
///    two and each cross component is at most `65_535^2 < 2^32` in magnitude), with no
///    tolerance.
/// 3. [`TopologyFinding::DuplicateFace`] and 4. [`TopologyFinding::FlippedDuplicateFace`]:
///    a face with the same three indices as a lower-indexed face, in the same or in the
///    opposite cyclic order, reported against the lowest-indexed face of that index set.
/// 5. [`TopologyFinding::BoundaryEdge`], 6. [`TopologyFinding::WindingInconsistentEdge`] and
///    7. [`TopologyFinding::NonManifoldEdge`]: an undirected edge used by one face, by two
///    faces in the same direction, or by three or more faces. An edge used by two faces in
///    opposite directions is manifold and reported by nothing.
///
/// Index-degenerate triangles take part in no later check. Every other triangle, duplicates
/// included, contributes its three edges. The first `findings.len()` records are written; the
/// [`TopologyReport`] counts all of them, so [`TopologyReport::is_complete`] is `false` when
/// the slice was too short; there are at most `5 n` records for `n` triangles. Entries past
/// [`TopologyReport::written`] are not touched, and the result does not depend on the
/// scratch contents on entry. Findings never make this fail.
///
/// # Errors
///
/// Checked in this order, before anything is written:
/// [`TopologyError::IndexOutOfRange`] when [`IndexBufferLayout::is_valid`] is `false` for
/// `vertices.len()`; [`TopologyError::TooManyTriangles`] past `u32::MAX` triangles (or when
/// `3 n` overflows `usize`); [`TopologyError::FaceScratchTooSmall`] below
/// [`face_scratch_len`]; [`TopologyError::EdgeScratchTooSmall`] below [`edge_scratch_len`].
pub fn validate_topology(
    vertices: &[QuantizedVertex],
    indices: IndexBufferLayout<'_>,
    scratch: TopologyScratch<'_>,
    findings: &mut [TopologyFinding],
) -> Result<TopologyReport, TopologyError> {
    let (face_slots, edge_slots) = checked_scratch(vertices.len(), indices, scratch)?;
    let mut sink = Sink::new(findings);
    triangles::index_degenerate(indices, &mut sink);
    triangles::geometric_degenerate(vertices, indices, &mut sink);
    faces::duplicates(indices, face_slots, &mut sink);
    edges::classify(indices, edge_slots, &mut sink);
    Ok(sink.report())
}

type Slots<'s> = (&'s mut [FaceSlot], &'s mut [EdgeSlot]);

fn checked_scratch<'s>(
    vertex_count: usize,
    indices: IndexBufferLayout<'_>,
    scratch: TopologyScratch<'s>,
) -> Result<Slots<'s>, TopologyError> {
    if !indices.is_valid(vertex_count) {
        return Err(TopologyError::IndexOutOfRange);
    }
    let triangle_count = indices.len();
    let too_many = TopologyError::TooManyTriangles { triangle_count };
    u32::try_from(triangle_count).map_err(|_| too_many)?;
    let edges_required = edge_scratch_len(triangle_count).ok_or(too_many)?;
    let (faces, edges) = scratch.into_parts();
    let (faces_provided, edges_provided) = (faces.len(), edges.len());
    let faces = faces.get_mut(..face_scratch_len(triangle_count)).ok_or(
        TopologyError::FaceScratchTooSmall { required: triangle_count, provided: faces_provided },
    )?;
    let edges = edges.get_mut(..edges_required).ok_or(TopologyError::EdgeScratchTooSmall {
        required: edges_required,
        provided: edges_provided,
    })?;
    Ok((faces, edges))
}
