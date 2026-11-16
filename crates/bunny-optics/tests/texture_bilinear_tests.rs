//! Evidence for `bunny-texture-bilinear/1`: hand-derived coordinates and lookups, exact
//! rational oracles over seeded triangles, hits and coordinates, and texel centres.
//!
//! Every oracle is exact integer arithmetic written here from the chapter's definition; none
//! calls the code under test. Seeded cases print their seed.

// Seeded draws are reduced before each `as` conversion, so every value is in range.
#![allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap, clippy::cast_sign_loss)]

use std::cmp::Ordering;

use bunny_linalg::FixedVec2;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_optics::{bilinear, interpolate_uv, BilinearTaps, TextureWrap, TEXTURE_BILINEAR_ID};
use bunny_query::BarycentricNumerators;
use wasm_bindgen_test::wasm_bindgen_test;

const ONE: i64 = ONE_RAW;
const TEXTURE_TEST_SEED: u64 = 0xB002_0367_7E70_0001;

/// The `SplitMix64` generator.
struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value in `0..2^bits`, `bits <= 127`.
    fn bits(&mut self, bits: u32) -> u128 {
        let wide = (u128::from(self.next()) << 64) | u128::from(self.next());
        if bits == 0 {
            0
        } else {
            wide >> (128 - bits)
        }
    }

    /// A raw coordinate: full range, near zero, or within a few texture widths.
    fn coordinate(&mut self) -> i64 {
        match self.next() % 4 {
            0 => self.next() as i64,
            1 => (self.next() % 2049) as i64 - 1024,
            2 => (self.next() as i64) >> 24,
            _ => (self.next() as i64) >> 28,
        }
    }
}

fn uv(x: i64, y: i64) -> FixedVec2 {
    FixedVec2::new(FixedQ32_32::from_raw(x), FixedQ32_32::from_raw(y))
}

fn at(uvs: [(i64, i64); 3], u: i128, v: i128, det: i128) -> (i64, i64) {
    let weights = BarycentricNumerators::new(u, v, det).expect("a point on the triangle");
    let t = interpolate_uv(uvs.map(|(x, y)| uv(x, y)), weights);
    (t.x.raw(), t.y.raw())
}

/// An unsigned integer of any size: little-endian base-`2^32` limbs, no trailing zero limb.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Big(Vec<u64>);

impl Big {
    fn trim(mut self) -> Self {
        while self.0.last() == Some(&0) {
            self.0.pop();
        }
        self
    }

    fn mul(&self, other: &Self) -> Self {
        let mut out = vec![0_u64; self.0.len() + other.0.len() + 1];
        for (i, a) in self.0.iter().enumerate() {
            for (j, b) in other.0.iter().enumerate() {
                out[i + j] += a * b;
                for k in i + j.. {
                    let carry = out[k] >> 32;
                    out[k] &= 0xFFFF_FFFF;
                    if carry == 0 {
                        break;
                    }
                    out[k + 1] += carry;
                }
            }
        }
        Self(out).trim()
    }

    fn add(&self, other: &Self) -> Self {
        let mut out = Vec::new();
        let mut carry = 0;
        for k in 0..=self.0.len().max(other.0.len()) {
            let t = self.0.get(k).unwrap_or(&0) + other.0.get(k).unwrap_or(&0) + carry;
            out.push(t & 0xFFFF_FFFF);
            carry = t >> 32;
        }
        Self(out).trim()
    }

    fn compare(&self, other: &Self) -> Ordering {
        self.0.len().cmp(&other.0.len()).then_with(|| self.0.iter().rev().cmp(other.0.iter().rev()))
    }
}

fn big(value: u128) -> Big {
    Big((0..4).map(|k| (value >> (32 * k)) as u64 & 0xFFFF_FFFF).collect()).trim()
}

/// Whether `r` is `sum w_i t_i / det` rounded to nearest, ties to even, decided exactly: with
/// every value shifted by `2^63` to be nonnegative, `(2r - 1) det <= 2N <= (2r + 1) det`, and an
/// end reached only when `r` is even.
fn is_nearest_even(r: i64, values: [i64; 3], weights: [i128; 3], det: i128) -> bool {
    let shift = |t: i64| (i128::from(t) + (1 << 63)) as u128;
    let twice_sum = values
        .iter()
        .zip(weights)
        .fold(big(0), |sum, (t, w)| sum.add(&big(shift(*t)).mul(&big(w as u128)).mul(&big(2))));
    let r = shift(r);
    let det = big(det as u128);
    let low = big(2 * r).mul(&det); // 2r det - det <= 2N  <=>  2r det <= 2N + det
    let high = big(2 * r).mul(&det).add(&det).add(&det); // 2N <= 2r det + det
    let lower = low.compare(&twice_sum.add(&det));
    let upper = twice_sum.add(&det).compare(&high);
    let even = r % 2 == 0;
    match (lower, upper) {
        (Ordering::Greater, _) | (_, Ordering::Greater) => false,
        (Ordering::Equal, _) | (_, Ordering::Equal) => even,
        _ => true,
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_110_hand_derived_coordinates_interpolate_exactly() {
    assert_eq!(TEXTURE_BILINEAR_ID, "bunny-texture-bilinear/1");
    let tri = [(0, 0), (ONE, 0), (0, ONE)];
    // Corners give their vertex exactly.
    assert_eq!(at(tri, 0, 0, 7), (0, 0));
    assert_eq!(at(tri, 7, 0, 7), (ONE, 0));
    assert_eq!(at(tri, 0, 7, 7), (0, ONE));
    // The centroid: 2^32 / 3 = 1 431 655 765.33.
    assert_eq!(at(tri, 1, 1, 3), (1_431_655_765, 1_431_655_765));
    // Two thirds: 2 863 311 530.67 rounds up.
    assert_eq!(at(tri, 2, 0, 3), (2_863_311_531, 0));
    // Ties to even: (1, 2) and (1, 3) halfway give 1.5 and 2.5 raw, both to 2.
    assert_eq!(at([(1, 1), (2, 3), (0, 0)], 1, 0, 2), (2, 2));
    // Negative coordinates: -1.5 raw ties to -2, -0.5 raw to 0.
    assert_eq!(at([(-1, 0), (-2, -1), (0, 0)], 1, 0, 2), (-2, 0));
    // Extremes of i64 and of det stay representable.
    let det = i128::MAX - 1;
    assert_eq!(at([(i64::MIN, i64::MAX), (i64::MAX, i64::MIN), (0, 0)], det / 2, 0, det), (0, 0));
    assert_eq!(at([(i64::MAX, i64::MIN); 3], 5, 9, det), (i64::MAX, i64::MIN));
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_111_interpolated_coordinates_match_the_exact_rational_oracle() {
    let seed = TEXTURE_TEST_SEED ^ 0x0111;
    println!("OPTICS-TP-111 seed {seed:#x}");
    let mut seeds = Seeds(seed);
    for case in 0..20_000 {
        let det_bits = [117, 127, 64, 8, 2][case % 5];
        let det = (seeds.bits(det_bits) as i128).max(1);
        let u = (seeds.bits(127) as i128) % (det + 1);
        let v = (seeds.bits(127) as i128) % (det - u + 1);
        let uvs = [0; 3].map(|_| (seeds.coordinate(), seeds.coordinate()));
        let weights = [det - u - v, u, v];
        let (x, y) = at(uvs, u, v, det);
        let ok = is_nearest_even(x, uvs.map(|t| t.0), weights, det)
            && is_nearest_even(y, uvs.map(|t| t.1), weights, det);
        assert!(ok, "seed {seed:#x} case {case}: {uvs:?} at {weights:?} / {det} gave {x}, {y}");
    }
}

/// Independent wrap: Euclidean remainder or clamp.
fn wrap_oracle(index: i128, size: u32, wrap: TextureWrap) -> u32 {
    let n = i128::from(size);
    match wrap {
        TextureWrap::Repeat => (((index % n) + n) % n) as u32,
        TextureWrap::Clamp => index.max(0).min(n - 1) as u32,
    }
}

/// The exact split along one axis, in units of `2^-33`: `X = 2 t size - 2^32`, so the texel
/// below is `floor(X / 2^33)` and the fraction `X mod 2^33` over `2^33`.
fn axis_oracle(t: i64, size: u32) -> (i128, i128) {
    let x = 2 * i128::from(t) * i128::from(size) - (1_i128 << 32);
    (x.div_euclid(1 << 33), x.rem_euclid(1 << 33))
}

/// Checks a lookup against the exact oracle: indices, wrapping, weights within half a raw unit
/// of the exact products, in `0..=1.0` and summing to exactly 1.0.
fn check_lookup(
    [u, v]: [i64; 2],
    [width, height]: [u32; 2],
    wrap: TextureWrap,
    taps: BilinearTaps,
) {
    let (x0, fx) = axis_oracle(u, width);
    let (y0, fy) = axis_oracle(v, height);
    let (c0, c1) = (wrap_oracle(x0, width, wrap), wrap_oracle(x0 + 1, width, wrap));
    let (r0, r1) = (wrap_oracle(y0, height, wrap), wrap_oracle(y0 + 1, height, wrap));
    let context = format!("u {u} v {v} size {width}x{height} {wrap:?}");
    assert_eq!(taps.texels(), [[c0, r0], [c1, r0], [c0, r1], [c1, r1]], "{context}");
    for [c, r] in taps.texels() {
        assert!(c < width && r < height, "{context}");
    }
    let full = 1_i128 << 33;
    let exact = [(full - fx) * (full - fy), fx * (full - fy), (full - fx) * fy, fx * fy];
    let weights = taps.weights().map(FixedQ32_32::raw);
    for (w, e) in weights.iter().zip(exact) {
        // |w - e / 2^34| <= 1/2 raw unit.
        assert!((i128::from(*w) * (1 << 34) - e).abs() <= 1 << 33, "{context}: {weights:?}");
        assert!((0..=ONE).contains(w), "{context}: {weights:?}");
    }
    assert_eq!(weights.iter().sum::<i64>(), ONE, "{context}");
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_112_hand_derived_lookups() {
    let half = ONE / 2;
    // A 2 by 2 texture at its middle: texels 0 and 1 each way, weights a quarter each.
    let taps = bilinear(uv(half, half), [2, 2], TextureWrap::Repeat).expect("a nonempty texture");
    assert_eq!(taps.texels(), [[0, 0], [1, 0], [0, 1], [1, 1]]);
    assert_eq!(taps.weights().map(FixedQ32_32::raw), [ONE / 4; 4]);
    // u = v = 0 on a 4 by 4 texture: halfway between the last and first texels.
    let zero = FixedQ32_32::ZERO;
    let repeat =
        bilinear(FixedVec2::new(zero, zero), [4, 4], TextureWrap::Repeat).expect("nonempty");
    assert_eq!(repeat.texels(), [[3, 3], [0, 3], [3, 0], [0, 0]]);
    let clamp = bilinear(FixedVec2::new(zero, zero), [4, 4], TextureWrap::Clamp).expect("nonempty");
    assert_eq!(clamp.texels(), [[0, 0]; 4]);
    assert_eq!(clamp.weights().map(FixedQ32_32::raw), [ONE / 4; 4]);
    // A 3-wide texture at u = 0.5: x = 1, a centre along u; v = 1/4 on 1 row: y = -1/4.
    let taps = bilinear(uv(half, ONE / 4), [3, 1], TextureWrap::Clamp).expect("nonempty");
    assert_eq!(taps.texels(), [[1, 0], [2, 0], [1, 0], [2, 0]]);
    assert_eq!(taps.weights().map(FixedQ32_32::raw), [ONE / 4, 0, 3 * ONE / 4, 0]);
    // u = v = 1.0 + 1 raw on 1 by 1: fx = fy = 2^31 + 1, and
    // fx fy = (2^31 + 1)^2 / 2^32 = 2^30 + 1 + 2^-32. w11 rounds to 2^30 + 1; w10 = (2^31 + 1)(2^31 - 1) / 2^32 = 2^30 - 2^-32 rounds to 2^30.
    let t = FixedQ32_32::from_raw(ONE + 1);
    let taps = bilinear(FixedVec2::new(t, t), [1, 1], TextureWrap::Repeat).expect("nonempty");
    let q = ONE / 4;
    assert_eq!(taps.weights().map(FixedQ32_32::raw), [q - 1, q, q, q + 1]);
    // A tie to even: fx = 2^31, fy = 1 raw: fx fy = 2^-1 raw ties to 0; fx (1 - fy) =
    // 2^31 - 1/2 ties to 2^31 (even).
    let taps = bilinear(uv(ONE, ONE / 2 + 1), [1, 1], TextureWrap::Repeat).expect("nonempty");
    assert_eq!(taps.weights().map(FixedQ32_32::raw), [half - 1, half, 1, 0]);
    // Empty textures are refused.
    assert_eq!(bilinear(FixedVec2::new(zero, zero), [0, 4], TextureWrap::Repeat), None);
    assert_eq!(bilinear(FixedVec2::new(zero, zero), [4, 0], TextureWrap::Clamp), None);
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_113_seeded_lookups_match_the_exact_oracle() {
    let seed = TEXTURE_TEST_SEED ^ 0x0113;
    println!("OPTICS-TP-113 seed {seed:#x}");
    let mut seeds = Seeds(seed);
    for case in 0..20_000 {
        let size = |s: &mut Seeds| match s.next() % 3 {
            0 => 1 + (s.next() % 8) as u32,
            1 => 1 + (s.next() % 4096) as u32,
            _ => (s.next() as u32).max(1),
        };
        let (width, height) = (size(&mut seeds), size(&mut seeds));
        let (u, v) = (seeds.coordinate(), seeds.coordinate());
        let wrap = if case % 2 == 0 { TextureWrap::Repeat } else { TextureWrap::Clamp };
        let taps = bilinear(uv(u, v), [width, height], wrap).expect("a nonempty texture");
        check_lookup([u, v], [width, height], wrap, taps);
    }
    // The ends of the raw range and of the size.
    for u in [i64::MIN, i64::MIN + 1, -1, 0, 1, i64::MAX - 1, i64::MAX] {
        for size in [1, 2, 3, u32::MAX] {
            for wrap in [TextureWrap::Repeat, TextureWrap::Clamp] {
                let taps = bilinear(uv(u, u.wrapping_neg()), [size, size], wrap).expect("nonempty");
                check_lookup([u, u.wrapping_neg()], [size, size], wrap, taps);
            }
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_114_a_texel_centre_returns_that_texel_with_weight_one() {
    let seed = TEXTURE_TEST_SEED ^ 0x0114;
    println!("OPTICS-TP-114 seed {seed:#x}");
    let mut seeds = Seeds(seed);
    for case in 0..5_000 {
        // Power-of-two sizes up to 2^20 make every centre (i + 1/2) / size exact in raw units.
        let (wb, hb) = ((seeds.next() % 21) as u32, (seeds.next() % 21) as u32);
        let (width, height) = (1_u32 << wb, 1_u32 << hb);
        let (i, j) = (seeds.next() as u32 % width, seeds.next() as u32 % height);
        // Shift by a whole number of tiles, negative or positive, for repeat.
        let tiles = (seeds.next() % 2001) as i64 - 1000;
        let centre = |k: u32, b: u32| (2 * i64::from(k) + 1) << (31 - b);
        let (u, v) = (centre(i, wb), centre(j, hb));
        for (wrap, shift) in [(TextureWrap::Clamp, 0), (TextureWrap::Repeat, tiles * ONE)] {
            let taps = bilinear(uv(u + shift, v - shift), [width, height], wrap).expect("nonempty");
            let context =
                format!("seed {seed:#x} case {case}: texel ({i}, {j}) of {width}x{height}");
            assert_eq!(taps.texels()[0], [i, j], "{context}");
            assert_eq!(taps.weights().map(FixedQ32_32::raw), [ONE, 0, 0, 0], "{context}");
        }
    }
}
