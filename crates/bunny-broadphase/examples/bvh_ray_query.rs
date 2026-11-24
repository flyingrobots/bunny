//! Build a BVH over triangles and ray-query it: the closest hit, then an any-hit test.
//!
//! Run with `cargo run -p bunny-broadphase --example bvh_ray_query`.
//!
//! The crate allocates nothing: the caller owns the node and primitive-index buffers and
//! sizes them for the worst case, `2 * n - 1` nodes and `n` indices for `n` primitives. The
//! builder returns how many nodes it used, or `None` when a buffer is too small.

use std::error::Error;

use bunny_broadphase::{any_hit, build_bvh_sah, closest_hit, BvhNode, BvhView};
use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::{fixed_q32_32::ONE_RAW, FixedQ32_32};
use bunny_query::{TraceRay, TraceTriangle, TriangleHit};

/// Four triangles in the plane `z = 0`, side by side along `x`.
const TRIANGLE_COUNT: i64 = 4;

/// What the queries found.
#[derive(Debug)]
struct Report {
    nodes_capacity: usize,
    primitive: u32,
    hit: TriangleHit,
    upward_hits: bool,
    outside_domain_refused: bool,
}

/// `x / 4` units as a Q32.32 value, so quarter units stay exact.
const fn quarters(x: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(x * (ONE_RAW / 4))
}

fn at(x: i64, y: i64, z: i64) -> FixedVec3 {
    FixedVec3::new(quarters(x), quarters(y), quarters(z))
}

/// Triangle `k` has corners `(k, 0, 0)`, `(k + 1, 0, 0)` and `(k, 1, 0)`. `TraceTriangle`
/// refuses a vertex outside the trace domain (magnitude below 256 units on every axis),
/// where its exact integer test could overflow.
fn triangles() -> Result<Vec<TraceTriangle>, Box<dyn Error>> {
    (0..TRIANGLE_COUNT)
        .map(|k| {
            TraceTriangle::new(at(4 * k, 0, 0), at(4 * k + 4, 0, 0), at(4 * k, 4, 0))
                .ok_or_else(|| "triangle outside the trace domain".into())
        })
        .collect()
}

/// Builds the tree into buffers sized for the worst case and returns the used nodes.
fn build(boxes: &[FixedAabb3]) -> Result<(Vec<BvhNode>, Vec<u32>), Box<dyn Error>> {
    let first = boxes.first().copied().ok_or("no primitives")?;
    let empty = BvhNode { bounds: first, first_child_or_prim_idx: 0, prim_count: 0 };
    let mut nodes = vec![empty; 2 * boxes.len() - 1];
    let mut prim_indices = vec![0; boxes.len()];
    let used = build_bvh_sah(&mut nodes, &mut prim_indices, boxes).ok_or("buffers too small")?;
    nodes.truncate(used);
    Ok((nodes, prim_indices))
}

/// The nearest triangle `ray` hits within 100 units. The closure tests one primitive; the
/// search passes the best distance so far as its limit, so farther hits are never reported.
/// `TraversalError` (a malformed tree or a tree too deep for the 64-entry stack) has no
/// `Display` or `Error` impl yet (flyingrobots/bunny#445), so it is formatted by hand.
fn nearest(
    bvh: BvhView<'_>,
    ray: &TraceRay,
    mesh: &[TraceTriangle],
) -> Result<Option<(u32, TriangleHit)>, Box<dyn Error>> {
    let test = |primitive: u32, limit| {
        let triangle = mesh.get(usize::try_from(primitive).ok()?)?;
        ray.hit_triangle(triangle, FixedQ32_32::ZERO, limit)
    };
    let found = closest_hit(bvh, ray, quarters(400), |p, limit| test(p, limit).map(|h| h.distance))
        .map_err(|error| format!("malformed tree: {error:?}"))?;
    // `closest_hit` reports the distance; test the winner once more for the full hit record.
    Ok(found.and_then(|best| Some((best.primitive, test(best.primitive, best.distance)?))))
}

fn run() -> Result<Report, Box<dyn Error>> {
    let mesh = triangles()?;
    let boxes: Vec<FixedAabb3> = mesh.iter().map(TraceTriangle::bounds).collect();
    let (nodes, prim_indices) = build(&boxes)?;
    let bvh = BvhView::new(&nodes, &prim_indices);
    // Straight down onto (2.25, 0.25, 0), inside triangle 2. The direction need not be unit
    // length, but each component must be at most 1.0 in magnitude.
    let down = TraceRay::new(at(9, 1, 20), at(0, 0, -4)).ok_or("ray outside the trace domain")?;
    let (primitive, hit) = nearest(bvh, &down, &mesh)?.ok_or("the ray missed")?;
    let up = TraceRay::new(at(9, 1, 20), at(0, 0, 4)).ok_or("ray outside the trace domain")?;
    let upward_hits = any_hit(bvh, &up, quarters(400), |p| hits(&mesh, &up, p))
        .map_err(|error| format!("malformed tree: {error:?}"))?;
    let outside_domain_refused = TraceRay::new(at(1_200, 0, 0), at(0, 0, -4)).is_none();
    Ok(Report {
        nodes_capacity: 2 * boxes.len() - 1,
        primitive,
        hit,
        upward_hits,
        outside_domain_refused,
    })
}

/// Whether `ray` hits primitive `p` at all, for `any_hit`.
fn hits(mesh: &[TraceTriangle], ray: &TraceRay, p: u32) -> bool {
    usize::try_from(p)
        .ok()
        .and_then(|index| mesh.get(index))
        .and_then(|triangle| ray.hit_triangle(triangle, FixedQ32_32::ZERO, quarters(400)))
        .is_some()
}

fn main() -> Result<(), Box<dyn Error>> {
    let report = run()?;
    let (u, v) = report.hit.barycentrics();
    println!("BVH over {TRIANGLE_COUNT} triangles, node buffer of {}", report.nodes_capacity);
    println!("closest hit: triangle {}", report.primitive);
    println!(
        "  distance raw {} (front face: {})",
        report.hit.distance.raw(),
        report.hit.front_face
    );
    println!("  barycentrics raw u {} v {}", u.raw(), v.raw());
    println!("upward ray hits anything: {}", report.upward_hits);
    println!(
        "origin at 300 units refused (trace domain is below 256): {}",
        report.outside_domain_refused
    );
    Ok(())
}

// dojo: allow panic-path -- test assertions pin the example's output to independent values
#[cfg(test)]
mod tests {
    use super::*;

    /// Hand-derived: the ray starts 5 units above the plane and moves 1 unit per unit of
    /// distance, so the hit is at 5.0 exactly. The point (2.25, 0.25) is triangle 2's corner
    /// (2, 0) plus 0.25 of each edge, so both barycentrics are 0.25 = 2^30 raw, exactly. Seen
    /// from above, the corners run counter-clockwise, so the ray hits the front face.
    #[test]
    fn queries_match_the_hand_derivation() -> Result<(), Box<dyn Error>> {
        let report = run()?;
        let (u, v) = report.hit.barycentrics();
        assert_eq!(report.nodes_capacity, 7);
        assert_eq!((report.primitive, report.hit.distance.raw()), (2, 5 * ONE_RAW));
        assert_eq!((u.raw(), v.raw(), report.hit.front_face), (ONE_RAW / 4, ONE_RAW / 4, true));
        assert!(!report.upward_hits);
        assert!(report.outside_domain_refused);
        Ok(())
    }
}
