//! CHARACTERIZATION tests for flyingrobots/bunny#116: what each public `bunny-mesh` function
//! does TODAY with degenerate input.
//!
//! These tests pin current behaviour, whatever it is. They are not an endorsed contract: a
//! cell pinned here can be inconsistent with a neighbouring cell (quantization reads an
//! inverted or zero-size range as "everything is the minimum", where the bounds helpers refuse
//! an inverted box). Changing a pinned cell is a behaviour change to a public function, which
//! the repository owner decides (see `docs/topics/degeneracy/README.md`); update the test and
//! the inventory together.

use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_mesh::{
    compute_mesh_hash, dequantize_scalar, edge_scratch_len, face_scratch_len, mesh_bounds,
    quantize_scalar, quantize_vertex, validate_topology, EdgeSlot, FaceSlot, IndexBufferLayout,
    MeshBoundsError, QuantizedVertex, TopologyFinding, TopologyKind, TopologyScratch, Triangle32,
};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const MAX: i64 = i64::MAX;
const MIN: i64 = i64::MIN;
const ONE: i64 = 1 << 32;

fn q(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

fn v(raw: [i64; 3]) -> FixedVec3 {
    FixedVec3::new(q(raw[0]), q(raw[1]), q(raw[2]))
}

/// `mesh_bounds` refuses a mesh with no triangles and accepts degenerate ones.
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_060_characterization_mesh_bounds() {
    let vertices = [v([0; 3]), v([ONE, 0, 0]), v([2 * ONE, 0, 0])];
    assert_eq!(
        mesh_bounds(&vertices, IndexBufferLayout::Width32(&[])),
        Err(MeshBoundsError::EmptyMesh)
    );
    let collinear = [Triangle32::new(0, 1, 2), Triangle32::new(1, 1, 1)];
    let bounds = mesh_bounds(&vertices, IndexBufferLayout::Width32(&collinear));
    assert_eq!(bounds, Ok(FixedAabb3::new(v([0; 3]), v([2 * ONE, 0, 0]))));
}

/// Quantization: a zero-size or inverted range maps every value to 0 and dequantizes every
/// code to `min`; the full raw range is exact at both ends.
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_061_characterization_quantization_ranges() {
    for (min, max) in [(q(5), q(5)), (q(10), q(0))] {
        for value in [q(MIN), q(0), q(5), q(10), q(MAX)] {
            assert_eq!(quantize_scalar(value, min, max), 0);
        }
        assert_eq!(dequantize_scalar(u16::MAX, min, max), min);
    }
    assert_eq!(quantize_scalar(q(MAX), q(MIN), q(MAX)), u16::MAX);
    assert_eq!(quantize_scalar(q(MIN), q(MIN), q(MAX)), 0);
    assert_eq!(dequantize_scalar(u16::MAX, q(MIN), q(MAX)), q(MAX));
    let inverted = FixedAabb3::new(v([ONE; 3]), v([0; 3]));
    assert_eq!(quantize_vertex(v([ONE / 2; 3]), &inverted), QuantizedVertex::new(0, 0, 0));
}

/// `validate_topology` accepts an empty mesh with an empty report and reports repeated
/// indices and collinear positions as findings, never as errors.
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_062_characterization_topology_degenerates() {
    let vertices = [
        QuantizedVertex::new(0, 0, 0),
        QuantizedVertex::new(1, 0, 0),
        QuantizedVertex::new(2, 0, 0),
    ];
    let mut faces = [FaceSlot::EMPTY; 4];
    let mut edges = [EdgeSlot::EMPTY; 12];
    let mut findings = [TopologyFinding::IndexDegenerateTriangle { triangle: u32::MAX }; 16];
    let empty = IndexBufferLayout::Width32(&[]);
    let scratch = TopologyScratch::new(&mut faces, &mut edges);
    let report = validate_topology(&vertices, empty, scratch, &mut findings).expect("accepted");
    assert_eq!((report.total(), report.is_complete()), (0, true));

    let triangles = [Triangle32::new(0, 0, 1), Triangle32::new(0, 1, 2)];
    let layout = IndexBufferLayout::Width32(&triangles);
    assert!(face_scratch_len(2) <= faces.len());
    assert!(edge_scratch_len(2).is_some_and(|n| n <= edges.len()));
    let scratch = TopologyScratch::new(&mut faces, &mut edges);
    let report = validate_topology(&vertices, layout, scratch, &mut findings).expect("accepted");
    assert_eq!(report.count(TopologyKind::IndexDegenerateTriangle), 1);
    assert_eq!(report.count(TopologyKind::GeometricallyDegenerateTriangle), 1);
}

/// `compute_mesh_hash` hashes whatever it is given: an empty mesh and inverted bounds are
/// hashed, not refused, and an inverted box hashes differently from its swap.
#[wasm_bindgen_test(unsupported = test)]
fn degen_tp_063_characterization_mesh_hash_inputs() {
    let ordered = FixedAabb3::new(v([0; 3]), v([ONE; 3]));
    let inverted = FixedAabb3::new(v([ONE; 3]), v([0; 3]));
    let empty = IndexBufferLayout::Width32(&[]);
    assert_ne!(compute_mesh_hash(&[], empty, &ordered), compute_mesh_hash(&[], empty, &inverted));
}
