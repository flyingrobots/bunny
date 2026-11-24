//! Integration test: topology validation (`bunny-mesh-topology/1`) of the Stanford bunny.
//!
//! The asset is read with `parse_ascii_ply`, quantized to `u16` over its exact bounds with
//! `quantize_vertex`, and validated. The expected findings come from an independent oracle
//! written in this file (`BTreeMap` and `BTreeSet`, `i128`, permutation parity), sharing no
//! code with the kernel; the counts per kind are pinned.

use std::collections::{BTreeMap, BTreeSet};

use bunny_codec::parse_ascii_ply;
use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_mesh::{
    edge_scratch_len, face_scratch_len, quantize_vertex, validate_topology, EdgeSlot, FaceSlot,
    IndexBufferLayout, QuantizedVertex, TopologyFinding, TopologyKind, TopologyScratch, Triangle32,
};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

use TopologyFinding::{
    BoundaryEdge, DuplicateFace, FlippedDuplicateFace, GeometricallyDegenerateTriangle,
    IndexDegenerateTriangle, NonManifoldEdge, WindingInconsistentEdge,
};

const ASSET: &[u8] = include_bytes!("fixtures/stanford-bunny-ascii/bun_zipper_res3.ply");
const ASSET_LEN: usize = 142_784;

fn bunny() -> (Vec<QuantizedVertex>, Vec<Triangle32>) {
    assert_eq!(ASSET.len(), ASSET_LEN, "fixture bytes were rewritten");
    let mesh = parse_ascii_ply(ASSET).expect("asset should parse");
    let points: Vec<FixedVec3> = mesh
        .vertices()
        .map(|vertex| {
            let vertex = vertex.expect("validated vertex");
            FixedVec3::new(vertex.x, vertex.y, vertex.z)
        })
        .collect();
    let axis = |pick: fn(&FixedVec3) -> FixedQ32_32| {
        let raws = points.iter().map(|p| pick(p).raw());
        (raws.clone().min().expect("vertices"), raws.max().expect("vertices"))
    };
    let (x, y, z) = (axis(|p| p.x), axis(|p| p.y), axis(|p| p.z));
    let raw = FixedQ32_32::from_raw;
    let bounds = FixedAabb3::new(
        FixedVec3::new(raw(x.0), raw(y.0), raw(z.0)),
        FixedVec3::new(raw(x.1), raw(y.1), raw(z.1)),
    );
    let vertices = points.iter().map(|p| quantize_vertex(*p, &bounds)).collect();
    let faces = mesh.triangles().map(|face| face.expect("validated triangle")).collect();
    (vertices, faces)
}

fn kernel(vertices: &[QuantizedVertex], faces: &[Triangle32]) -> Vec<TopologyFinding> {
    let mut face_slots = vec![FaceSlot::EMPTY; face_scratch_len(faces.len())];
    let mut edge_slots = vec![EdgeSlot::EMPTY; edge_scratch_len(faces.len()).expect("fits")];
    let mut out = vec![IndexDegenerateTriangle { triangle: u32::MAX }; 5 * faces.len()];
    let scratch = TopologyScratch::new(&mut face_slots, &mut edge_slots);
    let report = validate_topology(vertices, IndexBufferLayout::Width32(faces), scratch, &mut out)
        .expect("valid input");
    assert!(report.is_complete());
    out.truncate(report.written());
    out
}

#[wasm_bindgen_test(unsupported = test)]
fn mtopo_tp_015_stanford_bunny_findings_match_the_oracle_and_pinned_counts() {
    let (vertices, faces) = bunny();
    assert_eq!((vertices.len(), faces.len()), (1889, 3851));
    let found = kernel(&vertices, &faces);
    assert_eq!(found, oracle(&vertices, &faces));
    let counts: Vec<usize> = TopologyKind::ALL
        .iter()
        .map(|kind| found.iter().filter(|finding| finding.kind() == *kind).count())
        .collect();
    assert_eq!(counts, PINNED_COUNTS, "{found:?}");
}

/// Index-degenerate, geometrically degenerate, duplicate, flipped duplicate, boundary,
/// winding-inconsistent and non-manifold, in that order. A separate Python count over the raw
/// PLY face list (posted on the pull request) gives the same six combinatorial counts.
const PINNED_COUNTS: [usize; 7] = [0, 0, 10, 73, 60, 0, 141];

// --- Independent oracle (allocates; shares no code with the kernel) ---

fn odd_permutation([a, b, c]: [u32; 3]) -> bool {
    (u8::from(a > b) + u8::from(a > c) + u8::from(b > c)) % 2 == 1
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
