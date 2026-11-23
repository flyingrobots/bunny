//! `MeshView` and `MeshBvh`: refusals, the triangle order, and both searches against a test
//! of every triangle.

use bunny_broadphase::{build_bvh_sah, BvhNode, BvhView, ClosestHit, ClosestPoint};
use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_mesh::{IndexBufferLayout, MeshBvh, MeshError, MeshView, Triangle16, Triangle32};
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_query::{PointQuery, TraceRay, TraceTriangle};
use wasm_bindgen_test::wasm_bindgen_test;

const MESH_SEED: u64 = 0xB002_0246_0000_0003;
const EDGE: i64 = (1 << 40) - 1;

fn vec3(raw: [i64; 3]) -> FixedVec3 {
    FixedVec3::new(
        FixedQ32_32::from_raw(raw[0]),
        FixedQ32_32::from_raw(raw[1]),
        FixedQ32_32::from_raw(raw[2]),
    )
}

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

    fn raw3(&mut self, bound: i64) -> [i64; 3] {
        [self.signed(bound), self.signed(bound), self.signed(bound)]
    }
}

/// A closed octahedron of radius 2 units around the origin, plus a degenerate face.
fn octahedron() -> (Vec<FixedVec3>, Vec<Triangle32>) {
    let r = 2 * ONE_RAW;
    let positions = [[r, 0, 0], [-r, 0, 0], [0, r, 0], [0, -r, 0], [0, 0, r], [0, 0, -r]].map(vec3);
    let faces = [
        [0, 2, 4],
        [2, 1, 4],
        [1, 3, 4],
        [3, 0, 4],
        [2, 0, 5],
        [1, 2, 5],
        [3, 1, 5],
        [0, 3, 5],
        [0, 1, 1],
    ]
    .map(|[a, b, c]| Triangle32::new(a, b, c));
    (positions.to_vec(), faces.to_vec())
}

fn empty_node() -> BvhNode {
    BvhNode {
        bounds: FixedAabb3::new(vec3([0; 3]), vec3([0; 3])),
        first_child_or_prim_idx: 0,
        prim_count: 0,
    }
}

/// The documented layout: check the mesh, write its bounds, build with the SAH builder.
struct Built {
    nodes: Vec<BvhNode>,
    prim_indices: Vec<u32>,
}

impl Built {
    fn new(mesh: &MeshView<'_>) -> Self {
        let count = mesh.triangle_count() as usize;
        let mut boxes = vec![FixedAabb3::new(vec3([0; 3]), vec3([0; 3])); count];
        assert_eq!(mesh.write_bounds(&mut boxes), Ok(count));
        let mut nodes = vec![empty_node(); 2 * count];
        let mut prim_indices = vec![0; count];
        let used = build_bvh_sah(&mut nodes, &mut prim_indices, &boxes).expect("buffers fit");
        nodes.truncate(used);
        Self { nodes, prim_indices }
    }

    fn view(&self) -> BvhView<'_> {
        BvhView::new(&self.nodes, &self.prim_indices)
    }
}

fn brute_hit(mesh: &MeshView<'_>, ray: &TraceRay) -> Option<ClosestHit> {
    (0..mesh.triangle_count())
        .filter_map(|primitive| {
            let t = mesh.triangle(primitive)?;
            let hit = ray.hit_triangle(&t, FixedQ32_32::ZERO, FixedQ32_32::from_raw(i64::MAX))?;
            Some(ClosestHit { primitive, distance: hit.distance })
        })
        .min_by_key(|hit| (hit.distance, hit.primitive))
}

fn brute_point(mesh: &MeshView<'_>, query: &PointQuery) -> Option<ClosestPoint> {
    let nearest = (0..mesh.triangle_count())
        .map(|i| (i, query.closest_on_triangle(&mesh.triangle(i).expect("in range"))))
        .reduce(|best, next| if next.1.cmp_distance(&best.1).is_lt() { next } else { best })?;
    let (primitive, found) = nearest;
    Some(ClosestPoint {
        primitive,
        point: found.point(),
        distance_squared: found.distance_squared(),
    })
}

#[wasm_bindgen_test(unsupported = test)]
fn mq_tp_020_meshes_are_refused_with_typed_errors_in_order() {
    let (positions, faces) = octahedron();
    let wide = IndexBufferLayout::Width32(&faces);
    assert_eq!(
        MeshView::new(&positions, IndexBufferLayout::Width32(&[])).err(),
        Some(MeshError::EmptyMesh)
    );
    assert_eq!(
        MeshView::new(&[], IndexBufferLayout::Width16(&[])).err(),
        Some(MeshError::EmptyMesh)
    );
    // The domain bound, on and one raw unit past it, on any axis of any vertex, used or not.
    for axis in 0..3 {
        for (raw, ok) in [(EDGE, true), (-EDGE, true), (EDGE + 1, false), (-EDGE - 1, false)] {
            let mut moved = positions.clone();
            let mut coordinates = [0; 3];
            coordinates[axis] = raw;
            moved.push(vec3(coordinates));
            moved.push(vec3([i64::MAX; 3]));
            let expected = if ok { 7 } else { 6 };
            assert_eq!(
                MeshView::new(&moved, wide).err(),
                Some(MeshError::VertexOutsideTraceDomain { vertex: expected }),
                "axis {axis}, raw {raw}"
            );
        }
    }
    // The first triangle naming a missing vertex, at both widths; vertices are checked first.
    let mut bad = faces.clone();
    bad[3] = Triangle32::new(0, 6, 1);
    bad[5] = Triangle32::new(u32::MAX, 0, 1);
    let bad = IndexBufferLayout::Width32(&bad);
    assert_eq!(
        MeshView::new(&positions, bad).err(),
        Some(MeshError::IndexOutOfRange { triangle: 3 })
    );
    let narrow = [Triangle16::new(0, 1, 2), Triangle16::new(5, 6, 0)];
    let narrow = IndexBufferLayout::Width16(&narrow);
    assert_eq!(
        MeshView::new(&positions, narrow).err(),
        Some(MeshError::IndexOutOfRange { triangle: 1 })
    );
    let mut far = positions.clone();
    far[0] = vec3([1 << 40, 0, 0]);
    assert_eq!(
        MeshView::new(&far, bad).err(),
        Some(MeshError::VertexOutsideTraceDomain { vertex: 0 })
    );
    // Bounds buffers and primitive indices.
    let mesh = MeshView::new(&positions, wide).expect("valid");
    let mut short = vec![FixedAabb3::new(vec3([0; 3]), vec3([0; 3])); 8];
    assert_eq!(mesh.write_bounds(&mut short), Err(MeshError::BoundsBufferTooSmall));
    assert!(
        short.iter().all(|b| *b == FixedAabb3::new(vec3([0; 3]), vec3([0; 3]))),
        "nothing written"
    );
    let built = Built::new(&mesh);
    let mut indices = built.prim_indices.clone();
    indices[4] = 9;
    let nodes = built.nodes.clone();
    assert_eq!(
        MeshBvh::new(mesh, BvhView::new(&nodes, &indices)).err(),
        Some(MeshError::PrimitiveOutOfRange { entry: 4 })
    );
    assert!(MeshBvh::new(mesh, built.view()).is_ok());
}

#[wasm_bindgen_test(unsupported = test)]
fn mq_tp_021_triangle_i_of_the_index_buffer_is_primitive_i_at_both_widths() {
    let (positions, faces) = octahedron();
    let narrow: Vec<Triangle16> = faces
        .iter()
        .map(|f| {
            let n = |v: u32| u16::try_from(v).expect("small");
            Triangle16::new(n(f.v0), n(f.v1), n(f.v2))
        })
        .collect();
    let wide = MeshView::new(&positions, IndexBufferLayout::Width32(&faces)).expect("valid");
    let thin = MeshView::new(&positions, IndexBufferLayout::Width16(&narrow)).expect("valid");
    assert_eq!(wide.triangle_count(), 9);
    assert_eq!(wide.positions().len(), 6);
    assert_eq!(wide.indices().len(), 9);
    let mut boxes = vec![FixedAabb3::new(vec3([0; 3]), vec3([0; 3])); 10];
    assert_eq!(wide.write_bounds(&mut boxes), Ok(9));
    for (i, face) in (0_u32..).zip(&faces) {
        let vertex = |v: u32| positions[v as usize];
        let expected = TraceTriangle::new(vertex(face.v0), vertex(face.v1), vertex(face.v2));
        assert_eq!(wide.triangle(i), expected);
        assert_eq!(thin.triangle(i), expected);
        assert_eq!(Some(boxes[i as usize]), expected.map(|t| t.bounds()));
    }
    assert_eq!(wide.triangle(9), None);
    assert_eq!(boxes[9], FixedAabb3::new(vec3([0; 3]), vec3([0; 3])), "past the count: untouched");
}

#[wasm_bindgen_test(unsupported = test)]
fn mq_tp_022_mesh_searches_equal_a_test_of_every_triangle() {
    let (positions, faces) = octahedron();
    let mesh = MeshView::new(&positions, IndexBufferLayout::Width32(&faces)).expect("valid");
    let built = Built::new(&mesh);
    let bvh = MeshBvh::new(mesh, built.view()).expect("valid");
    assert_eq!(bvh.mesh().triangle_count(), 9);
    assert_eq!(bvh.bvh().nodes.len(), built.nodes.len());
    // The vertex (2, 0, 0) is shared by four faces and the degenerate one, all at distance 0:
    // the lowest index wins. The centre is equidistant from all eight faces.
    let found = bvh.closest_point(&PointQuery::new(vec3([2 * ONE_RAW, 0, 0])).expect("in domain"));
    let found = found.expect("well formed").expect("found");
    assert_eq!((found.primitive, found.distance_squared), (0, FixedQ32_32::ZERO));
    let centre = PointQuery::new(vec3([0; 3])).expect("in domain");
    let found = bvh.closest_point(&centre).expect("well formed").expect("found");
    assert_eq!(found, brute_point(&mesh, &centre).expect("found"));
    // The centre's nearest is the degenerate face along the x axis, through the centre.
    assert_eq!((found.primitive, found.distance_squared), (8, FixedQ32_32::ZERO));
    let mut seeds = Seeds(MESH_SEED);
    let mut hits = 0;
    for case in 0..300 {
        let p = PointQuery::new(vec3(seeds.raw3(3 * ONE_RAW))).expect("in domain");
        assert_eq!(
            bvh.closest_point(&p).expect("well formed"),
            brute_point(&mesh, &p),
            "case {case}"
        );
        // From anywhere near the octahedron toward a point inside its bounds.
        let (origin, target) = (seeds.raw3(4 * ONE_RAW), seeds.raw3(ONE_RAW));
        let toward = [0, 1, 2].map(|k| (target[k] - origin[k]) / 8);
        let ray = TraceRay::new(vec3(origin), vec3(toward)).expect("in domain");
        let hit = bvh.closest_hit(&ray, FixedQ32_32::from_raw(i64::MAX)).expect("well formed");
        assert_eq!(hit, brute_hit(&mesh, &ray), "seed {MESH_SEED:#X}, case {case}");
        hits += usize::from(hit.is_some());
    }
    assert!(hits > 100, "the rays should hit the octahedron: {hits}");
}
