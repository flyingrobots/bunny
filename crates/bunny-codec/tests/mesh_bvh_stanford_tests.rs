//! The committed Stanford bunny fixture, read as ASCII PLY, traced and queried through the
//! mesh layout (`MeshView`, `MeshView::write_bounds`, `build_bvh_sah`, `MeshBvh`), against a
//! test of every triangle.
//!
//! The fixture is excluded from the published crate, so this file is excluded with it
//! (`docs/topics/release-packaging/README.md#the-file-rule`).

use bunny_broadphase::{build_bvh_sah, BvhNode, BvhView, ClosestHit, ClosestPoint};
use bunny_codec::parse_ascii_ply;
use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_mesh::{IndexBufferLayout, MeshBvh, MeshView, Triangle32};
use bunny_num::FixedQ32_32;
use bunny_query::{PointQuery, TraceRay};
use wasm_bindgen_test::wasm_bindgen_test;

const ASSET: &[u8] = include_bytes!("fixtures/stanford-bunny-ascii/bun_zipper_res3.ply");
const STANFORD_SEED: u64 = 0xB002_0246_0000_0004;
const RAYS: usize = 128;
const QUERIES: usize = 64;
/// One unit in raw Q32.32.
const ONE: i64 = 1 << 32;

struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    fn signed(&mut self, bound: i64) -> i64 {
        let span = u64::try_from(2 * bound + 1).expect("bound is positive");
        i64::try_from(self.next() % span).expect("below the span") - bound
    }

    fn around(&mut self, centre: [i64; 3], reach: i64) -> [i64; 3] {
        centre.map(|c| c + self.signed(reach))
    }
}

fn vec3(raw: [i64; 3]) -> FixedVec3 {
    FixedVec3::new(
        FixedQ32_32::from_raw(raw[0]),
        FixedQ32_32::from_raw(raw[1]),
        FixedQ32_32::from_raw(raw[2]),
    )
}

fn read_fixture() -> (Vec<FixedVec3>, Vec<Triangle32>) {
    let mesh = parse_ascii_ply(ASSET).expect("the fixture parses");
    let positions = mesh
        .vertices()
        .map(|v| {
            let v = v.expect("validated");
            FixedVec3::new(v.x, v.y, v.z)
        })
        .collect();
    let faces = mesh.triangles().map(|t| t.expect("validated")).collect();
    (positions, faces)
}

#[wasm_bindgen_test(unsupported = test)]
fn mq_tp_030_the_stanford_bunny_is_traced_and_queried_through_the_mesh_layout() {
    let (positions, faces) = read_fixture();
    assert_eq!((positions.len(), faces.len()), (1889, 3851));
    let mesh = MeshView::new(&positions, IndexBufferLayout::Width32(&faces)).expect("in domain");
    let count = mesh.triangle_count() as usize;
    let mut boxes = vec![FixedAabb3::new(vec3([0; 3]), vec3([0; 3])); count];
    assert_eq!(mesh.write_bounds(&mut boxes), Ok(count));
    let empty = BvhNode { bounds: boxes[0], first_child_or_prim_idx: 0, prim_count: 0 };
    let (mut nodes, mut prim_indices) = (vec![empty; 2 * count], vec![0; count]);
    let used = build_bvh_sah(&mut nodes, &mut prim_indices, &boxes).expect("buffers fit");
    let bvh = MeshBvh::new(mesh, BvhView::new(&nodes[..used], &prim_indices)).expect("valid");

    // The middle of the bunny's bounds, about (-0.017, 0.109, -0.002) units.
    let centre = [-ONE / 58, ONE * 109 / 1000, -ONE / 500];
    let mut seeds = Seeds(STANFORD_SEED);
    let mut hits = 0;
    for case in 0..RAYS {
        let origin = seeds.around(centre, ONE * 3 / 10);
        let target = seeds.around(centre, ONE / 20);
        let ray = TraceRay::new(vec3(origin), vec3([0, 1, 2].map(|k| target[k] - origin[k])))
            .expect("in domain");
        let found = bvh.closest_hit(&ray, FixedQ32_32::from_raw(i64::MAX)).expect("well formed");
        let expected = (0..mesh.triangle_count())
            .filter_map(|primitive| {
                let t = mesh.triangle(primitive)?;
                let hit =
                    ray.hit_triangle(&t, FixedQ32_32::ZERO, FixedQ32_32::from_raw(i64::MAX))?;
                Some(ClosestHit { primitive, distance: hit.distance })
            })
            .min_by_key(|hit| (hit.distance, hit.primitive));
        assert_eq!(found, expected, "seed {STANFORD_SEED:#X}, ray {case}");
        hits += usize::from(found.is_some());
    }
    assert!(hits > RAYS / 2, "most rays aim at the bunny: {hits} of {RAYS}");

    let brute = |query: &PointQuery| {
        let (primitive, found) = (0..mesh.triangle_count())
            .map(|i| (i, query.closest_on_triangle(&mesh.triangle(i).expect("in range"))))
            .reduce(|best, next| if next.1.cmp_distance(&best.1).is_lt() { next } else { best })
            .expect("not empty");
        ClosestPoint { primitive, point: found.point(), distance_squared: found.distance_squared() }
    };
    for case in 0..QUERIES {
        let query = PointQuery::new(vec3(seeds.around(centre, ONE / 10))).expect("in domain");
        let found = bvh.closest_point(&query).expect("well formed");
        assert_eq!(found, Some(brute(&query)), "seed {STANFORD_SEED:#X}, query {case}");
    }
    // At a vertex, every face that uses it is at distance zero: the lowest such index wins.
    for vertex in (0..positions.len()).step_by(97) {
        let query = PointQuery::new(positions[vertex]).expect("in domain");
        let found = bvh.closest_point(&query).expect("well formed").expect("found");
        let named =
            |f: &Triangle32| [f.v0, f.v1, f.v2].contains(&u32::try_from(vertex).expect("small"));
        let first = faces.iter().position(named).expect("every vertex is used");
        assert_eq!(found.distance_squared, FixedQ32_32::ZERO, "vertex {vertex}");
        assert_eq!(found.point, positions[vertex], "vertex {vertex}");
        assert_eq!(found.primitive as usize, first, "vertex {vertex}");
    }
}
