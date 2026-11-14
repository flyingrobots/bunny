//! `parse_ascii_ply_with_limits`: each limit at and one past its value, the order of the
//! checks, a differential test against `parse_ascii_ply` that shows no limit changes an
//! accepted input or a profile error, and generated malformed input that never panics.
//! Every generated case prints its seed and index on failure.

use bunny_codec::{
    parse_ascii_ply, parse_ascii_ply_with_limits, AsciiPlyErrorKind as Kind, AsciiPlyLimit,
    AsciiPlyLimitedError, AsciiPlyLimits,
};
use wasm_bindgen_test::wasm_bindgen_test;

const HEADER: &str = "ply\nformat ascii 1.0\ncomment made by hand\nelement vertex 3\n\
property float x\nproperty float y\nproperty float z\nproperty uchar red\n\
element face 1\nproperty list uchar int vertex_indices\nend_header\n";
const BODY: &str = "0 0 0 255\n1.5 0 0 0\n0 -2.25 0 7\n3 0 1 2\n";

fn text() -> String {
    format!("{HEADER}{BODY}")
}

fn limits(vertices: usize, faces: usize, line: usize) -> AsciiPlyLimits {
    AsciiPlyLimits::new(vertices, faces, line)
}

fn exceeded(input: &str, limits: &AsciiPlyLimits) -> (AsciiPlyLimit, usize, usize, usize) {
    match parse_ascii_ply_with_limits(input.as_bytes(), limits) {
        Err(AsciiPlyLimitedError::Limit(error)) => {
            (error.limit(), error.maximum(), error.observed(), error.line())
        }
        other => panic!("expected a limit error, got {:?}", other.map(|_| ())),
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_055_counts_are_accepted_at_the_limit_and_refused_one_past_it() {
    let input = text();
    assert!(parse_ascii_ply_with_limits(input.as_bytes(), &limits(3, 1, 4096)).is_ok());
    assert_eq!(exceeded(&input, &limits(2, 1, 4096)), (AsciiPlyLimit::Vertices, 2, 3, 4));
    assert_eq!(exceeded(&input, &limits(3, 0, 4096)), (AsciiPlyLimit::Faces, 0, 1, 9));
    // Both over: the vertex count is reported first.
    assert_eq!(exceeded(&input, &limits(0, 0, 4096)).0, AsciiPlyLimit::Vertices);
    let mesh = parse_ascii_ply_with_limits(input.as_bytes(), &limits(3, 1, 4096)).expect("at");
    assert_eq!((mesh.vertex_count(), mesh.face_count()), (3, 1));
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_056_a_huge_declared_count_is_refused_before_any_body_line_is_read() {
    let header = HEADER.replace("vertex 3", "vertex 4294967295");
    // No body at all: the profile reader reads to the end and reports the missing record.
    let error = parse_ascii_ply(header.as_bytes()).expect_err("short");
    assert_eq!(error.kind(), Kind::PayloadTooShort);
    assert_eq!(
        exceeded(&header, &AsciiPlyLimits::DEFAULT),
        (AsciiPlyLimit::Vertices, 1 << 24, 4_294_967_295, 4)
    );
    // A count that does not fit `usize` keeps its profile error: the limit never sees it.
    let overflow = HEADER.replace("vertex 3", "vertex 99999999999999999999999");
    let expected = parse_ascii_ply(overflow.as_bytes()).expect_err("overflow");
    assert_eq!(expected.kind(), Kind::IntegerOverflow);
    let result = parse_ascii_ply_with_limits(overflow.as_bytes(), &AsciiPlyLimits::DEFAULT);
    assert!(matches!(result, Err(AsciiPlyLimitedError::Format(e)) if e == expected));
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_057_line_length_counts_bytes_without_the_terminator() {
    // Header line 3 is the longest: 60 bytes of comment.
    let comment = format!("comment {}", "c".repeat(52));
    assert_eq!(comment.len(), 60);
    for terminator in ["\n", "\r\n"] {
        let header = HEADER.replace("comment made by hand", &comment).replace('\n', terminator);
        let body = BODY.replace('\n', terminator);
        let input = format!("{header}{body}");
        let longest = input.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l).len()).max();
        assert_eq!(longest, Some(60));
        assert!(parse_ascii_ply_with_limits(input.as_bytes(), &limits(9, 9, 60)).is_ok());
        assert_eq!(exceeded(&input, &limits(9, 9, 59)), (AsciiPlyLimit::LineBytes, 59, 60, 3));
    }
    // The `end_header` line and a blank body line are lines too.
    let padded = text().replace("end_header", &format!("end_header{}", " ".repeat(100)));
    assert_eq!(exceeded(&padded, &limits(9, 9, 99)), (AsciiPlyLimit::LineBytes, 99, 110, 11));
    let blank =
        format!("{HEADER}0 0 0 255\n{}\n1.5 0 0 0\n0 -2.25 0 7\n3 0 1 2\n", " ".repeat(300));
    assert_eq!(exceeded(&blank, &limits(9, 9, 299)), (AsciiPlyLimit::LineBytes, 299, 300, 13));
    assert!(parse_ascii_ply_with_limits(blank.as_bytes(), &limits(9, 9, 300)).is_ok());
    // A blank line after the last record is checked too.
    let trailing = format!("{}{}\n", text(), " ".repeat(300));
    assert_eq!(exceeded(&trailing, &limits(9, 9, 299)).2, 300);
    // A line of a million tokens is refused by length, with its exact length observed.
    let huge = format!("{HEADER}{}\n", "1 ".repeat(1 << 20));
    assert_eq!(
        exceeded(&huge, &AsciiPlyLimits::DEFAULT),
        (AsciiPlyLimit::LineBytes, 4096, 2 << 20, 12)
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_058_the_checks_run_in_the_documented_order() {
    // 1. The profile's header structure errors come before any length check.
    let no_end = "ply\nformat ascii 1.0\n".to_string() + &"x".repeat(500);
    assert!(matches!(
        parse_ascii_ply_with_limits(no_end.as_bytes(), &limits(9, 9, 10)),
        Err(AsciiPlyLimitedError::Format(e)) if e.kind() == Kind::MissingHeaderEnd
    ));
    // 2. A long header line beats a later header error.
    let bad_then_long =
        text().replace("comment made by hand", &format!("{}\ncomment {}", "bogus", "c".repeat(80)));
    assert_eq!(exceeded(&bad_then_long, &limits(9, 9, 79)).0, AsciiPlyLimit::LineBytes);
    // 3. A header error beats a count limit.
    let bad_count =
        HEADER.replace("property float z\n", "property float z\nproperty list uchar int q\n");
    assert!(matches!(
        parse_ascii_ply_with_limits(bad_count.as_bytes(), &limits(0, 0, 4096)),
        Err(AsciiPlyLimitedError::Format(e)) if e.kind() == Kind::UnsupportedProperty
    ));
    // 4. A count limit beats a short body.
    assert_eq!(exceeded(HEADER, &limits(2, 9, 4096)).0, AsciiPlyLimit::Vertices);
    // 5. A long body line beats the parse error of its own record, and an earlier record
    //    error beats a later long line.
    let bad_long = text().replace("0 0 0 255", &format!("x{}", "9".repeat(50)));
    assert_eq!(exceeded(&bad_long, &limits(9, 9, 40)), (AsciiPlyLimit::LineBytes, 40, 51, 12));
    let bad_then_long = text()
        .replace("0 0 0 255", "x 0 0 255")
        .replace("3 0 1 2", &format!("3 0 1 2{}", " ".repeat(50)));
    let result = parse_ascii_ply_with_limits(bad_then_long.as_bytes(), &limits(9, 9, 40));
    assert!(
        matches!(result, Err(AsciiPlyLimitedError::Format(e)) if e.kind() == Kind::InvalidNumber && e.line() == 12)
    );
}

struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(bound).expect("bound")).expect("small")
    }
}

/// A valid mesh with random sizes, extra columns, line endings, blank lines and comments.
fn generated_mesh(seeds: &mut Seeds) -> String {
    let (vertices, faces) = (1 + seeds.below(12), seeds.below(12));
    let extra = seeds.below(4);
    let eol = if seeds.below(2) == 0 { "\n" } else { "\r\n" };
    let mut lines = vec!["ply".to_string(), "format ascii 1.0".to_string()];
    if seeds.below(2) == 0 {
        lines.push(format!("comment {}", "c".repeat(seeds.below(40))));
    }
    lines.push(format!("element vertex {vertices}"));
    lines.extend(["property float x", "property float y", "property float z"].map(String::from));
    lines.extend((0..extra).map(|column| format!("property uchar extra{column}")));
    lines.push(format!("element face {faces}"));
    lines.push("property list uchar int vertex_indices".to_string());
    lines.push("end_header".to_string());
    for _ in 0..vertices {
        let coordinate = |seeds: &mut Seeds| format!("{}.{}", seeds.below(100), seeds.below(1000));
        let mut columns = [coordinate(seeds), coordinate(seeds), coordinate(seeds)].to_vec();
        columns.extend((0..extra).map(|_| seeds.below(256).to_string()));
        lines.push(columns.join(" "));
        if seeds.below(5) == 0 {
            lines.push(" ".repeat(seeds.below(30)));
        }
    }
    for _ in 0..faces {
        let (a, b, c) = (seeds.below(vertices), seeds.below(vertices), seeds.below(vertices));
        lines.push(format!("3 {a} {b} {c}"));
    }
    lines.join(eol) + eol
}

/// Flips, deletes, duplicates or inserts a few bytes.
fn mutated(seeds: &mut Seeds, mut bytes: Vec<u8>) -> Vec<u8> {
    for _ in 0..=seeds.below(4) {
        let at = seeds.below(bytes.len().max(1));
        let byte = [b' ', b'\n', b'\r', b'0', b'9', b'-', b'.', b'e', 0xFF, b'x'][seeds.below(10)];
        match seeds.below(4) {
            0 if at < bytes.len() => bytes[at] = byte,
            1 if at < bytes.len() => {
                bytes.remove(at);
            }
            2 => bytes.insert(at.min(bytes.len()), byte),
            _ => {
                let end = (at + seeds.below(40)).min(bytes.len());
                let copy: Vec<u8> = bytes.get(at..end).unwrap_or_default().to_vec();
                bytes.splice(end..end, copy);
            }
        }
    }
    bytes
}

const SEED: u64 = 0xB002_7ACE_0300_0001;

/// With limits that no input of this size can reach, the new entry point returns what
/// `parse_ascii_ply` returns: the same counts and records for an accepted input, the same
/// error otherwise. This is the evidence that the implicit limits of `parse_ascii_ply`
/// (counts and lines bounded by the input) are exactly `AsciiPlyLimits::UNLIMITED`.
#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_059_generous_limits_change_no_accepted_input_and_no_profile_error() {
    let mut seeds = Seeds(SEED);
    let (mut accepted, mut refused) = (0_u32, 0_u32);
    for case in 0..20_000_u32 {
        let valid = generated_mesh(&mut seeds).into_bytes();
        let bytes = if case % 2 == 0 { valid } else { mutated(&mut seeds, valid) };
        let generous = [AsciiPlyLimits::UNLIMITED, limits(bytes.len(), bytes.len(), bytes.len())];
        let plain = parse_ascii_ply(&bytes);
        for generous in &generous {
            let limited = parse_ascii_ply_with_limits(&bytes, generous);
            let context =
                format!("seed {SEED:#x} case {case}: {:?}", String::from_utf8_lossy(&bytes));
            match (&plain, limited) {
                (Ok(plain), Ok(limited)) => {
                    assert_eq!(plain.vertex_count(), limited.vertex_count(), "{context}");
                    assert_eq!(plain.face_count(), limited.face_count(), "{context}");
                    assert!(plain.vertices().eq(limited.vertices()), "{context}");
                    assert!(plain.triangles().eq(limited.triangles()), "{context}");
                }
                (Err(plain), Err(AsciiPlyLimitedError::Format(limited))) => {
                    assert_eq!(*plain, limited, "{context}");
                }
                (plain, limited) => {
                    panic!("{context}: {:?} against {limited:?}", plain.as_ref().map(|_| ()))
                }
            }
        }
        if plain.is_ok() {
            accepted += 1;
        } else {
            refused += 1;
        }
    }
    assert!(accepted >= 8_000 && refused >= 5_000, "{accepted} accepted, {refused} refused");
}

/// Mutated and random input under small limits never panics, and every limit error is true:
/// `observed` exceeds `maximum`, and a line length is that of the named line, found by
/// splitting the input independently of the reader.
#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_060_malformed_input_under_small_limits_never_panics_and_limit_errors_are_true() {
    let mut seeds = Seeds(SEED ^ 1);
    let (mut by_limit, mut accepted) = ([0_u32; 3], 0_u32);
    for case in 0..30_000_u32 {
        let bytes = match case % 4 {
            0 => {
                let valid = generated_mesh(&mut seeds).into_bytes();
                mutated(&mut seeds, valid)
            }
            1 => {
                let mut bytes = generated_mesh(&mut seeds).into_bytes();
                bytes.truncate(seeds.below(bytes.len() + 1));
                bytes
            }
            3 => generated_mesh(&mut seeds).into_bytes(),
            _ => (0..seeds.below(300))
                .map(|_| u8::try_from(seeds.next() % 256).expect("byte"))
                .collect(),
        };
        let line = if seeds.below(2) == 0 { 1 + seeds.below(60) } else { 40 + seeds.below(60) };
        let small = limits(seeds.below(14), seeds.below(14), line);
        let context =
            format!("seed {:#x} case {case}: {:?}", SEED ^ 1, String::from_utf8_lossy(&bytes));
        match parse_ascii_ply_with_limits(&bytes, &small) {
            Ok(mesh) => {
                accepted += 1;
                assert!(mesh.vertex_count() <= small.max_vertices, "{context}");
                assert!(mesh.face_count() <= small.max_faces, "{context}");
                assert_eq!(mesh.vertices().count(), mesh.vertex_count(), "{context}");
                assert_eq!(mesh.triangles().count(), mesh.face_count(), "{context}");
            }
            Err(AsciiPlyLimitedError::Format(_)) => {}
            Err(AsciiPlyLimitedError::Limit(error)) => {
                assert!(error.observed() > error.maximum(), "{context}");
                match error.limit() {
                    AsciiPlyLimit::Vertices => {
                        by_limit[0] += 1;
                        assert_eq!(error.maximum(), small.max_vertices, "{context}");
                    }
                    AsciiPlyLimit::Faces => {
                        by_limit[1] += 1;
                        assert_eq!(error.maximum(), small.max_faces, "{context}");
                    }
                    AsciiPlyLimit::LineBytes => {
                        by_limit[2] += 1;
                        assert_eq!(error.maximum(), small.max_line_bytes, "{context}");
                        let line =
                            bytes.split(|b| *b == b'\n').nth(error.line() - 1).expect("line");
                        let length = line.strip_suffix(b"\r").unwrap_or(line).len();
                        assert_eq!(error.observed(), length, "{context}");
                        let earlier = bytes.split(|b| *b == b'\n').take(error.line() - 1);
                        assert!(
                            earlier.into_iter().all(|l| l.strip_suffix(b"\r").unwrap_or(l).len()
                                <= small.max_line_bytes),
                            "{context}: not the first long line"
                        );
                    }
                }
            }
        }
    }
    assert!(
        by_limit.iter().all(|&n| n >= 500) && accepted >= 500,
        "{by_limit:?}, {accepted} accepted"
    );
}
