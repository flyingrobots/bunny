//! Boundary, winding-inconsistent and non-manifold edges, from directed edges sorted in
//! caller scratch.

use super::finding::Sink;
use super::scratch::EdgeSlot;
use super::triangles::distinct_triangles;
use super::{TopologyFinding, TopologyKind};
use crate::IndexBufferLayout;

const EDGE_KINDS: [TopologyKind; 3] = [
    TopologyKind::BoundaryEdge,
    TopologyKind::WindingInconsistentEdge,
    TopologyKind::NonManifoldEdge,
];

const fn directed(from: u32, to: u32, triangle: u32) -> EdgeSlot {
    let forward = from < to;
    let (lo, hi) = if forward { (from, to) } else { (to, from) };
    EdgeSlot { lo, hi, triangle, forward }
}

fn fill(indices: IndexBufferLayout<'_>, slots: &mut [EdgeSlot]) -> usize {
    let edges = distinct_triangles(indices)
        .flat_map(|(t, [a, b, c])| [directed(a, b, t), directed(b, c, t), directed(c, a, t)]);
    let mut used = 0;
    for (target, edge) in slots.iter_mut().zip(edges) {
        *target = edge;
        used += 1;
    }
    used
}

/// The finding for one undirected edge's uses, sorted by triangle; `None` when manifold.
fn finding(group: &[EdgeSlot]) -> Option<TopologyFinding> {
    let first = group.first()?;
    let edge = [first.lo, first.hi];
    match group {
        [only] => Some(TopologyFinding::BoundaryEdge { edge, triangle: only.triangle }),
        [a, b] if a.forward == b.forward => Some(TopologyFinding::WindingInconsistentEdge {
            edge,
            triangles: [a.triangle, b.triangle],
        }),
        [_, _] => None,
        _ => Some(TopologyFinding::NonManifoldEdge {
            edge,
            uses: u32::try_from(group.len()).unwrap_or(u32::MAX),
            first_triangle: first.triangle,
        }),
    }
}

/// Sorts the `3 m` directed edges of the `m` distinct-index faces by (lo, hi, triangle),
/// then scans the groups once per edge kind, so records come in kind, then edge, order. A
/// face with three distinct indices has three distinct edges, so keys are unique and the
/// unstable sort has one result. Cost: one sort of `3 m` slots and three linear scans.
pub(super) fn classify(
    indices: IndexBufferLayout<'_>,
    slots: &mut [EdgeSlot],
    sink: &mut Sink<'_>,
) {
    let used = fill(indices, slots);
    let Some(slots) = slots.get_mut(..used) else { return };
    slots.sort_unstable_by_key(|edge| (edge.lo, edge.hi, edge.triangle));
    for kind in EDGE_KINDS {
        let same_edge = |a: &EdgeSlot, b: &EdgeSlot| (a.lo, a.hi) == (b.lo, b.hi);
        for found in slots.chunk_by(same_edge).filter_map(finding) {
            if found.kind() == kind {
                sink.push(found);
            }
        }
    }
}
