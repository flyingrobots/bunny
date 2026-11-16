//! Per-vertex texture coordinates of the ASCII PLY reader (flyingrobots/bunny#369): the
//! pairs `s`, `t` and `u`, `v`, their exact conversion, the refusal of a lone or mixed
//! pair, and a file without them read as before.
//!
//! Every generated case prints its seed and index on failure.

use bunny_codec::{
    parse_ascii_ply, parse_ascii_ply_with_limits, parse_decimal_q32_32, AsciiPlyErrorKind as Kind,
    AsciiPlyLimitedError, AsciiPlyLimits, AsciiPlyMesh,
};
use std::fmt::Write as _;

use wasm_bindgen_test::wasm_bindgen_test;

const STANFORD: &[u8] = include_bytes!("fixtures/stanford-bunny-ascii/bun_zipper_res3.ply");

struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }
}

/// A decimal with exactly `k` fractional digits whose value is `m / 2^k`, and its raw
/// Q32.32 value `m * 2^(32 - k)`. The decimal is `m * 5^k / 10^k`, so no rounding is
/// involved and the oracle needs no conversion: it is a shift.
fn dyadic(seeds: &mut Seeds) -> (String, i64) {
    let k = u32::try_from(seeds.below(17)).expect("small");
    let magnitude = i128::from(seeds.below(1 << (k + 20)));
    let m = if seeds.below(2) == 0 { magnitude } else { -magnitude };
    let scaled = m.abs() * 5_i128.pow(k);
    let digits = format!("{scaled:0>width$}", width = usize::try_from(k).expect("k") + 1);
    let (whole, fraction) = digits.split_at(digits.len() - usize::try_from(k).expect("k"));
    let sign = if m < 0 { "-" } else { "" };
    let text = if k == 0 { format!("{sign}{whole}") } else { format!("{sign}{whole}.{fraction}") };
    (text, i64::try_from(m << (32 - k)).expect("fits Q32.32"))
}

/// One vertex property line per `(type, name)`, then `count` rows of `columns`.
fn mesh(properties: &[(&str, &str)], rows: &[Vec<String>]) -> String {
    let mut text = format!("ply\nformat ascii 1.0\nelement vertex {}\n", rows.len());
    for (kind, name) in properties {
        writeln!(text, "property {kind} {name}").expect("writing to a String");
    }
    text.push_str("element face 1\nproperty list uchar int vertex_indices\nend_header\n");
    for row in rows {
        text.push_str(&row.join(" "));
        text.push('\n');
    }
    text.push_str("3 0 0 0\n");
    text
}

fn uv_raw(mesh: &AsciiPlyMesh<'_>) -> Vec<(i64, i64)> {
    let uvs = mesh.texture_coordinates().expect("texture coordinates declared");
    uvs.map(|uv| uv.expect("validated")).map(|uv| (uv.u.raw(), uv.v.raw())).collect()
}

fn refusal(text: &str) -> (Kind, usize) {
    let error = parse_ascii_ply(text.as_bytes()).expect_err("input should be refused");
    (error.kind(), error.line())
}

/// Generated dyadic decimals in an `x y z s t` layout and in a `u nx x v y z` layout (the
/// pair first, a skipped column between, `double` types): every texture coordinate and
/// position equals the shifted integer, through both entry points.
#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_066_both_pairs_return_exact_texture_coordinates_for_generated_decimals() {
    let seed = 0x369_5eed;
    let mut seeds = Seeds(seed);
    let layouts: [&[(&str, &str)]; 2] = [
        &[("float", "x"), ("float", "y"), ("float", "z"), ("float", "s"), ("float", "t")],
        &[
            ("double", "u"),
            ("float", "nx"),
            ("float64", "x"),
            ("float32", "v"),
            ("double", "y"),
            ("float", "z"),
        ],
    ];
    for case in 0..400 {
        let (layout, rows_wanted) = (layouts[case % 2], 1 + seeds.below(8));
        let mut rows = Vec::new();
        let mut expected = Vec::new();
        for _ in 0..rows_wanted {
            let values: Vec<(String, i64)> = layout.iter().map(|_| dyadic(&mut seeds)).collect();
            let raw = |name: &str| values[layout.iter().position(|(_, n)| *n == name).unwrap()].1;
            let (u, v) = if case % 2 == 0 { ("s", "t") } else { ("u", "v") };
            expected.push(([raw("x"), raw("y"), raw("z")], (raw(u), raw(v))));
            rows.push(values.into_iter().map(|(text, _)| text).collect());
        }
        let text = mesh(layout, &rows);
        let context = format!("seed {seed:#x}, case {case}:\n{text}");
        let plain = parse_ascii_ply(text.as_bytes()).unwrap_or_else(|e| panic!("{context}\n{e}"));
        let limited = parse_ascii_ply_with_limits(text.as_bytes(), &AsciiPlyLimits::DEFAULT)
            .unwrap_or_else(|e| panic!("{context}\n{e}"));
        for mesh in [plain, limited] {
            assert!(mesh.has_texture_coordinates(), "{context}");
            let positions: Vec<[i64; 3]> = mesh
                .vertices()
                .map(|v| v.expect("validated"))
                .map(|v| [v.x.raw(), v.y.raw(), v.z.raw()])
                .collect();
            let wanted_positions: Vec<[i64; 3]> = expected.iter().map(|(p, _)| *p).collect();
            let wanted_uvs: Vec<(i64, i64)> = expected.iter().map(|(_, uv)| *uv).collect();
            assert_eq!(positions, wanted_positions, "{context}");
            assert_eq!(uv_raw(&mesh), wanted_uvs, "{context}");
        }
    }
}

/// Hand-derived values, and generated decimals that need rounding (up to 18 fractional
/// digits, signs, exponents): each texture coordinate is the conversion of its own text by
/// `parse_decimal_q32_32`, the rule the chapter names.
#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_067_texture_coordinates_follow_the_decimal_rule() {
    let layout = [("float", "x"), ("float", "y"), ("float", "z"), ("float", "s"), ("float", "t")];
    let row = |s: &str, t: &str| vec!["0".into(), "0".into(), "0".into(), s.into(), t.into()];
    let hand = mesh(&layout, &[row("0.5", "1"), row("-0.125", "0.25"), row("1e-1", ".75")]);
    let parsed = parse_ascii_ply(hand.as_bytes()).expect("hand mesh");
    // 0.5 = 2^31; 1 = 2^32; -0.125 = -2^29; 0.25 = 2^30; 0.75 = 3 * 2^30;
    // 0.1 * 2^32 = 429496729.6, which rounds to 429496730.
    let wanted = vec![(1 << 31, 1 << 32), (-(1 << 29), 1 << 30), (429_496_730, 3 << 30)];
    assert_eq!(uv_raw(&parsed), wanted);

    let seed = 0x369_dec;
    let mut seeds = Seeds(seed);
    for case in 0..2000 {
        let decimal = |seeds: &mut Seeds| {
            let sign = ["", "-", "+"][usize::try_from(seeds.below(3)).expect("small")];
            let whole = seeds.below(100_000);
            let fraction = format!("{:0>18}", seeds.next() % 1_000_000_000_000_000_000);
            let fraction = &fraction[..usize::try_from(seeds.below(19)).expect("small")];
            let exponent = if seeds.below(4) == 0 {
                format!("e{}", i64::try_from(seeds.below(7)).expect("small") - 3)
            } else {
                String::new()
            };
            format!("{sign}{whole}.{fraction}{exponent}")
        };
        let (s, t) = (decimal(&mut seeds), decimal(&mut seeds));
        let text = mesh(&layout, &[row(&s, &t)]);
        let convert =
            |text: &str| parse_decimal_q32_32(text.as_bytes()).map(bunny_num::FixedQ32_32::raw);
        let context = format!("seed {seed:#x}, case {case}: s = {s}, t = {t}");
        match (convert(&s), convert(&t)) {
            (Ok(s), Ok(t)) => {
                let parsed = parse_ascii_ply(text.as_bytes()).expect(&context);
                assert_eq!(uv_raw(&parsed), vec![(s, t)], "{context}");
            }
            _ => assert!(parse_ascii_ply(text.as_bytes()).is_err(), "{context}"),
        }
    }
}

/// A lone name of either pair, names from both pairs, a repeated name, and a texture name
/// with an integer type are `UnsupportedProperty`, at the line of the last texture name
/// declared (lines 4 to 6 are `x`, `y`, `z`, so the first extra property is line 7).
#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_068_an_incomplete_or_ambiguous_pair_is_refused_at_its_line() {
    let xyz = [("float", "x"), ("float", "y"), ("float", "z")];
    let cases: [(&[(&str, &str)], usize); 9] = [
        (&[("float", "s")], 7),
        (&[("float", "t")], 7),
        (&[("float", "u")], 7),
        (&[("float", "v"), ("float", "nx")], 7),
        (&[("float", "s"), ("float", "nx"), ("float", "v")], 9),
        (&[("float", "s"), ("float", "t"), ("float", "u"), ("float", "v")], 10),
        (&[("float", "u"), ("float", "v"), ("float", "s")], 9),
        (&[("float", "s"), ("float", "s"), ("float", "t")], 8),
        (&[("float", "s"), ("uchar", "t")], 8),
    ];
    for (extra, line) in cases {
        let properties: Vec<(&str, &str)> = xyz.iter().chain(extra).copied().collect();
        let row = vec!["0".to_string(); properties.len()];
        let text = mesh(&properties, &[row]);
        assert_eq!(refusal(&text), (Kind::UnsupportedProperty, line), "{text}");
        let limited = parse_ascii_ply_with_limits(text.as_bytes(), &AsciiPlyLimits::DEFAULT);
        let Err(AsciiPlyLimitedError::Format(error)) = limited else {
            panic!("expected a format error: {text}");
        };
        assert_eq!((error.kind(), error.line()), (Kind::UnsupportedProperty, line), "{text}");
    }
}

/// A texture coordinate that is not a decimal, overflows or is too long is refused like a
/// position, at its body line (line 13 is the second vertex).
#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_069_a_bad_texture_coordinate_is_refused_at_its_body_line() {
    let layout = [("float", "x"), ("float", "y"), ("float", "z"), ("float", "u"), ("float", "v")];
    let row = |v: &str| vec!["0".into(), "0".into(), "0".into(), "0".into(), v.into()];
    for (value, kind) in [
        ("nan", Kind::InvalidNumber),
        ("1e10", Kind::NumberOverflow),
        ("1.2345678901234567890123456789", Kind::NumberTooLong),
    ] {
        let text = mesh(&layout, &[row("0"), row(value)]);
        assert_eq!(refusal(&text), (kind, 13), "{value}");
    }
}

/// A file without texture coordinates reads as before: the committed Stanford fixture has
/// none, and its positions and triangles hash to the digest taken with the reader at
/// `origin/main` 4a3957b, before this change (FNV-1a 64 over the little-endian raw x, y, z
/// of each vertex, then v0, v1, v2 of each triangle). A file whose skipped columns have
/// other names, such as `nx` or `confidence`, keeps them skipped.
#[wasm_bindgen_test(unsupported = test)]
fn aply_tp_070_a_file_without_texture_coordinates_reads_as_before() {
    fn fnv(state: &mut u64, bytes: &[u8]) {
        for byte in bytes {
            *state ^= u64::from(*byte);
            *state = state.wrapping_mul(0x0100_0000_01b3);
        }
    }
    let stanford = parse_ascii_ply(STANFORD).expect("fixture");
    assert!(!stanford.has_texture_coordinates());
    assert!(stanford.texture_coordinates().is_none());
    let mut digest = 0xcbf2_9ce4_8422_2325_u64;
    for vertex in stanford.vertices() {
        let vertex = vertex.expect("validated");
        for raw in [vertex.x.raw(), vertex.y.raw(), vertex.z.raw()] {
            fnv(&mut digest, &raw.to_le_bytes());
        }
    }
    for triangle in stanford.triangles() {
        let triangle = triangle.expect("validated");
        for index in [triangle.v0, triangle.v1, triangle.v2] {
            fnv(&mut digest, &index.to_le_bytes());
        }
    }
    assert_eq!((stanford.vertex_count(), stanford.face_count()), (1889, 3851));
    assert_eq!(digest, 0x5342_2bbd_42c5_ab62);

    let layout = [("float", "x"), ("float", "y"), ("float", "z"), ("uchar", "nx")];
    let text = mesh(&layout, &[vec!["1".into(), "2".into(), "3".into(), "not-a-number".into()]]);
    let parsed = parse_ascii_ply(text.as_bytes()).expect("skipped column");
    assert!(parsed.texture_coordinates().is_none());
}
