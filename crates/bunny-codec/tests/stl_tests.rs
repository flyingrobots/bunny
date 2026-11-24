//! Constructed cases for `parse_binary_stl` (`bunny-binary-stl-triangle-mesh/1`): accepted
//! meshes, every length refusal, every vertex coordinate position refused for each non-finite
//! value and for the Q32.32 range, and the conversion at its boundaries. Expected raw values
//! are derived by hand from the binary32 bit patterns in `stl_support`.

mod stl_support;

use bunny_codec::{
    parse_binary_stl, StlError, StlMesh, StlVertex, BINARY_STL_MAX_TRIANGLES, BINARY_STL_PROFILE_ID,
};
use bunny_mesh::Triangle32;
use bunny_num::FixedQ32_32;
use stl_support::{
    header, stl, stl_with_count, tetrahedron, Facet, EIGHTH, HALF, MINUS_TWO_AND_A_QUARTER,
    NON_FINITE, ONE, RAW_ONE,
};
use wasm_bindgen_test::wasm_bindgen_test;

fn vertex(x: i64, y: i64, z: i64) -> StlVertex {
    StlVertex {
        x: FixedQ32_32::from_raw(x),
        y: FixedQ32_32::from_raw(y),
        z: FixedQ32_32::from_raw(z),
    }
}

fn vertices(mesh: &StlMesh<'_>) -> Vec<StlVertex> {
    mesh.vertices().collect::<Result<_, _>>().expect("a parsed mesh yields only Ok vertices")
}

fn triangles(mesh: &StlMesh<'_>) -> Vec<Triangle32> {
    mesh.triangles().collect::<Result<_, _>>().expect("a parsed mesh yields only Ok triangles")
}

fn expected_len(count: u32) -> u64 {
    84 + 50 * u64::from(count)
}

fn mismatch(count: u32, actual: usize) -> StlError {
    StlError::LengthMismatch {
        triangles: count,
        expected: expected_len(count),
        actual: u64::try_from(actual).expect("test inputs are small"),
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn stl_tp_001_profile_has_a_stable_identity() {
    assert_eq!(BINARY_STL_PROFILE_ID, "bunny-binary-stl-triangle-mesh/1");
    assert_eq!(BINARY_STL_MAX_TRIANGLES, 1_431_655_765);
    assert_eq!(u64::from(BINARY_STL_MAX_TRIANGLES) * 3, u64::from(u32::MAX));
}

#[wasm_bindgen_test(unsupported = test)]
fn stl_tp_002_tetrahedron_reads_in_file_order_without_welding() {
    let bytes = stl(&tetrahedron());
    assert_eq!(bytes.len(), 284);
    assert_eq!(&bytes[80..84], &[4, 0, 0, 0]);
    let mesh = parse_binary_stl(&bytes).expect("tetrahedron");
    assert_eq!((mesh.triangle_count(), mesh.vertex_count()), (4, 12));
    let (o, x, y, z) =
        (vertex(0, 0, 0), vertex(RAW_ONE, 0, 0), vertex(0, RAW_ONE, 0), vertex(0, 0, RAW_ONE));
    // Twelve vertices, three per facet in stored order: shared corners are not welded.
    assert_eq!(vertices(&mesh), [o, y, x, o, x, z, o, z, y, x, y, z]);
    let expected: Vec<Triangle32> =
        (0..4).map(|i| Triangle32::new(3 * i, 3 * i + 1, 3 * i + 2)).collect();
    assert_eq!(triangles(&mesh), expected);
}

#[wasm_bindgen_test(unsupported = test)]
fn stl_tp_003_coordinates_convert_exactly() {
    let facet = Facet::new([[HALF, MINUS_TWO_AND_A_QUARTER, EIGHTH], [ONE, HALF, 0], [0, 0, 0]]);
    let mesh = parse_binary_stl(&stl(&[facet])).expect("one facet");
    let got = vertices(&mesh);
    assert_eq!(got[0], vertex(1 << 31, -(9 << 30), 1 << 29));
    assert_eq!(got[1], vertex(RAW_ONE, 1 << 31, 0));
}

#[wasm_bindgen_test(unsupported = test)]
fn stl_tp_004_zero_triangles_is_an_empty_mesh() {
    let bytes = stl(&[]);
    assert_eq!(bytes.len(), 84);
    let mesh = parse_binary_stl(&bytes).expect("empty mesh");
    assert_eq!((mesh.triangle_count(), mesh.vertex_count()), (0, 0));
    assert_eq!(mesh.vertices().count(), 0);
    assert_eq!(mesh.triangles().count(), 0);
}

#[wasm_bindgen_test(unsupported = test)]
fn stl_tp_005_the_header_is_not_interpreted() {
    let facets = tetrahedron();
    let reference = parse_binary_stl(&stl(&facets)).map(|m| (vertices(&m), triangles(&m)));
    let mut all_ones = [0xFF_u8; 80];
    all_ones[0] = 0;
    for text in [header(b"solid tetrahedron"), header(b"solid"), header(b""), all_ones] {
        let bytes = stl_with_count(&text, 4, &facets);
        let mesh = parse_binary_stl(&bytes).expect("binary STL whatever the header says");
        assert_eq!(Ok((vertices(&mesh), triangles(&mesh))), reference);
    }
    // ASCII STL is not part of the profile and is not detected: it is read as binary, and
    // this one fails the length rule because bytes 80..84 are text.
    let ascii =
        b"solid t\n facet normal 0 0 1\n  outer loop\n   vertex 0 0 0\n   vertex 1 0 0\n   \
vertex 0 1 0\n  endloop\n endfacet\nendsolid t\n";
    let count = u32::from_le_bytes(ascii[80..84].try_into().expect("four bytes"));
    assert_eq!(
        parse_binary_stl(ascii).map(|m| m.triangle_count()),
        Err(mismatch(count, ascii.len()))
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn stl_tp_006_stored_normals_and_attributes_are_ignored() {
    let facets = tetrahedron();
    let reference = parse_binary_stl(&stl(&facets)).map(|m| (vertices(&m), triangles(&m)));
    let normals =
        [[0, 0, 0], [ONE, 0, 0], [0x7FC0_0000, 0x7F80_0000, 0xFF80_0000], [0xFFFF_FFFF; 3]];
    for normal in normals {
        for attribute in [0, 1, 0x7FFF, 0xFFFF] {
            let changed = facets.map(|f| Facet { normal, attribute, ..f });
            let mesh =
                parse_binary_stl(&stl(&changed)).expect("normals and attributes are ignored");
            assert_eq!(Ok((vertices(&mesh), triangles(&mesh))), reference);
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn stl_tp_007_winding_is_kept_and_degenerate_triangles_are_kept() {
    let (a, b, c) = ([0, 0, 0], [ONE, 0, 0], [0, ONE, 0]);
    // Clockwise against an outward +z normal: not reordered from the normal.
    let reversed = Facet { normal: [0, 0, ONE], vertices: [a, c, b], attribute: 0 };
    let point = Facet::new([b, b, b]);
    let collinear = Facet::new([a, b, [ONE | 0x0080_0000, 0, 0]]);
    let mesh = parse_binary_stl(&stl(&[reversed, point, collinear])).expect("degenerates are kept");
    let (va, vb, vc) = (vertex(0, 0, 0), vertex(RAW_ONE, 0, 0), vertex(0, RAW_ONE, 0));
    let two = vertex(2 * RAW_ONE, 0, 0);
    assert_eq!(vertices(&mesh), [va, vc, vb, vb, vb, vb, va, vb, two]);
    assert_eq!(mesh.triangle_count(), 3);
}

#[wasm_bindgen_test(unsupported = test)]
fn stl_tp_008_input_shorter_than_the_header_and_count_is_too_short() {
    for length in [0_usize, 1, 79, 80, 83] {
        let bytes = vec![0_u8; length];
        assert_eq!(
            parse_binary_stl(&bytes).map(|m| m.triangle_count()),
            Err(StlError::TooShort { length })
        );
    }
    assert!(parse_binary_stl(&[0_u8; 84]).is_ok());
}

#[wasm_bindgen_test(unsupported = test)]
fn stl_tp_009_the_length_must_be_exactly_84_plus_50_per_triangle() {
    let good = stl(&tetrahedron());
    let refused = |bytes: &[u8]| parse_binary_stl(bytes).map(|m| m.triangle_count());
    for cut in [1, 2, 25, 49, 50, 51, 199] {
        let short = &good[..good.len() - cut];
        assert_eq!(refused(short), Err(mismatch(4, short.len())), "truncated by {cut}");
    }
    for extra in [1_usize, 2, 49, 50, 51] {
        let mut long = good.clone();
        long.extend(std::iter::repeat_n(0_u8, extra));
        assert_eq!(refused(&long), Err(mismatch(4, long.len())), "{extra} trailing bytes");
    }
    let mut empty_plus_one = stl(&[]);
    empty_plus_one.push(0);
    assert_eq!(refused(&empty_plus_one), Err(mismatch(0, 85)));
    let facets = tetrahedron();
    for count in [0, 3, 5, 6, 100] {
        let bytes = stl_with_count(&header(b"x"), count, &facets);
        assert_eq!(refused(&bytes), Err(mismatch(count, 284)), "count {count}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn stl_tp_010_counts_at_the_overflow_boundaries_are_length_mismatches() {
    // 85_899_344 is the largest count whose length fits u32; one more needs 4_294_967_334
    // bytes. u32::MAX / 3 is the largest count whose vertex indices fit u32.
    let counts = [
        85_899_344,
        85_899_345,
        BINARY_STL_MAX_TRIANGLES - 1,
        BINARY_STL_MAX_TRIANGLES,
        BINARY_STL_MAX_TRIANGLES + 1,
        u32::MAX - 1,
        u32::MAX,
    ];
    assert_eq!(expected_len(85_899_344), 4_294_967_284);
    assert_eq!(expected_len(85_899_345), 4_294_967_334);
    assert_eq!(expected_len(u32::MAX), 214_748_364_834);
    for count in counts {
        for facets in [&[][..], &tetrahedron()[..1], &tetrahedron()[..]] {
            let bytes = stl_with_count(&header(b"huge"), count, facets);
            assert_eq!(
                parse_binary_stl(&bytes).map(|m| m.triangle_count()),
                Err(mismatch(count, bytes.len())),
                "count {count}"
            );
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn stl_tp_011_a_big_endian_count_is_read_little_endian() {
    let facets = tetrahedron();
    let mut bytes = stl(&facets[..1]);
    bytes[80..84].copy_from_slice(&1_u32.to_be_bytes());
    assert_eq!(
        parse_binary_stl(&bytes).map(|m| m.triangle_count()),
        Err(mismatch(16_777_216, 134))
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn stl_tp_012_every_non_finite_vertex_coordinate_is_refused_where_it_is() {
    let facets = tetrahedron();
    for bits in NON_FINITE {
        for position in 0..9_u8 {
            let mut changed = facets;
            let (vertex, axis) = (position / 3, position % 3);
            changed[1].vertices[usize::from(vertex)][usize::from(axis)] = bits;
            let expected = StlError::NonFiniteVertex { triangle: 1, vertex, axis };
            assert_eq!(
                parse_binary_stl(&stl(&changed)).map(|m| m.triangle_count()),
                Err(expected),
                "{bits:#x}"
            );
        }
    }
    // The first bad coordinate in file order is reported.
    let mut two_bad = facets;
    two_bad[3].vertices[0][0] = NON_FINITE[0];
    two_bad[2].vertices[2][1] = NON_FINITE[3];
    let expected = StlError::NonFiniteVertex { triangle: 2, vertex: 2, axis: 1 };
    assert_eq!(parse_binary_stl(&stl(&two_bad)).map(|m| m.triangle_count()), Err(expected));
}

#[wasm_bindgen_test(unsupported = test)]
fn stl_tp_013_coordinates_outside_q32_32_are_refused_where_they_are() {
    let facets = tetrahedron();
    let out_of_range = [0x4F00_0000, 0xCF00_0001, 0x7F7F_FFFF, 0xFF7F_FFFF, 0x5F00_0000];
    for bits in out_of_range {
        for position in 0..9_u8 {
            let mut changed = facets;
            let (vertex, axis) = (position / 3, position % 3);
            changed[3].vertices[usize::from(vertex)][usize::from(axis)] = bits;
            let expected = StlError::VertexOutOfRange { triangle: 3, vertex, axis };
            assert_eq!(
                parse_binary_stl(&stl(&changed)).map(|m| m.triangle_count()),
                Err(expected),
                "{bits:#x}"
            );
        }
    }
    // A non-finite value before an out-of-range one in file order wins, and the reverse.
    let mut both = facets;
    both[0].vertices[0][2] = 0x4F00_0000;
    both[0].vertices[1][0] = NON_FINITE[1];
    let expected = StlError::VertexOutOfRange { triangle: 0, vertex: 0, axis: 2 };
    assert_eq!(parse_binary_stl(&stl(&both)).map(|m| m.triangle_count()), Err(expected));
}

#[wasm_bindgen_test(unsupported = test)]
fn stl_tp_014_the_range_ends_and_ties_follow_try_from_f32() {
    // (bits, raw): the last values in range, ties to even, and values that round to zero.
    let cases: [(u32, i64); 12] = [
        (0x4EFF_FFFF, ((1_i64 << 31) - 128) << 32),
        (0xCF00_0000, i64::MIN),
        (0xCEFF_FFFF, -(((1_i64 << 31) - 128) << 32)),
        (0x2F00_0000, 0),
        (0xAF00_0000, 0),
        (0x2FC0_0000, 2),
        (0xAFC0_0000, -2),
        (0x3020_0000, 2),
        (0x3060_0000, 4),
        (0x0000_0001, 0),
        (0x8000_0000, 0),
        (0x2F00_0001, 1),
    ];
    for (bits, raw) in cases {
        let facet = Facet::new([[bits, 0, 0], [0, bits, 0], [0, 0, bits]]);
        let mesh = parse_binary_stl(&stl(&[facet])).expect("in range");
        assert_eq!(
            vertices(&mesh),
            [vertex(raw, 0, 0), vertex(0, raw, 0), vertex(0, 0, raw)],
            "{bits:#x}"
        );
        let inherited = FixedQ32_32::try_from_f32(f32_from(bits)).map(FixedQ32_32::raw);
        assert_eq!(inherited, Ok(raw), "the reader uses try_from_f32 unchanged: {bits:#x}");
    }
}

/// The binary32 value of `bits`, handed only to the conversion the reader inherits.
fn f32_from(bits: u32) -> f32 {
    f32::from_bits(bits)
}

#[wasm_bindgen_test(unsupported = test)]
fn stl_tp_015_errors_display_their_position_and_are_errors() {
    let cases = [
        (StlError::TooShort { length: 83 }, "STL input is 83 bytes, shorter than the 84-byte header and triangle count"),
        (
            mismatch(4, 283),
            "STL declares 4 triangles, which needs exactly 284 bytes; the input is 283 bytes",
        ),
        (
            StlError::TooManyTriangles { triangles: u32::MAX },
            "STL declares 4294967295 triangles; at most 1431655765 have vertex indices that fit u32",
        ),
        (
            StlError::NonFiniteVertex { triangle: 7, vertex: 2, axis: 1 },
            "STL triangle 7 vertex 2 coordinate y is not finite",
        ),
        (
            StlError::VertexOutOfRange { triangle: 0, vertex: 0, axis: 2 },
            "STL triangle 0 vertex 0 coordinate z is outside the Q32.32 range",
        ),
    ];
    for (error, text) in cases {
        assert_eq!(error.to_string(), text);
        let dynamic: &dyn core::error::Error = &error;
        assert!(dynamic.source().is_none());
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn stl_tp_016_iterators_restart_and_yield_exactly_the_counts() {
    let facets = tetrahedron();
    let bytes = stl(&facets[..3]);
    let mesh = parse_binary_stl(&bytes).expect("three facets");
    let first = vertices(&mesh);
    assert_eq!(first.len(), 9);
    assert_eq!(vertices(&mesh), first);
    assert_eq!(mesh.vertices().count(), mesh.vertex_count());
    assert_eq!(mesh.triangles().count(), mesh.triangle_count());
    let copy = mesh;
    assert_eq!(triangles(&copy), triangles(&mesh));
}
