//! Integration tests for the ASCII PLY triangle-mesh reader (`bunny-ascii-ply-triangle-mesh/1`).
//!
//! Expected raw coordinates are hand-derived or computed with exact integer arithmetic:
//! raw = round(value * 2^32), ties to even. No floating point is used.

use bunny_codec::{
    parse_ascii_ply, AsciiPlyError, AsciiPlyErrorKind as Kind, AsciiPlyVertex, ASCII_PLY_PROFILE_ID,
};
use bunny_mesh::Triangle32;
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const HEADER: &str = "ply\nformat ascii 1.0\ncomment made by hand\nelement vertex 4\n\
property float x\nproperty float y\nproperty float z\nproperty float confidence\n\
property float intensity\nelement face 2\nproperty list uchar int vertex_indices\nend_header\n";
const BODY: &str = "-0.0378297 0.12794 0.00447467 0.850855 0.5\n\
-0.0447794 0.128887 0.00190497 0.900159 0.7\n\
-0.0680095 0.151244 0.0371953 0.398443 0.1\n\
0.5 -12.5 2 1 1\n3 0 1 2\n3 2 1 3\n";

fn mesh_text() -> String {
    format!("{HEADER}{BODY}")
}

fn vertex(x: i64, y: i64, z: i64) -> AsciiPlyVertex {
    AsciiPlyVertex {
        x: FixedQ32_32::from_raw(x),
        y: FixedQ32_32::from_raw(y),
        z: FixedQ32_32::from_raw(z),
    }
}

/// Raw values of `BODY`'s four vertices, each `round(decimal * 2^32)` derived by hand:
/// -0.0378297 * 2^32 = -162477323.6.. -> -162477324, and likewise for the others;
/// 0.5 * 2^32 = 2147483648, -12.5 * 2^32 = -53687091200, 2 * 2^32 = 8589934592.
fn expected_vertices() -> [AsciiPlyVertex; 4] {
    [
        vertex(-162_477_324, 549_498_116, 19_218_561),
        vertex(-192_326_059, 553_565_450, 8_181_784),
        vertex(-292_098_578, 649_588_034, 159_752_597),
        vertex(2_147_483_648, -53_687_091_200, 8_589_934_592),
    ]
}

fn collect_vertices(text: &str) -> Vec<AsciiPlyVertex> {
    let mesh = parse_ascii_ply(text.as_bytes()).expect("mesh should parse");
    mesh.vertices().map(|item| item.expect("validated vertex")).collect()
}

fn collect_triangles(text: &str) -> Vec<Triangle32> {
    let mesh = parse_ascii_ply(text.as_bytes()).expect("mesh should parse");
    mesh.triangles().map(|item| item.expect("validated triangle")).collect()
}

fn refusal(text: &str) -> (Kind, usize) {
    refusal_bytes(text.as_bytes())
}

fn refusal_bytes(bytes: &[u8]) -> (Kind, usize) {
    let error: AsciiPlyError = parse_ascii_ply(bytes).expect_err("input should be refused");
    (error.kind(), error.line())
}

/// Replaces the one place `from` occurs, and fails loudly if it does not occur exactly once.
fn edit(text: &str, from: &str, to: &str) -> String {
    assert_eq!(text.matches(from).count(), 1, "{from:?} must occur exactly once");
    text.replace(from, to)
}

const ONE: i64 = 1 << 32;

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_001_geordi_style_mesh_skips_extra_properties_by_declaration() {
    assert_eq!(ASCII_PLY_PROFILE_ID, "bunny-ascii-ply-triangle-mesh/1");
    let text = mesh_text();
    let mesh = parse_ascii_ply(text.as_bytes()).expect("mesh should parse");
    assert_eq!((mesh.vertex_count(), mesh.face_count()), (4, 2));
    assert_eq!(collect_vertices(&text), expected_vertices());
    assert_eq!(collect_triangles(&text), [Triangle32::new(0, 1, 2), Triangle32::new(2, 1, 3)]);
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_002_coordinate_slots_follow_the_declared_order_and_types() {
    // Extra properties come first, in the middle and last; z, x, y are declared in that
    // order, with double and float64 spellings. The values 3, -3, 0.75 are exact.
    let text = "ply\nformat ascii 1.0\nelement vertex 1\nproperty uchar tag\nproperty double z\n\
property int id\nproperty float64 x\nproperty float32 y\nproperty short extra\n\
element face 0\nproperty list uint8 uint32 vertex_indices\nend_header\n9 3 7 -3 0.75 5\n";
    assert_eq!(collect_vertices(text), [vertex(-12_884_901_888, 3_221_225_472, 12_884_901_888)]);
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_003_lf_crlf_and_mixed_line_endings_are_accepted() {
    let lf = mesh_text();
    let crlf = lf.replace('\n', "\r\n");
    let mixed: String = lf
        .split_inclusive('\n')
        .enumerate()
        .map(
            |(index, line)| {
                if index % 2 == 0 {
                    line.replace('\n', "\r\n")
                } else {
                    line.to_owned()
                }
            },
        )
        .collect();
    for text in [&lf, &crlf, &mixed] {
        assert_eq!(collect_vertices(text), expected_vertices());
        assert_eq!(collect_triangles(text).len(), 2);
    }
    // The last face line may lack a terminator, with or without CRLF elsewhere.
    let unterminated = lf.trim_end_matches('\n');
    assert_eq!(collect_triangles(unterminated).len(), 2);
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_004_a_lone_carriage_return_is_not_whitespace_or_a_line_end() {
    // A CR inside a body line is part of a token, so the number is invalid (line 16).
    let body_cr = edit(&mesh_text(), "-12.5 2", "-12.5\r 2");
    assert_eq!(refusal(&body_cr), (Kind::InvalidNumber, 16));
    // A CR at the very end of the input has no LF behind it and stays in the last token.
    let tail_cr = format!("{}\r", mesh_text().trim_end_matches('\n'));
    assert_eq!(refusal(&tail_cr), (Kind::InvalidIndex, 18));
    // A CR in a header line that is not directly before its LF is part of a word.
    let header_cr = edit(&mesh_text(), "format ascii 1.0\n", "format ascii 1.0\r \n");
    assert_eq!(refusal(&header_cr), (Kind::UnsupportedFormat, 2));
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_005_blank_lines_comments_and_whitespace_follow_the_stated_rules() {
    // Header: blank lines, `comment` and `obj_info` lines, leading and trailing spaces and
    // tabs, and runs of separators are ignored. Body: blank and whitespace-only lines are
    // skipped before, between and after records; separators are runs of spaces or tabs.
    let text = "ply\t \n\nformat   ascii\t1.0 \t\n obj_info anything at all\ncomment\n\
\t comment indented\nelement vertex 2 \nproperty float x\nproperty  float y\t\nproperty float z\n\n\
element face 1\nproperty list uchar int vertex_indices\n  end_header \t\n\n\
 1\t 2   3  \n   \n\t\n4 5 6\n\n 3\t0 1\t1 \n\n   \n";
    assert_eq!(
        collect_vertices(text),
        [vertex(ONE, 2 * ONE, 3 * ONE), vertex(4 * ONE, 5 * ONE, 6 * ONE)]
    );
    assert_eq!(collect_triangles(text), [Triangle32::new(0, 1, 1)]);
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_006_body_comments_and_content_after_the_records_are_refused() {
    let body_comment = edit(&mesh_text(), "3 0 1 2\n", "comment hi\n3 0 1 2\n");
    assert_eq!(refusal(&body_comment), (Kind::InvalidIndex, 17));
    // Blank lines after the last record are fine, but a further record is not (line 21).
    let trailing = format!("{}\n\n3 0 1 2\n", mesh_text());
    assert_eq!(refusal(&trailing), (Kind::TrailingData, 21));
    // Extra content on the last face line is a face line of the wrong length.
    let junk = format!("{} x\n", mesh_text().trim_end_matches('\n'));
    assert_eq!(refusal(&junk), (Kind::FaceFieldCount, 18));
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_007_header_refusals_name_their_kind_and_line() {
    let base = mesh_text();
    let cases: [(String, Kind, usize); 22] = [
        (edit(&base, "ply\n", "PLY\n"), Kind::InvalidMagic, 1),
        (edit(&base, "ply\n", " ply\n"), Kind::InvalidMagic, 1),
        (edit(&base, "ply\n", "ply extra\n"), Kind::InvalidMagic, 1),
        (edit(&base, "ascii 1.0", "binary_little_endian 1.0"), Kind::UnsupportedFormat, 2),
        (edit(&base, "ascii 1.0", "ascii 1.1"), Kind::UnsupportedFormat, 2),
        (edit(&base, "ascii 1.0", "ascii"), Kind::UnsupportedFormat, 2),
        (edit(&base, "ascii 1.0\n", "ascii 1.0\nformat ascii 1.0\n"), Kind::UnsupportedFormat, 3),
        (edit(&base, "format ascii 1.0\n", ""), Kind::UnsupportedFormat, 3),
        (edit(&base, "comment made by hand", "bogus 1"), Kind::InvalidHeaderLine, 3),
        (edit(&base, "vertex 4", "vertex four"), Kind::InvalidCount, 4),
        (edit(&base, "vertex 4", "vertex +4"), Kind::InvalidCount, 4),
        (edit(&base, "vertex 4", "vertex -4"), Kind::InvalidCount, 4),
        (edit(&base, "vertex 4", "vertex"), Kind::InvalidCount, 4),
        (edit(&base, "vertex 4", "vertex 99999999999999999999999"), Kind::IntegerOverflow, 4),
        (edit(&base, "vertex 4", "vertex 4 5"), Kind::InvalidHeaderLine, 4),
        (
            edit(&base, "element face 2", "element edge 1\nproperty int a\nelement face 2"),
            Kind::UnsupportedElement,
            10,
        ),
        (
            edit(&base, "element face 2", "element vertex 1\nelement face 2"),
            Kind::UnsupportedElement,
            10,
        ),
        (edit(&base, "property float z\n", ""), Kind::MissingVertexElement, 9),
        (edit(&base, "property list uchar int vertex_indices\n", ""), Kind::MissingFaceElement, 11),
        (
            edit(
                &base,
                "element vertex 4",
                "element face 2\nproperty list uchar int vertex_indices\nelement vertex 4",
            ),
            Kind::MissingVertexElement,
            4,
        ),
        (edit(&base, "end_header\n", ""), Kind::MissingHeaderEnd, 18),
        (String::new(), Kind::MissingHeaderEnd, 1),
    ];
    for (text, kind, line) in &cases {
        assert_eq!(refusal(text), (*kind, *line), "refusing {text:?}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_008_property_refusals_name_their_kind_and_line() {
    let base = mesh_text();
    let cases: [(String, usize); 10] = [
        (edit(&base, "property float x", "property int x"), 5),
        (edit(&base, "property float x", "property quad x"), 5),
        (edit(&base, "property float x", "property list uchar float x"), 5),
        (edit(&base, "property float x", "property float x extra"), 5),
        (edit(&base, "property float confidence", "property float x"), 8),
        (edit(&base, "property float confidence", "property float"), 8),
        (edit(&base, "list uchar int vertex_indices", "list uchar int vertex_index"), 11),
        (edit(&base, "list uchar int vertex_indices", "list short int vertex_indices"), 11),
        (edit(&base, "list uchar int vertex_indices", "list uchar float vertex_indices"), 11),
        (
            edit(
                &base,
                "vertex_indices\n",
                "vertex_indices\nproperty list uchar int vertex_indices\n",
            ),
            12,
        ),
    ];
    for (text, line) in &cases {
        assert_eq!(refusal(text), (Kind::UnsupportedProperty, *line), "refusing {text:?}");
    }
    // A property before any element is not allowed either.
    let early = edit(&base, "format ascii 1.0\n", "format ascii 1.0\nproperty float x\n");
    assert_eq!(refusal(&early), (Kind::UnsupportedProperty, 3));
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_009_empty_elements_are_ignored_with_their_properties() {
    let text = edit(
        &mesh_text(),
        "element face 2",
        "element edge 0\nproperty int a\nproperty int b\nelement face 2",
    );
    assert_eq!(collect_vertices(&text), expected_vertices());
    assert_eq!(collect_triangles(&text).len(), 2);
    // A mesh with no vertices and no faces is valid.
    let empty = "ply\nformat ascii 1.0\nelement vertex 0\nproperty float x\nproperty float y\n\
property float z\nelement face 0\nproperty list uchar int vertex_indices\nend_header\n";
    let mesh = parse_ascii_ply(empty.as_bytes()).expect("empty mesh should parse");
    assert_eq!((mesh.vertex_count(), mesh.face_count()), (0, 0));
    assert_eq!(mesh.vertices().count() + mesh.triangles().count(), 0);
    // An end_header line with no terminator is not a header terminator.
    assert_eq!(refusal(empty.trim_end_matches('\n')), (Kind::MissingHeaderEnd, 10));
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_010_header_text_must_be_utf8_and_body_may_be_any_ascii() {
    let mut bytes = mesh_text().into_bytes();
    let at = bytes.windows(7).position(|window| window == b"by hand").expect("comment text");
    bytes[at] = 0xFF;
    assert_eq!(refusal_bytes(&bytes), (Kind::HeaderUtf8, 3));
    // A non-ASCII comment that is valid UTF-8 is fine.
    let utf8 = edit(&mesh_text(), "made by hand", "mad\u{e9} by h\u{e5}nd");
    assert_eq!(collect_vertices(&utf8), expected_vertices());
    // Non-ASCII bytes in the body are not a number.
    let body = edit(&mesh_text(), "0.12794", "0.1\u{e9}");
    assert_eq!(refusal(&body), (Kind::InvalidNumber, 13));
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_011_vertex_line_refusals_name_their_kind_and_line() {
    let base = mesh_text();
    let cases: [(String, Kind); 9] = [
        (
            edit(
                &base,
                "-0.0378297 0.12794 0.00447467 0.850855 0.5",
                "-0.0378297 0.12794 0.00447467 0.850855",
            ),
            Kind::VertexFieldCount,
        ),
        (
            edit(
                &base,
                "-0.0378297 0.12794 0.00447467 0.850855 0.5",
                "-0.0378297 0.12794 0.00447467 0.850855 0.5 7",
            ),
            Kind::VertexFieldCount,
        ),
        (edit(&base, "0.12794", "nan"), Kind::InvalidNumber),
        (edit(&base, "0.12794", "1e"), Kind::InvalidNumber),
        (edit(&base, "0.12794", "0x1p3"), Kind::InvalidNumber),
        (edit(&base, "0.12794", "2147483648"), Kind::NumberOverflow),
        (edit(&base, "0.12794", "-1e400"), Kind::NumberOverflow),
        (edit(&base, "0.12794", "0.1234567890123456789012345678901"), Kind::NumberTooLong),
        (edit(&base, "0.12794", "1.2.3"), Kind::InvalidNumber),
    ];
    for (text, kind) in &cases {
        assert_eq!(refusal(text), (*kind, 13), "refusing {text:?}");
    }
    // Extra properties are skipped by declaration and never interpreted.
    let odd_extras = edit(&base, "0.850855 0.5", "nan #");
    assert_eq!(collect_vertices(&odd_extras), expected_vertices());
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_012_face_refusals_name_their_kind_and_line() {
    let base = mesh_text();
    let cases: [(&str, Kind); 15] = [
        ("4 0 1 2 3", Kind::NonTriangularFace),
        ("2 0 1", Kind::NonTriangularFace),
        ("0", Kind::NonTriangularFace),
        ("99999999999999999999999 0 1 2", Kind::NonTriangularFace),
        ("3 0 1", Kind::FaceFieldCount),
        ("3 0 1 2 3", Kind::FaceFieldCount),
        ("3", Kind::FaceFieldCount),
        ("3 0 1 -1", Kind::NegativeIndex),
        ("3 0 1 4", Kind::IndexOutOfBounds),
        ("3 0 1 99999999999999999999999999", Kind::IndexOutOfBounds),
        ("3 0 1 x", Kind::InvalidIndex),
        ("3 0 1 1.0", Kind::InvalidIndex),
        ("3 0 1 +1", Kind::InvalidIndex),
        ("3 0 1 -", Kind::InvalidIndex),
        ("x 0 1 2", Kind::InvalidIndex),
    ];
    for (face, kind) in cases {
        let text = edit(&base, "3 0 1 2\n", &format!("{face}\n"));
        assert_eq!(refusal(&text), (kind, 17), "refusing face {face:?}");
    }
    // The last valid index is vertex_count - 1.
    assert_eq!(
        collect_triangles(&edit(&base, "3 0 1 2\n", "3 3 3 3\n"))[0],
        Triangle32::new(3, 3, 3)
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_013_missing_records_and_huge_declared_counts_are_refused_without_allocating() {
    let three_vertices: String =
        format!("{HEADER}{}", BODY.lines().take(3).collect::<Vec<_>>().join("\n") + "\n");
    assert_eq!(refusal(&three_vertices), (Kind::PayloadTooShort, 16));
    let one_face_short =
        format!("{HEADER}{}", BODY.lines().take(5).collect::<Vec<_>>().join("\n") + "\n");
    assert_eq!(refusal(&one_face_short), (Kind::PayloadTooShort, 18));
    // A declared count far above the body is a short body, found by reading the body.
    let huge_vertices = edit(&mesh_text(), "vertex 4", "vertex 4294967295");
    assert_eq!(refusal(&huge_vertices), (Kind::VertexFieldCount, 17));
    let huge_faces = edit(&mesh_text(), "face 2", "face 4294967295");
    assert_eq!(refusal(&huge_faces), (Kind::PayloadTooShort, 19));
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_014_errors_display_their_line_and_the_formats_refuse_each_other() {
    let error = parse_ascii_ply(b"ply\nformat ascii 1.0\n").expect_err("no end_header");
    assert_eq!(error.to_string(), "line 3: PLY header terminator was not found");
    let boxed: Box<dyn std::error::Error> = Box::new(error);
    assert_eq!(boxed.to_string(), error.to_string());
    // The binary reader refuses ASCII text, and this reader refuses a binary header.
    assert!(bunny_codec::parse_binary_ply(mesh_text().as_bytes()).is_err());
    let binary = "ply\nformat binary_little_endian 1.0\nelement vertex 0\nproperty float x\n\
property float y\nproperty float z\nelement face 0\nproperty list uchar int vertex_indices\nend_header\n";
    assert_eq!(refusal(binary), (Kind::UnsupportedFormat, 2));
    assert!(bunny_codec::parse_binary_ply(binary.as_bytes()).is_ok());
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_015_iterators_restart_and_yield_exactly_the_declared_counts() {
    let text = mesh_text();
    let mesh = parse_ascii_ply(text.as_bytes()).expect("mesh should parse");
    for _ in 0..2 {
        assert_eq!(mesh.vertices().count(), 4);
        assert_eq!(mesh.triangles().count(), 2);
    }
    let mut vertices = mesh.vertices();
    assert_eq!(vertices.next().and_then(Result::ok), Some(expected_vertices()[0]));
    assert_eq!(vertices.clone().count(), 3);
}
