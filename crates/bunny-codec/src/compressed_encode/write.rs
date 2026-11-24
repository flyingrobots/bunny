use bunny_geom::FixedAabb3;
use bunny_mesh::{IndexBufferLayout, QuantizedVertex, Triangle16, Triangle32};

use super::plan::Plan;

const MAGIC: &[u8; 8] = b"BUNNYQZ!";
const VERSION: u8 = 1;
const FLAGS: u16 = 0;

/// The records being encoded.
#[derive(Clone, Copy)]
pub(super) struct Records<'a> {
    pub(super) bounds: &'a FixedAabb3,
    pub(super) vertices: &'a [QuantizedVertex],
    pub(super) triangles: IndexBufferLayout<'a>,
}

/// Writes the whole stream into `stream`, which the caller has sized to `plan.total_len`.
///
/// The bytes come from one iterator of exactly `plan.total_len` items, header first, then the
/// vertex records, then the triangle records, so no offset is computed and nothing is indexed.
pub(super) fn write_stream(stream: &mut [u8], plan: &Plan, records: Records<'_>) {
    let head =
        header(plan, records.bounds).chain(records.vertices.iter().copied().flat_map(vertex_bytes));
    match records.triangles {
        IndexBufferLayout::Width16(faces) => {
            fill(stream, head.chain(faces.iter().copied().flat_map(triangle16_bytes)));
        }
        IndexBufferLayout::Width32(faces) => {
            fill(stream, head.chain(faces.iter().copied().flat_map(triangle32_bytes)));
        }
    }
}

fn fill(stream: &mut [u8], bytes: impl Iterator<Item = u8>) {
    for (slot, byte) in stream.iter_mut().zip(bytes) {
        *slot = byte;
    }
}

/// The 76 header bytes: magic, version, index width, flags, counts, payload length, then the
/// bounds as six raw `i64` values, min x, y, z then max x, y, z.
fn header(plan: &Plan, bounds: &FixedAabb3) -> impl Iterator<Item = u8> {
    let corners =
        [bounds.min.x, bounds.min.y, bounds.min.z, bounds.max.x, bounds.max.y, bounds.max.z];
    MAGIC
        .iter()
        .copied()
        .chain([VERSION, plan.index_width])
        .chain(FLAGS.to_le_bytes())
        .chain(plan.vertex_count.to_le_bytes())
        .chain(plan.triangle_count.to_le_bytes())
        .chain(plan.payload_len.to_le_bytes())
        .chain(corners.into_iter().flat_map(|value| value.to_raw().to_le_bytes()))
}

const fn vertex_bytes(vertex: QuantizedVertex) -> [u8; 6] {
    let [x0, x1] = vertex.x.to_le_bytes();
    let [y0, y1] = vertex.y.to_le_bytes();
    let [z0, z1] = vertex.z.to_le_bytes();
    [x0, x1, y0, y1, z0, z1]
}

const fn triangle16_bytes(face: Triangle16) -> [u8; 6] {
    let [a0, a1] = face.v0.to_le_bytes();
    let [b0, b1] = face.v1.to_le_bytes();
    let [c0, c1] = face.v2.to_le_bytes();
    [a0, a1, b0, b1, c0, c1]
}

const fn triangle32_bytes(face: Triangle32) -> [u8; 12] {
    let [a0, a1, a2, a3] = face.v0.to_le_bytes();
    let [b0, b1, b2, b3] = face.v1.to_le_bytes();
    let [c0, c1, c2, c3] = face.v2.to_le_bytes();
    [a0, a1, a2, a3, b0, b1, b2, b3, c0, c1, c2, c3]
}
