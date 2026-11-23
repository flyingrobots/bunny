//! Integration tests: topology validation (`bunny-mesh-topology/1`).
//!
//! The oracle at the bottom of this file is written separately from the kernel and shares no
//! code with it: it uses `BTreeMap` and `BTreeSet` freely (tests may allocate), decides the
//! cyclic order of a face by the parity of its permutation rather than by rotation, and
//! decides collinearity with `i128` edge vectors taken from the second corner, not the first.

use std::collections::{BTreeMap, BTreeSet};

use bunny_mesh::{
    edge_scratch_len, face_scratch_len, validate_topology, EdgeSlot, FaceSlot, IndexBufferLayout,
    QuantizedVertex, TopologyError, TopologyFinding, TopologyKind, TopologyReport, TopologyScratch,
    TopologySeverity, Triangle16, Triangle32, MESH_TOPOLOGY_CONTRACT_ID,
};
use wasm_bindgen_test::wasm_bindgen_test;

use TopologyFinding::{
    BoundaryEdge, DuplicateFace, FlippedDuplicateFace, GeometricallyDegenerateTriangle,
    IndexDegenerateTriangle, NonManifoldEdge, WindingInconsistentEdge,
};

const TOPOLOGY_SEED: u64 = 0xB002_0122_0000_0001;
const TOPOLOGY_CASES: usize = 2_000;
const SCRATCH_CASES: usize = 300;
const FILL: TopologyFinding = IndexDegenerateTriangle { triangle: u32::MAX };

fn v(x: u16, y: u16, z: u16) -> QuantizedVertex {
    QuantizedVertex::new(x, y, z)
}

fn t(a: u32, b: u32, c: u32) -> Triangle32 {
    Triangle32::new(a, b, c)
}

/// The tetrahedron used by several fixtures; every face is wound outward.
fn tetra_vertices() -> Vec<QuantizedVertex> {
    vec![v(0, 0, 0), v(100, 0, 0), v(0, 100, 0), v(0, 0, 100)]
}

fn tetra_faces() -> Vec<Triangle32> {
    vec![t(0, 2, 1), t(0, 1, 3), t(0, 3, 2), t(1, 2, 3)]
}

fn narrow(faces: &[Triangle32]) -> Vec<Triangle16> {
    faces
        .iter()
        .map(|f| {
            let n = |i: u32| u16::try_from(i).expect("fixture index fits u16");
            Triangle16::new(n(f.v0), n(f.v1), n(f.v2))
        })
        .collect()
}

fn run_layout(
    vertices: &[QuantizedVertex],
    indices: IndexBufferLayout<'_>,
) -> (Vec<TopologyFinding>, TopologyReport) {
    let n = indices.len();
    let mut faces = vec![FaceSlot::EMPTY; face_scratch_len(n)];
    let mut edges = vec![EdgeSlot::EMPTY; edge_scratch_len(n).expect("small mesh")];
    let mut out = vec![FILL; 5 * n];
    let scratch = TopologyScratch::new(&mut faces, &mut edges);
    let report = validate_topology(vertices, indices, scratch, &mut out).expect("valid input");
    assert!(report.is_complete(), "5n slots always hold every finding");
    out.truncate(report.written());
    (out, report)
}

/// Runs both index widths (when the indices fit `u16`) and checks they agree bit for bit.
fn run(vertices: &[QuantizedVertex], faces: &[Triangle32]) -> Vec<TopologyFinding> {
    let (wide, report) = run_layout(vertices, IndexBufferLayout::Width32(faces));
    if faces.iter().all(|f| u16::try_from(f.v0.max(f.v1).max(f.v2)).is_ok()) {
        let short = narrow(faces);
        let (narrow_out, narrow_report) = run_layout(vertices, IndexBufferLayout::Width16(&short));
        assert_eq!(narrow_out, wide, "Width16 and Width32 must agree");
        assert_eq!(narrow_report, report);
    }
    for kind in TopologyKind::ALL {
        let listed = wide.iter().filter(|f| f.kind() == kind).count();
        assert_eq!(report.count(kind), u64::try_from(listed).expect("count"), "{kind:?}");
    }
    assert_eq!(report.total(), u64::try_from(wide.len()).expect("count"));
    wide
}

#[wasm_bindgen_test(unsupported = test)]
fn mtopo_tp_001_closed_tetrahedron_has_no_findings() {
    assert_eq!(MESH_TOPOLOGY_CONTRACT_ID, "bunny-mesh-topology/1");
    let found = run(&tetra_vertices(), &tetra_faces());
    assert_eq!(found, Vec::new());
    assert_eq!(found, oracle(&tetra_vertices(), &tetra_faces()));
    assert_eq!(run(&[], &[]), Vec::new(), "the empty mesh is clean");
}

#[wasm_bindgen_test(unsupported = test)]
fn mtopo_tp_002_open_mesh_has_boundary_edges_only() {
    let faces = &tetra_faces()[..3];
    let found = run(&tetra_vertices(), faces);
    assert_eq!(
        found,
        vec![
            BoundaryEdge { edge: [1, 2], triangle: 0 },
            BoundaryEdge { edge: [1, 3], triangle: 1 },
            BoundaryEdge { edge: [2, 3], triangle: 2 },
        ]
    );
    assert_eq!(found, oracle(&tetra_vertices(), faces));
}

#[wasm_bindgen_test(unsupported = test)]
fn mtopo_tp_003_repeated_indices_are_index_degenerate_only() {
    // Collinear positions too, so a kernel that also tested these geometrically would show.
    let vertices = [v(0, 0, 0), v(1, 0, 0), v(2, 0, 0)];
    let faces = [t(0, 0, 1), t(0, 1, 1), t(1, 0, 1), t(2, 2, 2)];
    let found = run(&vertices, &faces);
    let expected: Vec<_> = (0..4).map(|triangle| IndexDegenerateTriangle { triangle }).collect();
    assert_eq!(found, expected, "no duplicate, edge or geometric finding for these faces");
    assert_eq!(found, oracle(&vertices, &faces));
}

#[wasm_bindgen_test(unsupported = test)]
fn mtopo_tp_004_collinear_face_in_a_closed_mesh_is_geometrically_degenerate_only() {
    // Vertex 2 lies on the segment from 0 to 1, so face 0 has zero area.
    let vertices = [v(0, 0, 0), v(100, 0, 0), v(50, 0, 0), v(0, 0, 100)];
    let found = run(&vertices, &tetra_faces());
    assert_eq!(found, vec![GeometricallyDegenerateTriangle { triangle: 0 }]);
    assert_eq!(found, oracle(&vertices, &tetra_faces()));
}

fn geometric(p: [QuantizedVertex; 3]) -> bool {
    let found = run(&p, &[t(0, 1, 2)]);
    assert_eq!(found, oracle(&p, &[t(0, 1, 2)]));
    found.contains(&GeometricallyDegenerateTriangle { triangle: 0 })
}

#[wasm_bindgen_test(unsupported = test)]
fn mtopo_tp_005_the_zero_area_test_is_exact_at_the_coordinate_extremes() {
    let o = v(0, 0, 0);
    // Exactly collinear along the full diagonal, and one unit either side of it.
    assert!(geometric([o, v(65_535, 65_535, 0), v(65_534, 65_534, 0)]));
    assert!(!geometric([o, v(65_535, 65_535, 0), v(65_534, 65_533, 0)]), "cross z = -65535");
    assert!(!geometric([o, v(65_535, 65_535, 0), v(65_533, 65_534, 0)]), "cross z = 65535");
    // A cross component of exactly -1 and +1: the smallest nonzero area.
    assert!(!geometric([o, v(65_535, 65_534, 0), v(65_534, 65_533, 0)]), "cross z = -1");
    assert!(!geometric([o, v(65_535, 65_534, 0), v(1, 1, 0)]), "cross z = 1");
    assert!(geometric([o, v(65_535, 65_534, 0), v(0, 0, 0)]), "coincident with corner 0");
    // Each cross component is -65535^2, past i32: a 32-bit product would overflow.
    assert!(!geometric([v(0, 65_535, 0), v(65_535, 0, 0), v(0, 0, 65_535)]));
    // Coincident positions at distinct indices, two and three of them.
    assert!(geometric([v(7, 8, 9), v(7, 8, 9), v(1, 2, 3)]));
    assert!(geometric([v(65_535, 0, 65_535); 3]));
    // Collinear in three dimensions, off every coordinate plane: (1, 2, 3) + k (2, 3, 4) at
    // k = 16_383 reaches z = 65_535; one unit off in z, y or x is not collinear.
    let (a, b) = (v(1, 2, 3), v(3, 5, 7));
    assert!(geometric([a, b, v(32_767, 49_151, 65_535)]), "k = 16_383");
    assert!(!geometric([a, b, v(32_767, 49_151, 65_534)]), "z one unit off");
    assert!(!geometric([a, b, v(32_767, 49_152, 65_535)]), "y one unit off");
    assert!(!geometric([a, b, v(32_768, 49_151, 65_535)]), "x one unit off");
}

#[wasm_bindgen_test(unsupported = test)]
fn mtopo_tp_006_exact_duplicates_compare_rotated_keys() {
    let vertices = [v(0, 0, 0), v(10, 0, 0), v(0, 10, 0)];
    // Face 1 is face 0 rotated: the same cyclic order, so an exact duplicate.
    let faces = [t(0, 1, 2), t(1, 2, 0)];
    let found = run(&vertices, &faces);
    // The duplicate's edges are each used twice in the same direction: that is implied.
    assert_eq!(
        found,
        vec![
            DuplicateFace { triangle: 1, first: 0 },
            WindingInconsistentEdge { edge: [0, 1], triangles: [0, 1] },
            WindingInconsistentEdge { edge: [0, 2], triangles: [0, 1] },
            WindingInconsistentEdge { edge: [1, 2], triangles: [0, 1] },
        ]
    );
    assert_eq!(found, oracle(&vertices, &faces));
}

#[wasm_bindgen_test(unsupported = test)]
fn mtopo_tp_007_flipped_duplicates_are_reported_against_the_first_face() {
    let vertices = [v(0, 0, 0), v(10, 0, 0), v(0, 10, 0)];
    // A two-sided sheet: face 1 is face 0 reversed, and every edge is used once each way.
    let faces = [t(0, 1, 2), t(2, 1, 0)];
    let found = run(&vertices, &faces);
    assert_eq!(found, vec![FlippedDuplicateFace { triangle: 1, first: 0 }]);
    assert_eq!(found, oracle(&vertices, &faces));

    // Every later face is compared with the lowest-indexed face of its index set.
    let faces = [t(2, 0, 1), t(1, 0, 2), t(0, 1, 2), t(0, 2, 1)];
    let found: Vec<_> = run(&vertices, &faces)
        .into_iter()
        .filter(|f| matches!(f, DuplicateFace { .. } | FlippedDuplicateFace { .. }))
        .collect();
    assert_eq!(
        found,
        vec![
            DuplicateFace { triangle: 2, first: 0 },
            FlippedDuplicateFace { triangle: 1, first: 0 },
            FlippedDuplicateFace { triangle: 3, first: 0 },
        ]
    );
    assert_eq!(run(&vertices, &faces), oracle(&vertices, &faces));
}

#[wasm_bindgen_test(unsupported = test)]
fn mtopo_tp_008_one_reversed_face_gives_winding_inconsistent_edges_only() {
    let mut faces = tetra_faces();
    faces[3] = t(1, 3, 2);
    let found = run(&tetra_vertices(), &faces);
    assert_eq!(
        found,
        vec![
            WindingInconsistentEdge { edge: [1, 2], triangles: [0, 3] },
            WindingInconsistentEdge { edge: [1, 3], triangles: [1, 3] },
            WindingInconsistentEdge { edge: [2, 3], triangles: [2, 3] },
        ]
    );
    assert_eq!(found, oracle(&tetra_vertices(), &faces));
}

#[wasm_bindgen_test(unsupported = test)]
fn mtopo_tp_009_two_tetrahedra_sharing_an_edge_give_one_non_manifold_edge() {
    let vertices = [
        v(200, 200, 200),
        v(300, 200, 200),
        v(200, 300, 200),
        v(200, 200, 300),
        v(200, 100, 200),
        v(200, 200, 100),
    ];
    let mut faces = tetra_faces();
    faces.extend([t(0, 4, 1), t(0, 1, 5), t(0, 5, 4), t(1, 4, 5)]);
    let found = run(&vertices, &faces);
    assert_eq!(found, vec![NonManifoldEdge { edge: [0, 1], uses: 4, first_triangle: 0 }]);
    assert_eq!(found, oracle(&vertices, &faces));
}

/// One mesh with five finding kinds, listed in triangle order unlike the record order.
fn mixed() -> (Vec<QuantizedVertex>, Vec<Triangle32>) {
    let vertices =
        vec![v(0, 0, 0), v(10, 0, 0), v(0, 10, 0), v(0, 0, 10), v(0, 0, 20), v(0, 0, 30)];
    let faces = vec![t(0, 1, 2), t(3, 3, 4), t(2, 1, 0), t(0, 1, 2), t(3, 4, 5)];
    (vertices, faces)
}

fn mixed_expected() -> Vec<TopologyFinding> {
    vec![
        IndexDegenerateTriangle { triangle: 1 },
        GeometricallyDegenerateTriangle { triangle: 4 },
        DuplicateFace { triangle: 3, first: 0 },
        FlippedDuplicateFace { triangle: 2, first: 0 },
        BoundaryEdge { edge: [3, 4], triangle: 4 },
        BoundaryEdge { edge: [3, 5], triangle: 4 },
        BoundaryEdge { edge: [4, 5], triangle: 4 },
        NonManifoldEdge { edge: [0, 1], uses: 3, first_triangle: 0 },
        NonManifoldEdge { edge: [0, 2], uses: 3, first_triangle: 0 },
        NonManifoldEdge { edge: [1, 2], uses: 3, first_triangle: 0 },
    ]
}

#[wasm_bindgen_test(unsupported = test)]
fn mtopo_tp_010_records_come_in_kind_then_field_order_with_fixed_severities() {
    use TopologySeverity::{Error, Warning};
    let (vertices, faces) = mixed();
    let found = run(&vertices, &faces);
    assert_eq!(found, mixed_expected());
    assert_eq!(found, oracle(&vertices, &faces));
    assert!(found.windows(2).all(|w| w[0] < w[1]), "strictly increasing");

    let severities: Vec<_> = TopologyKind::ALL.iter().map(|k| (*k, k.severity())).collect();
    assert_eq!(
        severities,
        vec![
            (TopologyKind::IndexDegenerateTriangle, Error),
            (TopologyKind::GeometricallyDegenerateTriangle, Warning),
            (TopologyKind::DuplicateFace, Error),
            (TopologyKind::FlippedDuplicateFace, Warning),
            (TopologyKind::BoundaryEdge, Warning),
            (TopologyKind::WindingInconsistentEdge, Warning),
            (TopologyKind::NonManifoldEdge, Warning),
        ]
    );
    let (_, report) = run_layout(&vertices, IndexBufferLayout::Width32(&faces));
    assert_eq!((report.error_count(), report.warning_count()), (2, 8));
    let (_, clean) = run_layout(&tetra_vertices(), IndexBufferLayout::Width32(&tetra_faces()));
    assert_eq!((clean.error_count(), clean.warning_count()), (0, 0));
}

#[wasm_bindgen_test(unsupported = test)]
fn mtopo_tp_011_a_short_output_buffer_keeps_the_ordered_prefix_and_reports_the_total() {
    let (vertices, faces) = mixed();
    let expected = mixed_expected();
    let mut face_slots = vec![FaceSlot::EMPTY; faces.len()];
    let mut edge_slots = vec![EdgeSlot::EMPTY; 3 * faces.len()];
    for len in 0..=expected.len() + 2 {
        let mut out = vec![FILL; len];
        let scratch = TopologyScratch::new(&mut face_slots, &mut edge_slots);
        let report =
            validate_topology(&vertices, IndexBufferLayout::Width32(&faces), scratch, &mut out)
                .expect("valid input");
        let written = len.min(expected.len());
        assert_eq!(report.written(), written, "len={len}");
        assert_eq!(report.total(), 10, "len={len}");
        assert_eq!(report.is_complete(), len >= expected.len(), "len={len}");
        assert_eq!(&out[..written], &expected[..written], "len={len}");
        assert!(out[written..].iter().all(|f| *f == FILL), "untouched past written; len={len}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn mtopo_tp_012_invalid_input_is_refused_in_a_fixed_order() {
    let vertices = tetra_vertices();
    let faces = tetra_faces();
    let bad = [t(0, 1, 4)];
    let bad16 = [Triangle16::new(0, 4, 1)];
    let mut out = [FILL; 4];
    let call =
        |indices: IndexBufferLayout<'_>, faces_len: usize, edges_len: usize, out: &mut [_]| {
            let mut f = vec![FaceSlot::EMPTY; faces_len];
            let mut e = vec![EdgeSlot::EMPTY; edges_len];
            validate_topology(&vertices, indices, TopologyScratch::new(&mut f, &mut e), out)
        };
    // The existing index check runs first, before any scratch size check.
    assert_eq!(
        call(IndexBufferLayout::Width32(&bad), 0, 0, &mut out),
        Err(TopologyError::IndexOutOfRange)
    );
    assert_eq!(
        call(IndexBufferLayout::Width16(&bad16), 0, 0, &mut out),
        Err(TopologyError::IndexOutOfRange)
    );
    assert!(!IndexBufferLayout::Width32(&bad).is_valid(vertices.len()));
    let w = IndexBufferLayout::Width32(&faces);
    // Face scratch is checked before edge scratch; one slot short of either is refused.
    assert_eq!(
        call(w, 3, 0, &mut out),
        Err(TopologyError::FaceScratchTooSmall { required: 4, provided: 3 })
    );
    assert_eq!(
        call(w, 4, 11, &mut out),
        Err(TopologyError::EdgeScratchTooSmall { required: 12, provided: 11 })
    );
    assert_eq!(out, [FILL; 4], "a refusal writes no record");
    assert!(call(w, 4, 12, &mut out).is_ok(), "exactly the documented sizes are enough");
    assert!(call(w, 5, 20, &mut out).is_ok(), "longer scratch is fine");
    assert_eq!(face_scratch_len(4), 4);
    assert_eq!(edge_scratch_len(4), Some(12));
    assert_eq!(edge_scratch_len(usize::MAX), None);
    let empty = call(IndexBufferLayout::Width16(&[]), 0, 0, &mut []).expect("empty mesh");
    assert_eq!((empty.total(), empty.written(), empty.is_complete()), (0, 0, true));
}

// --- Seeded generated meshes ---

fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state >> 33
}

fn pick(state: &mut u64, n: usize) -> usize {
    usize::try_from(next(state)).expect("31 bits") % n
}

fn coordinate(state: &mut u64) -> u16 {
    match pick(state, 4) {
        0 | 1 => u16::try_from(pick(state, 3)).expect("small"),
        2 => [0, u16::MAX][pick(state, 2)],
        _ => u16::try_from(next(state) & 0xFFFF).expect("16 bits"),
    }
}

fn random_vertices(state: &mut u64) -> Vec<QuantizedVertex> {
    let count = 3 + pick(state, 6);
    let mut vertices: Vec<QuantizedVertex> = Vec::with_capacity(count);
    for _ in 0..count {
        let vertex = match vertices.last() {
            Some(previous) if pick(state, 5) == 0 => *previous,
            _ => v(coordinate(state), coordinate(state), coordinate(state)),
        };
        vertices.push(vertex);
    }
    vertices
}

fn random_faces(state: &mut u64, vertex_count: usize) -> Vec<Triangle32> {
    let count = pick(state, 17);
    let index = |state: &mut u64| u32::try_from(pick(state, vertex_count)).expect("small");
    let mut faces: Vec<Triangle32> = Vec::with_capacity(count);
    for _ in 0..count {
        let earlier = (!faces.is_empty()).then(|| faces[pick(state, faces.len())]);
        let face = match (pick(state, 5), earlier) {
            (0, Some(f)) => t(f.v1, f.v2, f.v0),
            (1, Some(f)) => t(f.v2, f.v1, f.v0),
            (2, _) => {
                let a = index(state);
                t(a, a, index(state))
            }
            _ => t(index(state), index(state), index(state)),
        };
        faces.push(face);
    }
    faces
}

#[wasm_bindgen_test(unsupported = test)]
fn mtopo_tp_013_seeded_meshes_match_the_oracle_bit_for_bit() {
    let mut state = TOPOLOGY_SEED;
    let mut seen = [0_u64; 7];
    for case in 0..TOPOLOGY_CASES {
        let vertices = random_vertices(&mut state);
        let faces = random_faces(&mut state, vertices.len());
        let found = run(&vertices, &faces);
        assert_eq!(found, oracle(&vertices, &faces), "seed={TOPOLOGY_SEED:#018x}; case={case}");
        for finding in &found {
            let slot = TopologyKind::ALL.iter().position(|k| *k == finding.kind()).expect("kind");
            seen[slot] += 1;
        }
    }
    // Every kind is exercised many times by the 2 000 cases.
    assert!(seen.iter().all(|n| *n >= 100), "seed={TOPOLOGY_SEED:#018x}; seen={seen:?}");
}

#[wasm_bindgen_test(unsupported = test)]
fn mtopo_tp_014_output_does_not_depend_on_scratch_contents_on_entry() {
    let (mixed_vertices, mixed_faces) = mixed();
    let mut face_slots = vec![FaceSlot::EMPTY; 64];
    let mut edge_slots = vec![EdgeSlot::EMPTY; 192];
    let mut out = vec![FILL; 7 * 64];
    let mut state = TOPOLOGY_SEED ^ 0x5C7A_7C11;
    for case in 0..SCRATCH_CASES {
        // Leave junk from another mesh in the scratch, then permute it.
        let scratch = TopologyScratch::new(&mut face_slots, &mut edge_slots);
        validate_topology(
            &mixed_vertices,
            IndexBufferLayout::Width32(&mixed_faces),
            scratch,
            &mut out,
        )
        .expect("valid input");
        face_slots.reverse();
        edge_slots.rotate_left(1 + case % 37);
        let vertices = random_vertices(&mut state);
        let faces = random_faces(&mut state, vertices.len());
        let scratch = TopologyScratch::new(&mut face_slots, &mut edge_slots);
        let report =
            validate_topology(&vertices, IndexBufferLayout::Width32(&faces), scratch, &mut out)
                .expect("valid input");
        assert_eq!(&out[..report.written()], run(&vertices, &faces).as_slice(), "case={case}");
    }
}

// --- Independent oracle (allocates; shares no code with the kernel) ---

/// The parity of the permutation that sorts three distinct indices, by counting inversions.
fn odd_permutation([a, b, c]: [u32; 3]) -> bool {
    let inversions = u8::from(a > b) + u8::from(a > c) + u8::from(b > c);
    inversions % 2 == 1
}

fn collinear(vertices: &[QuantizedVertex], corners: [u32; 3]) -> bool {
    let p: Vec<[i128; 3]> = corners
        .iter()
        .map(|&i| {
            let q = vertices[usize::try_from(i).expect("index")];
            [i128::from(q.x), i128::from(q.y), i128::from(q.z)]
        })
        .collect();
    let a: Vec<i128> = (0..3).map(|k| p[0][k] - p[1][k]).collect();
    let b: Vec<i128> = (0..3).map(|k| p[2][k] - p[1][k]).collect();
    (0..3).all(|k| a[(k + 1) % 3] * b[(k + 2) % 3] == a[(k + 2) % 3] * b[(k + 1) % 3])
}

fn oracle(vertices: &[QuantizedVertex], faces: &[Triangle32]) -> Vec<TopologyFinding> {
    let mut out = Vec::new();
    let mut sets: BTreeMap<Vec<u32>, Vec<(u32, bool)>> = BTreeMap::new();
    let mut edges: BTreeMap<(u32, u32), Vec<(u32, u32)>> = BTreeMap::new();
    for (index, face) in faces.iter().enumerate() {
        let triangle = u32::try_from(index).expect("small");
        let corners = [face.v0, face.v1, face.v2];
        let set: BTreeSet<u32> = corners.into_iter().collect();
        if set.len() < 3 {
            out.push(IndexDegenerateTriangle { triangle });
            continue;
        }
        if collinear(vertices, corners) {
            out.push(GeometricallyDegenerateTriangle { triangle });
        }
        sets.entry(set.into_iter().collect())
            .or_default()
            .push((triangle, odd_permutation(corners)));
        for (from, to) in [(face.v0, face.v1), (face.v1, face.v2), (face.v2, face.v0)] {
            edges.entry((from.min(to), from.max(to))).or_default().push((triangle, from));
        }
    }
    for uses in sets.values() {
        let (first, first_odd) = uses[0];
        for &(triangle, odd) in &uses[1..] {
            out.push(if odd == first_odd {
                DuplicateFace { triangle, first }
            } else {
                FlippedDuplicateFace { triangle, first }
            });
        }
    }
    for (&(lo, hi), uses) in &edges {
        let edge = [lo, hi];
        match uses.as_slice() {
            [(triangle, _)] => out.push(BoundaryEdge { edge, triangle: *triangle }),
            [(a, from_a), (b, from_b)] if from_a == from_b => {
                out.push(WindingInconsistentEdge { edge, triangles: [*a, *b] });
            }
            [_, _] => {}
            _ => out.push(NonManifoldEdge {
                edge,
                uses: u32::try_from(uses.len()).expect("small"),
                first_triangle: uses[0].0,
            }),
        }
    }
    out.sort();
    out
}
