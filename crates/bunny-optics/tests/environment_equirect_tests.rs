//! Evidence for `bunny-environment-equirect/1`: a direction to equirectangular coordinates and
//! back, against an independent interval oracle (`equirect_oracle`), with the axes, the seam,
//! the poles and the refusals constructed by hand, the extremes, the round trips in both
//! directions, and golden vectors from a separate 60-digit decimal model.
//!
//! The oracle shares nothing with the kernel: Machin's `pi`, Taylor `atan`, `sin` and `cos` in
//! 124-bit fixed point, and the horizontal length bracketed by a big-integer square root.
//! Seeded cases print their seed and counts.

// Seeded draws are reduced before each `as` conversion, so every value is in range.
#![allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap, clippy::cast_sign_loss)]

// Only the big integers and the square comparison of the shared module are used here.
mod equirect_oracle;
#[allow(dead_code)]
mod exact;

use bunny_linalg::{FixedVec2, FixedVec3};
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::{FixedAngle, FixedQ32_32};
use bunny_optics::{
    direction_from_equirect, equirect_from_direction, EquirectError, ENVIRONMENT_EQUIRECT_ID,
};
use equirect_oracle::{product, Oracle, SLACK, TURN_BITS};
use exact::{sqrt_product_at_most, Big, Signed};
use wasm_bindgen_test::wasm_bindgen_test;

const EQUIRECT_SEED: u64 = 0xB002_0394_E9A1_0001;
const ONE: i64 = ONE_RAW;
const HALF: i64 = ONE / 2;
const QUARTER: i64 = ONE / 4;

/// The forward bound on `u`, `1/2 + 2^-32` raw units, in oracle turn units (`2^64` per raw).
const U_BOUND: u128 = (1 << 63) + (1 << 32);
/// The forward bound on `v`, `1/2 + 2^-29` raw units, in oracle turn units (`2^63` per raw).
const V_BOUND: u128 = (1 << 62) + (1 << 34);
/// The inverse's oracle slack, in units of `2^-124`.
const INVERSE_SLACK: i128 = 1 << 12;

type Raw3 = [i64; 3];
type Raw2 = [i64; 2];

/// A 64-bit linear congruential generator.
struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    /// A value in `low..=high`.
    fn range(&mut self, low: i64, high: i64) -> i64 {
        let span = (high as i128 - low as i128 + 1) as u128;
        (low as i128 + (u128::from(self.next()) % span) as i128) as i64
    }

    /// Any raw `i64`, from two draws.
    fn any(&mut self) -> i64 {
        ((self.next() << 22) ^ self.next()) as i64
    }

    fn below(&mut self, bound: u32) -> u32 {
        (self.next() % u64::from(bound)) as u32
    }

    /// A nonzero direction of one of six kinds, chosen by `kind`.
    fn direction(&mut self, kind: u32) -> Raw3 {
        loop {
            let d = match kind % 6 {
                0 => [(); 3].map(|()| self.range(-ONE, ONE)),
                1 => [(); 3].map(|()| self.any() >> self.below(64)),
                2 => self.near_pole(),
                3 => self.near_seam(),
                4 => [(); 3].map(|()| self.any()),
                _ => [(); 3].map(|()| self.range(-8, 8)),
            };
            if d != [0; 3] {
                return d;
            }
        }
    }

    fn near_pole(&mut self) -> Raw3 {
        let small = 1_i64 << self.below(21);
        [self.range(-small, small), self.any() >> self.below(32), self.range(-small, small)]
    }

    fn near_seam(&mut self) -> Raw3 {
        let small = 1_i64 << self.below(21);
        let z = (self.any() >> self.below(32)).unsigned_abs().min(i64::MAX as u64) as i64;
        [self.range(-small, small), self.any() >> self.below(64), z]
    }
}

fn fixed(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

fn forward(d: Raw3) -> Result<Raw2, EquirectError> {
    let uv = equirect_from_direction(FixedVec3::new(fixed(d[0]), fixed(d[1]), fixed(d[2])))?;
    Ok([uv.x.raw(), uv.y.raw()])
}

fn inverse(uv: Raw2) -> Result<Raw3, EquirectError> {
    let d = direction_from_equirect(FixedVec2::new(fixed(uv[0]), fixed(uv[1])))?;
    Ok([d.x.raw(), d.y.raw(), d.z.raw()])
}

fn unsigned(raw: i64) -> u128 {
    u128::try_from(raw).expect("a nonnegative coordinate")
}

/// The distance between two turn values on the circle.
fn circular_gap(a: u128, b: u128) -> u128 {
    let full = 1_u128 << TURN_BITS;
    let d = (a % full + full - b % full) % full;
    d.min(full - d)
}

/// The distance between two raw `u` values on the circle of `2^32` raw units.
fn circular_raw(a: i64, b: i64) -> i64 {
    let d = (a - b).rem_euclid(ONE);
    d.min(ONE - d)
}

/// Maps `d`, checks the ranges and both bounds against the oracle, and returns `[u, v]`.
fn check_forward(oracle: &Oracle, d: Raw3) -> Raw2 {
    let [u, v] = forward(d).unwrap_or_else(|e| panic!("{d:?} refused: {e}"));
    assert!((0..ONE).contains(&u), "{d:?}: u = {u} outside [0, 1)");
    assert!((0..=ONE).contains(&v), "{d:?}: v = {v} outside [0, 1]");
    match oracle.longitude(d[0], d[2]) {
        None => assert_eq!(u, 0, "{d:?}: a pole has u = 0"),
        Some(phi) => {
            let gap = circular_gap(unsigned(u) << 64, phi);
            assert!(gap + SLACK <= U_BOUND, "{d:?}: u = {u} is {gap} units from {phi}");
        }
    }
    let (low, high) = oracle.polar_interval(d);
    let at = unsigned(v) << 63;
    for exact in [low, high] {
        let gap = at.abs_diff(exact);
        assert!(gap + SLACK <= V_BOUND, "{d:?}: v = {v} is {gap} units from {exact}");
    }
    [u, v]
}

/// Inverts `uv`, checks each component and the length against the oracle, and returns it.
fn check_inverse(oracle: &Oracle, uv: Raw2) -> Raw3 {
    let d = inverse(uv).unwrap_or_else(|e| panic!("{uv:?} refused: {e}"));
    let (sin_polar, cos_polar) = oracle.sin_cos_turns(unsigned(uv[1]), 33);
    let (sin_long, cos_long) = oracle.sin_cos_turns(u128::from(uv[0] as u64), 32);
    let exact = [-product(sin_polar, sin_long), cos_polar, product(sin_polar, cos_long)];
    let bounds = [5_i128 << 91, 1 << 92, 5 << 91];
    for k in 0..3 {
        let gap = (i128::from(d[k]) << 92) - exact[k];
        assert!(gap.abs() <= bounds[k] + INVERSE_SLACK, "{uv:?}: component {k} of {d:?}");
    }
    let squared: i128 = d.iter().map(|&c| i128::from(c) * i128::from(c)).sum();
    let (low, high) = ((25 * i128::from(ONE) - 92).pow(2), (25 * i128::from(ONE) + 92).pow(2));
    assert!((low..=high).contains(&(625 * squared)), "{uv:?}: |{d:?}| not within 3.68 raw of 1");
    d
}

/// Whether `raw / 2^32` is within `42/5` raw units (Euclidean) of the unit vector along `d`:
/// `(|R|^2 + 2^64 - B^2) |d| <= 2^33 (R . d)`, times 25, decided by comparing squares.
fn near_unit_direction(raw: Raw3, d: Raw3) -> bool {
    let squared: i128 = raw.iter().map(|&c| i128::from(c) * i128::from(c)).sum();
    let a = 25 * squared + 25 * (1_i128 << 64) - 1764;
    let length: u128 = d.iter().map(|&c| u128::from(c.unsigned_abs()).pow(2)).sum();
    let dot: i128 = raw.iter().zip(d).map(|(&r, c)| i128::from(r) * i128::from(c)).sum();
    sqrt_product_at_most(&Signed::new(a), &Big::new(length), &Signed::product(dot, 25, 33))
}

/// The `u` round-trip condition `(2 |du| - 1) m <= 0.58 2^32` for `m = min(v, 1 - v)` raw.
fn u_round_trip_holds(du: i64, v: i64) -> bool {
    let m = i128::from(v.min(ONE - v));
    100 * (2 * i128::from(du) - 1) * m <= 58 * i128::from(ONE)
}

#[test]
#[wasm_bindgen_test]
fn optics_tp_230_seeded_directions_agree_with_the_interval_oracle() {
    println!("seed {EQUIRECT_SEED:#x} + 230");
    let (oracle, mut seeds) = (Oracle::new(), Seeds(EQUIRECT_SEED + 230));
    let cases = 12_000;
    for case in 0..cases {
        check_forward(&oracle, seeds.direction(case));
    }
    println!("{cases} directions checked, {} of each of six kinds", cases / 6);
}

#[test]
#[wasm_bindgen_test]
fn optics_tp_231_axes_and_diagonals_are_exact() {
    let cases: [(Raw3, Raw2); 14] = [
        ([0, 0, -1], [HALF, HALF]),
        ([1, 0, 0], [3 * QUARTER, HALF]),
        ([-1, 0, 0], [QUARTER, HALF]),
        ([0, 0, 1], [0, HALF]),
        ([0, 1, 0], [0, 0]),
        ([0, -1, 0], [0, ONE]),
        ([1, 0, 1], [7 * ONE / 8, HALF]),
        ([-1, 0, 1], [ONE / 8, HALF]),
        ([-1, 0, -1], [3 * ONE / 8, HALF]),
        ([1, 0, -1], [5 * ONE / 8, HALF]),
        ([1, 1, 0], [3 * QUARTER, QUARTER]),
        ([0, -1, 1], [0, 3 * QUARTER]),
        ([-1, -1, 0], [QUARTER, 3 * QUARTER]),
        ([0, 1, -1], [HALF, QUARTER]),
    ];
    for scale in [1, 3, ONE, 1 << 40, 1 << 62] {
        for (d, uv) in cases {
            assert_eq!(forward(d.map(|c| c * scale)), Ok(uv), "{d:?} times {scale}");
        }
    }
    // (3, 5, 4): the horizontal length 5 equals y, so the polar angle is an eighth of a turn.
    for scale in [1, 7, ONE, 1 << 60] {
        let d = [3 * scale, 5 * scale, 4 * scale];
        assert_eq!(forward(d).map(|[_, v]| v), Ok(QUARTER), "{d:?}");
        assert_eq!(forward([4 * scale, -5 * scale, 3 * scale]).map(|[_, v]| v), Ok(3 * QUARTER));
    }
}

#[test]
#[wasm_bindgen_test]
fn optics_tp_232_the_seam_is_at_u_zero_and_continuous_one_raw_unit_either_side() {
    let oracle = Oracle::new();
    for z in [ONE, 1 << 40, 1 << 62, i64::MAX] {
        for y in [0, ONE, -ONE, i64::MIN, i64::MAX] {
            let on = check_forward(&oracle, [0, y, z]);
            assert_eq!(on[0], 0, "on the seam, z = {z}, y = {y}");
            for x in [-1, 1] {
                let side = check_forward(&oracle, [x, y, z]);
                assert!(circular_raw(side[0], 0) <= 1, "x = {x}, z = {z}: u = {}", side[0]);
                assert!((side[1] - on[1]).abs() <= 1, "x = {x} raw moves v");
            }
            let wide = z >> 3;
            let [east, west] = [wide, -wide].map(|x| check_forward(&oracle, [x, y, z])[0]);
            assert!(east > HALF && west < HALF, "{east} and {west} are on the wrong sides");
            assert_eq!(east + west, ONE, "the mirror of the seam");
        }
    }
    // Below z = 0 the plane x = 0 is the image centre, not the seam.
    assert_eq!(forward([0, 0, -ONE]).map(|[u, _]| u), Ok(HALF));
}

#[test]
#[wasm_bindgen_test]
fn optics_tp_233_the_poles_have_u_zero_and_their_neighbours_are_continuous() {
    let oracle = Oracle::new();
    for y in [1, ONE, 1 << 62, i64::MAX] {
        assert_eq!(forward([0, y, 0]), Ok([0, 0]), "straight up, y = {y}");
        assert_eq!(forward([0, -y, 0]), Ok([0, ONE]), "straight down, y = {y}");
    }
    assert_eq!(forward([0, i64::MIN, 0]), Ok([0, ONE]));
    for y in [ONE, -ONE, i64::MAX, i64::MIN] {
        for off in [[1, 0], [-1, 0], [0, 1], [0, -1], [1, 1], [-1, -1]] {
            let [_, v] = check_forward(&oracle, [off[0], y, off[1]]);
            let pole = if y > 0 { 0 } else { ONE };
            assert!((v - pole).abs() <= 1, "one raw unit off the pole, y = {y}: v = {v}");
        }
    }
}

#[test]
#[wasm_bindgen_test]
fn optics_tp_234_the_zero_direction_is_refused_and_any_other_length_accepted() {
    assert_eq!(forward([0, 0, 0]), Err(EquirectError::ZeroDirection));
    assert_eq!(forward([1, 0, 0]), Ok([3 * QUARTER, HALF]));
    assert_eq!(forward([0, 1, 0]), Ok([0, 0]));
    assert_eq!(forward([0, 0, 1]), Ok([0, HALF]));
    assert_eq!(forward([i64::MIN; 3]), forward([-1, -1, -1]));
}

#[test]
#[wasm_bindgen_test]
fn optics_tp_235_doubling_gives_the_same_bits_and_the_mirror_reflects_u() {
    println!("seed {EQUIRECT_SEED:#x} + 235");
    let mut seeds = Seeds(EQUIRECT_SEED + 235);
    for case in 0..4_000 {
        let d = seeds.direction(case).map(|c| c >> 2);
        if d == [0; 3] {
            continue;
        }
        assert_eq!(forward(d.map(|c| c * 2)), forward(d), "{d:?} doubled");
        let [u, v] = forward(d).expect("nonzero");
        let [mirror_u, mirror_v] = forward([-d[0], d[1], d[2]]).expect("nonzero");
        assert_eq!(mirror_v, v, "{d:?} mirrored in x moves v");
        assert_eq!((u + mirror_u) % ONE, 0, "{d:?} mirrored in x: {u} and {mirror_u}");
    }
}

#[test]
#[wasm_bindgen_test]
fn optics_tp_236_extreme_components_neither_wrap_nor_panic() {
    let oracle = Oracle::new();
    let values = [i64::MIN, i64::MIN + 1, -ONE, -1, 0, 1, ONE, i64::MAX];
    let mut checked = 0;
    for x in values {
        for y in values {
            for z in values {
                if [x, y, z] != [0; 3] {
                    check_forward(&oracle, [x, y, z]);
                    checked += 1;
                }
            }
        }
    }
    assert_eq!(checked, 511);
}

#[test]
#[wasm_bindgen_test]
fn optics_tp_237_seeded_coordinates_invert_within_the_bound() {
    println!("seed {EQUIRECT_SEED:#x} + 237");
    let (oracle, mut seeds) = (Oracle::new(), Seeds(EQUIRECT_SEED + 237));
    let cases = 6_000;
    for case in 0..cases {
        let u = if case % 2 == 0 { seeds.range(0, ONE - 1) } else { seeds.any() };
        let v = match case % 7 {
            0 => seeds.range(0, 1 << 16),
            1 => ONE - seeds.range(0, 1 << 16),
            _ => seeds.range(0, ONE),
        };
        check_inverse(&oracle, [u, v]);
    }
    for u in [0, 1, QUARTER, HALF, ONE - 1, i64::MIN, i64::MAX] {
        for v in [0, 1, QUARTER, HALF, ONE - 1, ONE] {
            check_inverse(&oracle, [u, v]);
        }
    }
    println!("{cases} seeded coordinates and 42 constructed ones checked");
}

#[test]
#[wasm_bindgen_test]
fn optics_tp_238_poles_and_axes_invert_exactly_and_u_wraps() {
    println!("seed {EQUIRECT_SEED:#x} + 238");
    let mut seeds = Seeds(EQUIRECT_SEED + 238);
    for _ in 0..500 {
        let u = seeds.any();
        assert_eq!(inverse([u, 0]), Ok([0, ONE, 0]), "straight up at u = {u}");
        assert_eq!(inverse([u, ONE]), Ok([0, -ONE, 0]), "straight down at u = {u}");
        let (base, v) = (u.rem_euclid(ONE), seeds.range(0, ONE));
        let expected = inverse([base, v]);
        assert!(expected.is_ok(), "[{base}, {v}]");
        for turns in [-3, -1, 1, 5] {
            assert_eq!(inverse([base + turns * ONE, v]), expected, "u = {base} + {turns} turns");
        }
    }
    let axes: [(Raw2, Raw3); 4] = [
        ([0, HALF], [0, 0, ONE]),
        ([QUARTER, HALF], [-ONE, 0, 0]),
        ([HALF, HALF], [0, 0, -ONE]),
        ([3 * QUARTER, HALF], [ONE, 0, 0]),
    ];
    for (uv, d) in axes {
        assert_eq!(inverse(uv), Ok(d), "{uv:?}");
    }
}

#[test]
#[wasm_bindgen_test]
fn optics_tp_239_v_outside_the_unit_interval_is_refused() {
    for v in [-1, i64::MIN, ONE + 1, i64::MAX] {
        assert_eq!(inverse([0, v]), Err(EquirectError::VOutsideUnitInterval), "v = {v}");
    }
    assert!(inverse([0, 0]).is_ok() && inverse([0, ONE]).is_ok());
    let errors = [EquirectError::ZeroDirection, EquirectError::VOutsideUnitInterval];
    let messages = errors.map(|e| e.to_string());
    assert!(messages.iter().all(|m| !m.is_empty()) && messages[0] != messages[1]);
    let _: &dyn std::error::Error = &errors[0];
    assert_eq!(ENVIRONMENT_EQUIRECT_ID, "bunny-environment-equirect/1");
}

#[test]
#[wasm_bindgen_test]
fn optics_tp_240_direction_to_coordinates_to_direction_is_within_the_bound() {
    println!("seed {EQUIRECT_SEED:#x} + 240");
    let mut seeds = Seeds(EQUIRECT_SEED + 240);
    let mut constructed = Vec::new();
    for z in [ONE, 1 << 50, i64::MAX] {
        for x in [-1, 0, 1, -(1 << 20), 1 << 20] {
            constructed.extend([[x, 0, z], [x, ONE, z], [x, -(1 << 40), z]]);
        }
    }
    for y in [ONE, -ONE, i64::MAX, i64::MIN] {
        for [x, z] in [[0, 0], [1, 0], [0, -1], [-1, 1], [1 << 10, -(1 << 12)]] {
            constructed.push([x, y, z]);
        }
    }
    let cases = 6_000;
    let seeded = (0..cases).map(|case| seeds.direction(case));
    for d in constructed.clone().into_iter().chain(seeded) {
        let uv = forward(d).expect("nonzero");
        let back = inverse(uv).expect("in range");
        assert!(near_unit_direction(back, d), "{d:?} -> {uv:?} -> {back:?}");
    }
    println!("{cases} seeded and {} seam and pole directions checked", constructed.len());
}

#[test]
#[wasm_bindgen_test]
fn optics_tp_241_coordinates_to_direction_to_coordinates_is_within_the_bound() {
    println!("seed {EQUIRECT_SEED:#x} + 241");
    let mut seeds = Seeds(EQUIRECT_SEED + 241);
    let mut constructed = Vec::new();
    for u in [0, 1, ONE - 1, HALF, QUARTER] {
        for v in [1 << 16, (1 << 16) + 1, QUARTER, HALF, ONE - (1 << 16)] {
            constructed.push([u, v]);
        }
    }
    let cases = 6_000;
    let seeded =
        (0..cases).map(|_| [seeds.range(0, ONE - 1), seeds.range(1 << 16, ONE - (1 << 16))]);
    for [u, v] in constructed.clone().into_iter().chain(seeded) {
        let [u2, v2] = forward(inverse([u, v]).expect("in range")).expect("nonzero");
        assert!((v2 - v).abs() <= 1, "[{u}, {v}] -> v = {v2}");
        let du = circular_raw(u2, u);
        assert!(u_round_trip_holds(du, v), "[{u}, {v}] -> u = {u2}");
    }
    for _ in 0..200 {
        let u = seeds.any();
        assert_eq!(forward(inverse([u, 0]).expect("a pole")), Ok([0, 0]));
        assert_eq!(forward(inverse([u, ONE]).expect("a pole")), Ok([0, ONE]));
    }
    println!("{cases} seeded and {} constructed coordinates checked", constructed.len());
}

/// A pair `(a, b)` near `2^61 (cos, sin)` of `target` units whose `FixedAngle::atan2(b, a)` is
/// exactly `target`, found by a search around the oracle's sine and cosine.
fn pair_at_angle(oracle: &Oracle, target: u64) -> (i64, i64) {
    let (s, c) = oracle.sin_cos_turns(u128::from(target), 64);
    let (a0, b0) = ((c >> 63) as i64, (s >> 63) as i64);
    for da in -16..=16 {
        for db in -16..=16 {
            let (a, b) = (a0 + da, b0 + db);
            if FixedAngle::atan2(fixed(b), fixed(a)).map(FixedAngle::raw) == Some(target) {
                return (a, b);
            }
        }
    }
    panic!("no pair within 16 raw units has the angle {target}")
}

/// `p / 2^32` rounded to nearest, ties to even, written out for the tie cases.
fn shift_to_even(p: i128) -> i64 {
    let (floor, rest) = (p.div_euclid(1 << 32), p.rem_euclid(1 << 32));
    let up = rest > 1 << 31 || (rest == 1 << 31 && floor % 2 != 0);
    (floor + i128::from(up)) as i64
}

#[test]
#[wasm_bindgen_test]
fn optics_tp_242_exact_ties_round_to_even() {
    let oracle = Oracle::new();
    // v: the polar angle (2q + 1) 2^30 units is q + 1/2 raw, which rounds to the even one.
    for q in [1_u64 << 30, (1 << 30) + 1, (1 << 29) + 7, (3 << 30) + 2] {
        let (vertical, horizontal) = pair_at_angle(&oracle, (2 * q + 1) << 30);
        let even = if q % 2 == 0 { q } else { q + 1 } as i64;
        assert_eq!(forward([0, vertical, horizontal]).map(|[_, v]| v), Ok(even), "q = {q}");
        // Four times larger plus one: shifted right, floor(y / 2^s) restores the tied pair.
        let wide = [0, 4 * vertical + 1, 4 * horizontal];
        assert_eq!(forward(wide).map(|[_, v]| v), Ok(even), "q = {q}, {wide:?}");
    }
    // u: atan2(-x, z) of (2p + 1) 2^31 units is p + 1/2 raw; p = 2^32 - 1 rounds up to the seam.
    for p in [5_u64, 6, 1 << 31, (1 << 32) - 1] {
        let target = ((2 * p + 1) << 31).wrapping_neg();
        let (z, x) = pair_at_angle(&oracle, target);
        let even = (if p % 2 == 0 { p } else { p + 1 } % (1 << 32)) as i64;
        assert_eq!(forward([x, 0, z]).map(|[u, _]| u), Ok(even), "p = {p}");
    }
    // The inverse's products: sin theta with 15 trailing zero bits times a sine or cosine with
    // 16 is an exact tie; four of each, whose floors have both parities.
    let polar = |v: i64| FixedAngle::from_raw((v as u64) << 31).sin().raw();
    let v = (ONE / 3..).find(|&v| polar(v).trailing_zeros() == 15).expect("a polar sine");
    let long = |u: i64| FixedAngle::from_turns(fixed(u)).sin_cos();
    let product = |a: FixedQ32_32| i128::from(polar(v)) * i128::from(a.raw());
    let mut parities = [0; 2];
    for (component, pick) in [(0, 0), (2, 1)] {
        let part = |u: i64| if pick == 0 { long(u).0 } else { long(u).1 };
        let tied = (1..).filter(|&u| part(u).raw().trailing_zeros() == 16).take(4);
        for u in tied.collect::<Vec<_>>() {
            let exact = product(part(u));
            let expected =
                if component == 0 { -shift_to_even(exact) } else { shift_to_even(exact) };
            assert_eq!(inverse([u, v]).map(|d| d[component]), Ok(expected), "[{u}, {v}]");
            parities[(exact >> 32).rem_euclid(2) as usize] += 1;
        }
    }
    assert!(parities[0] > 0 && parities[1] > 0, "tie floors of one parity only: {parities:?}");
}

#[test]
#[wasm_bindgen_test]
fn optics_tp_243_golden_vectors_are_bit_exact() {
    // Correctly rounded raw `(u, v)` from a 60-digit decimal model (posted on the pull
    // request); every exact value is at least 0.011 raw units from a rounding midpoint, so the
    // bound forces the nearest value.
    let forward_goldens: [(Raw3, Raw2); 12] = [
        ([1, 2, 3], [4_075_029_790, 1_376_500_434]),
        ([-5, 7, 11], [291_627_740, 1_429_633_609]),
        ([1_431_655_765, 858_993_459, -613_566_757], [2_944_455_506, 1_458_471_508]),
        ([123_456_789, -987_654_321, 192_837_465], [3_905_701_948, 3_983_522_603]),
        ([-(1 << 62), 205_891_132_094_649, (1 << 61) + 12_345], [756_808_418, 2_147_429_056]),
        ([i64::MAX, i64::MIN, 1], [3_221_225_472, 3_221_225_472]),
        ([-1, ONE, ONE], [0, 1_073_741_824]),
        ([1, -ONE, ONE], [0, 3_221_225_472]),
        ([40_000, 3, -50_000], [2_608_713_126, 2_147_419_595]),
        ([-(1 << 40) + 17, 1 << 41, -(1 << 39) - 5], [1_390_675_230, 696_880_688]),
        ([ONE, ONE, ONE], [3_758_096_384, 1_306_042_535]),
        ([-3, -4, -5], [1_778_071_643, 2_969_490_328]),
    ];
    for (d, uv) in forward_goldens {
        assert_eq!(forward(d), Ok(uv), "{d:?}");
    }
    // Pinned outputs of the inverse; TP-237's oracle bounds them, these pin their bits.
    let inverse_goldens: [(Raw2, Raw3); 6] = [
        ([ONE / 8, QUARTER], [-2_147_483_648, 3_037_000_500, 2_147_483_648]),
        ([1_431_655_765, 1_431_655_765], [-3_221_225_472, 2_147_483_649, -1_859_775_391]),
        ([123_456_789, 987_654_321], [-510_116_937, 3_222_093_392, 2_793_678_392]),
        ([ONE - 1, 1], [0, ONE, 3]),
        ([-858_993_459, ONE - 12_345], [36_885, -ONE, 11_985]),
        ([HALF + 1, HALF - 1], [6, 3, -ONE]),
    ];
    for (uv, d) in inverse_goldens {
        assert_eq!(inverse(uv), Ok(d), "{uv:?}");
    }
}
