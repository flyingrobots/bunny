//! Triangle access and the two degeneracy passes.

use super::finding::Sink;
use super::TopologyFinding;
use crate::{IndexBufferLayout, QuantizedVertex};

/// The corner indices of triangle `triangle`, widened to `u32`.
fn corners(indices: IndexBufferLayout<'_>, triangle: usize) -> Option<[u32; 3]> {
    match indices {
        IndexBufferLayout::Width16(faces) => faces
            .get(triangle)
            .map(|face| [u32::from(face.v0), u32::from(face.v1), u32::from(face.v2)]),
        IndexBufferLayout::Width32(faces) => {
            faces.get(triangle).map(|face| [face.v0, face.v1, face.v2])
        }
    }
}

/// Every triangle in index order with its `u32` index. `validate_topology` has refused more
/// than `u32::MAX` triangles, so none is skipped.
pub(super) fn each_triangle(
    indices: IndexBufferLayout<'_>,
) -> impl Iterator<Item = (u32, [u32; 3])> + '_ {
    (0..indices.len()).filter_map(move |t| Some((u32::try_from(t).ok()?, corners(indices, t)?)))
}

/// Every triangle with three distinct corner indices, in index order.
pub(super) fn distinct_triangles(
    indices: IndexBufferLayout<'_>,
) -> impl Iterator<Item = (u32, [u32; 3])> + '_ {
    each_triangle(indices).filter(|(_, corners)| !has_repeated_index(*corners))
}

const fn has_repeated_index([a, b, c]: [u32; 3]) -> bool {
    a == b || b == c || a == c
}

pub(super) fn index_degenerate(indices: IndexBufferLayout<'_>, sink: &mut Sink<'_>) {
    for (triangle, corners) in each_triangle(indices) {
        if has_repeated_index(corners) {
            sink.push(TopologyFinding::IndexDegenerateTriangle { triangle });
        }
    }
}

pub(super) fn geometric_degenerate(
    vertices: &[QuantizedVertex],
    indices: IndexBufferLayout<'_>,
    sink: &mut Sink<'_>,
) {
    for (triangle, corners) in distinct_triangles(indices) {
        if zero_area(vertices, corners) == Some(true) {
            sink.push(TopologyFinding::GeometricallyDegenerateTriangle { triangle });
        }
    }
}

fn position(vertices: &[QuantizedVertex], index: u32) -> Option<[i64; 3]> {
    let vertex = vertices.get(usize::try_from(index).ok()?)?;
    Some([i64::from(vertex.x), i64::from(vertex.y), i64::from(vertex.z)])
}

const fn difference([ax, ay, az]: [i64; 3], [bx, by, bz]: [i64; 3]) -> [i64; 3] {
    [ax - bx, ay - by, az - bz]
}

/// Whether the cross product of the edges from corner 0 is zero, exactly. Coordinates are
/// `u16`, so each edge component is in `[-65_535, 65_535]` (17 signed bits) and each product
/// of two is at most `65_535^2 < 2^32` in magnitude. Each cross component is twice the signed
/// area of the triangle projected onto a coordinate plane, inside a `65_535` square, so it is
/// at most `65_535^2` too. `i64` holds every step exactly, with no rounding.
fn zero_area(vertices: &[QuantizedVertex], [a, b, c]: [u32; 3]) -> Option<bool> {
    let origin = position(vertices, a)?;
    let [ux, uy, uz] = difference(position(vertices, b)?, origin);
    let [vx, vy, vz] = difference(position(vertices, c)?, origin);
    Some(uy * vz == uz * vy && uz * vx == ux * vz && ux * vy == uy * vx)
}
