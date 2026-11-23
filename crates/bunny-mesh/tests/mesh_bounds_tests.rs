//! `mesh_bounds`: the box of the vertices a mesh's triangles use (flyingrobots/bunny#151).
//!
//! The oracle gathers the indexed vertices into a list and takes each axis's minimum and
//! maximum with the standard library; it shares no code with the kernel.

use bunny_linalg::FixedVec3;
use bunny_mesh::{mesh_bounds, IndexBufferLayout, MeshBoundsError, Triangle16, Triangle32};
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const SEED: u64 = 0xB015_1000_0000_0002;
const CASES: usize = 2_000;

type Raw3 = [i64; 3];

fn v(raw: Raw3) -> FixedVec3 {
    FixedVec3::new(
        FixedQ32_32::from_raw(raw[0]),
        FixedQ32_32::from_raw(raw[1]),
        FixedQ32_32::from_raw(raw[2]),
    )
}

fn raw(p: FixedVec3) -> Raw3 {
    [p.x.raw(), p.y.raw(), p.z.raw()]
}

fn got(
    vertices: &[FixedVec3],
    indices: IndexBufferLayout<'_>,
) -> Result<(Raw3, Raw3), MeshBoundsError> {
    mesh_bounds(vertices, indices).map(|b| (raw(b.min), raw(b.max)))
}

fn oracle(vertices: &[Raw3], faces: &[[u32; 3]]) -> Result<(Raw3, Raw3), MeshBoundsError> {
    if faces.is_empty() {
        return Err(MeshBoundsError::EmptyMesh);
    }
    let mut used = Vec::new();
    for (triangle, face) in faces.iter().enumerate() {
        for &index in face {
            let vertex = usize::try_from(index).ok().and_then(|i| vertices.get(i));
            used.push(*vertex.ok_or(MeshBoundsError::IndexOutOfRange { triangle })?);
        }
    }
    let lo = [0, 1, 2].map(|a| used.iter().map(|p| p[a]).min().expect("non-empty"));
    let hi = [0, 1, 2].map(|a| used.iter().map(|p| p[a]).max().expect("non-empty"));
    Ok((lo, hi))
}

#[wasm_bindgen_test(unsupported = test)]
fn bounds_tp_010_mesh_bounds_hand_derived() {
    let vertices = [v([0, 0, 0]), v([4, 0, 0]), v([0, 3, 0]), v([0, 0, -2]), v([100, 100, 100])];
    let none16: [Triangle16; 0] = [];
    let none32: [Triangle32; 0] = [];
    assert_eq!(
        got(&vertices, IndexBufferLayout::Width16(&none16)),
        Err(MeshBoundsError::EmptyMesh)
    );
    assert_eq!(got(&[], IndexBufferLayout::Width32(&none32)), Err(MeshBoundsError::EmptyMesh));
    let one = [Triangle16::new(0, 1, 2)];
    let want = Ok(([0, 0, 0], [4, 3, 0]));
    assert_eq!(got(&vertices, IndexBufferLayout::Width16(&one)), want);
    // Vertex 4, far away, is in the buffer but used by no triangle: it is not covered.
    let two = [Triangle32::new(0, 1, 2), Triangle32::new(2, 3, 0)];
    assert_eq!(got(&vertices, IndexBufferLayout::Width32(&two)), Ok(([0, 0, -2], [4, 3, 0])));
    // A degenerate triangle (one vertex three times) bounds that point.
    let point = [Triangle16::new(3, 3, 3)];
    assert_eq!(got(&vertices, IndexBufferLayout::Width16(&point)), Ok(([0, 0, -2], [0, 0, -2])));
    // Out of range: index 5 is one past the last vertex; the first bad triangle is named.
    let bad = [Triangle16::new(0, 1, 2), Triangle16::new(0, 5, 1), Triangle16::new(9, 9, 9)];
    let err = Err(MeshBoundsError::IndexOutOfRange { triangle: 1 });
    assert_eq!(got(&vertices, IndexBufferLayout::Width16(&bad)), err);
    let last = [Triangle32::new(4, 4, 4)];
    assert_eq!(got(&vertices, IndexBufferLayout::Width32(&last)), Ok(([100; 3], [100; 3])));
    let past = [Triangle32::new(0, 1, u32::MAX)];
    let err = Err(MeshBoundsError::IndexOutOfRange { triangle: 0 });
    assert_eq!(got(&vertices, IndexBufferLayout::Width32(&past)), err);
    assert_eq!(got(&[], IndexBufferLayout::Width16(&one)), err, "no vertices at all");
    // The whole raw range, exact.
    let ends = [v([i64::MIN; 3]), v([i64::MAX; 3]), v([0; 3])];
    let all = [Triangle16::new(0, 1, 2)];
    assert_eq!(got(&ends, IndexBufferLayout::Width16(&all)), Ok(([i64::MIN; 3], [i64::MAX; 3])));
}

fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(0x5851_F42D_4C95_7F2D).wrapping_add(0x1405_7B7E_F767_814F);
    *state >> 11 ^ *state << 53 ^ *state
}

fn raw_any(state: &mut u64) -> i64 {
    let shift = next(state) % 64;
    i64::from_ne_bytes(next(state).to_ne_bytes()) >> shift
}

fn below(state: &mut u64, n: u64) -> u32 {
    u32::try_from(next(state) % n).expect("small")
}

#[wasm_bindgen_test(unsupported = test)]
fn bounds_tp_011_seeded_mesh_bounds_match_the_oracle() {
    let mut state = SEED;
    let (mut ok, mut out_of_range, mut empty) = (0_usize, 0_usize, 0_usize);
    for case in 0..CASES {
        let n_vertices = below(&mut state, 12);
        let vertices: Vec<Raw3> =
            (0..n_vertices).map(|_| [0; 3].map(|_| raw_any(&mut state))).collect();
        let n_faces = below(&mut state, 6);
        // One face in sixteen draws its indices up to two past the end, so some meshes refer
        // outside the buffer; an empty buffer makes every index out of range.
        let faces: Vec<[u32; 3]> = (0..n_faces)
            .map(|_| {
                let past = if next(&mut state).is_multiple_of(16) { 2 } else { 0 };
                let span = (u64::from(n_vertices) + past).max(1);
                [0; 3].map(|_| below(&mut state, span))
            })
            .collect();
        let fixed: Vec<FixedVec3> = vertices.iter().map(|p| v(*p)).collect();
        let want = oracle(&vertices, &faces);
        let ctx = format!("seed={SEED:#x} case={case} vertices={vertices:?} faces={faces:?}");
        let wide: Vec<Triangle32> =
            faces.iter().map(|f| Triangle32::new(f[0], f[1], f[2])).collect();
        assert_eq!(got(&fixed, IndexBufferLayout::Width32(&wide)), want, "32; {ctx}");
        let narrow: Vec<Triangle16> = faces
            .iter()
            .map(|f| f.map(|i| u16::try_from(i).expect("small")))
            .map(|f| Triangle16::new(f[0], f[1], f[2]))
            .collect();
        assert_eq!(got(&fixed, IndexBufferLayout::Width16(&narrow)), want, "16; {ctx}");
        match want {
            Ok(_) => ok += 1,
            Err(MeshBoundsError::IndexOutOfRange { .. }) => out_of_range += 1,
            Err(MeshBoundsError::EmptyMesh) => empty += 1,
        }
    }
    println!(
        "seed={SEED:#x}: {CASES} meshes; ok {ok}, index out of range {out_of_range}, empty {empty}"
    );
    assert_eq!((ok, out_of_range, empty), EXPECTED_MESH_COUNTS, "seed={SEED:#x}");
}

/// Of the 2 000 seeded meshes: bounded, refused for an index, refused as empty.
const EXPECTED_MESH_COUNTS: (usize, usize, usize) = (1_397, 291, 312);
