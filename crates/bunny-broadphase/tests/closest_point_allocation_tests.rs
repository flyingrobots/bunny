//! `closest_point` and the point-triangle kernel allocate nothing.

#![cfg(not(target_arch = "wasm32"))]

use bunny_broadphase::{build_bvh_sah, closest_point, BvhNode, BvhView};
use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_query::{PointQuery, TraceTriangle};

fn vec3(x: i64, y: i64, z: i64) -> FixedVec3 {
    FixedVec3::new(FixedQ32_32::from_raw(x), FixedQ32_32::from_raw(y), FixedQ32_32::from_raw(z))
}

#[test]
fn mq_tp_015_closest_point_allocates_nothing() {
    let triangles: Vec<TraceTriangle> = (0..64)
        .map(|i| {
            let x = i64::from(i % 8) * ONE_RAW;
            let y = i64::from(i / 8) * ONE_RAW;
            TraceTriangle::new(
                vec3(x, y, 0),
                vec3(x + ONE_RAW, y, i64::from(i)),
                vec3(x, y + ONE_RAW, 0),
            )
            .expect("in domain")
        })
        .collect();
    let boxes: Vec<FixedAabb3> = triangles.iter().map(TraceTriangle::bounds).collect();
    let empty = BvhNode { bounds: boxes[0], first_child_or_prim_idx: 0, prim_count: 0 };
    let (mut nodes, mut prim_indices) = (vec![empty; 2 * boxes.len()], vec![0; boxes.len()]);
    let used = build_bvh_sah(&mut nodes, &mut prim_indices, &boxes).expect("buffers fit");
    let bvh = BvhView::new(&nodes[..used], &prim_indices);
    let query =
        PointQuery::new(vec3(3 * ONE_RAW + 5, 2 * ONE_RAW, ONE_RAW / 3)).expect("in domain");

    let mut result = None;
    let counts = allocation_counter::measure(|| {
        result = Some(closest_point(bvh, &query, |primitive| {
            triangles.get(primitive as usize).map(|t| query.closest_on_triangle(t))
        }));
    });
    assert_eq!(counts.count_total, 0, "the search and the kernel allocated");
    let found = result.expect("ran").expect("well formed").expect("found");
    let nearest = (0_u32..)
        .zip(&triangles)
        .map(|(i, t)| (i, query.closest_on_triangle(t)))
        .reduce(|best, next| if next.1.cmp_distance(&best.1).is_lt() { next } else { best })
        .expect("not empty");
    assert_eq!((found.primitive, found.point), (nearest.0, nearest.1.point()));
}
