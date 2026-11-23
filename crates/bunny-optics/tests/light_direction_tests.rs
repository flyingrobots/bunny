//! Evidence for `bunny-light-direction/1`: `DirectionalLight::from_angles` and
//! `DirectionalLight::from_direction`. Hand-derived directions on the axes, the diagonals and
//! the poles; seeded angles checked bit for bit against an integer model of the chapter's steps
//! and within the stated bound of an exact series oracle; the division's rounding on
//! hand-built ties; the refusals.
//!
//! The oracles are written here from the chapter's formulas and share no helper with the
//! kernel. The step model takes `FixedAngle::sin_cos` as its input, as the kernel does; the
//! exact oracle (`light_angle_oracle`) does not use it.

mod light_angle_oracle;

use bunny_linalg::FixedVec3;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::{FixedAngle, FixedQ32_32};
use bunny_optics::{
    DirectionalLight, LightDirectionError, ShadingError, LIGHT_DIRECTION_ID, MAX_LIGHT_RAW,
};
use wasm_bindgen_test::wasm_bindgen_test;

use light_angle_oracle::{Oracle, SCALED_BITS};

const ONE: i64 = ONE_RAW;
const QUARTER: u64 = 1 << 62;
const EIGHTH: u64 = 1 << 61;
const LIGHT_DIRECTION_SEED: u64 = 0xB002_0313_A2E1_0001;
/// The seeded angle count the issue asks for.
const SEEDED_ANGLES: usize = 100_000;
/// The per-component bound in thousandths of a raw unit: `5 sqrt(3) + 1/2 + 10^-8 < 9.161`.
const COMPONENT_BOUND_MILLI: i128 = 9_161;
/// The angular bound on `sin(angle)` in thousandths of `2^-32`: `sqrt(13.5) + sqrt(3)/2 < 4.55`.
const ANGLE_BOUND_MILLI: f64 = 4_550.0;

type Raw3 = [i64; 3];

fn fixed(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

fn vec3(raw: Raw3) -> FixedVec3 {
    FixedVec3::new(fixed(raw[0]), fixed(raw[1]), fixed(raw[2]))
}

fn raw3(v: FixedVec3) -> Raw3 {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

fn white() -> [FixedQ32_32; 3] {
    [FixedQ32_32::ONE; 3]
}

/// The direction `from_angles` gives for `(azimuth, elevation)` in raw turn units.
fn from_angles(azimuth: u64, elevation: u64) -> Raw3 {
    let light =
        DirectionalLight::from_angles(FixedAngle::from_raw(azimuth), FixedAngle::from_raw(elevation), white())
            .expect("every angle pair gives a light");
    raw3(light.toward_light())
}

/// `n / d` for `d > 0`, rounded to nearest with ties to even, by floor division: written here,
/// not shared with the kernel.
fn round_even(n: i128, d: i128) -> i128 {
    let (floor, remainder) = (n.div_euclid(d), n.rem_euclid(d));
    let up = 2 * remainder > d || (2 * remainder == d && floor.rem_euclid(2) == 1);
    floor + i128::from(up)
}

/// The chapter's steps on raw integers: the two products rounded to Q32.32, then each component
/// times `2^32` over the largest magnitude, rounded.
fn step_model(azimuth: u64, elevation: u64) -> Raw3 {
    let (sin_a, cos_a) = FixedAngle::from_raw(azimuth).sin_cos();
    let (sin_e, cos_e) = FixedAngle::from_raw(elevation).sin_cos();
    let product = |a: FixedQ32_32, b: FixedQ32_32| {
        round_even(i128::from(a.raw()) * i128::from(b.raw()), 1 << 32)
    };
    let v = [product(cos_e, cos_a), i128::from(sin_e.raw()), product(cos_e, sin_a)];
    scale_model(v)
}

/// Each component times `2^32` over the largest magnitude, rounded to nearest, ties to even.
fn scale_model(v: [i128; 3]) -> Raw3 {
    let largest = v.iter().map(|c| c.abs()).max().expect("three components");
    v.map(|c| i64::try_from(round_even(c << 32, largest)).expect("at most 1.0"))
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_290_hand_derived_directions_on_the_axes_and_diagonals() {
    assert_eq!(LIGHT_DIRECTION_ID, "bunny-light-direction/1");
    let half = 2 * QUARTER;
    let three_quarters = 3 * QUARTER;
    // Azimuth 0 and elevation 0 point along +x; a quarter azimuth along +z; a quarter
    // elevation straight up. sin_cos is exact at quarter turns, so these are exact.
    let cases: [(u64, u64, Raw3); 12] = [
        (0, 0, [ONE, 0, 0]),
        (0, QUARTER, [0, ONE, 0]),
        (QUARTER, 0, [0, 0, ONE]),
        (half, 0, [-ONE, 0, 0]),
        (three_quarters, 0, [0, 0, -ONE]),
        (0, three_quarters, [0, -ONE, 0]),
        // Past a quarter turn the formula continues over the pole: elevation a half turn
        // with azimuth 0 points along -x, with azimuth a quarter along -z.
        (0, half, [-ONE, 0, 0]),
        (QUARTER, half, [0, 0, -ONE]),
        // An eighth turn: sin and cos read the same table entry, so they are equal raw values
        // and both components divide to exactly 1.0.
        (0, EIGHTH, [ONE, ONE, 0]),
        (0, 3 * EIGHTH, [-ONE, ONE, 0]),
        (EIGHTH, 0, [ONE, 0, ONE]),
        // Below the horizon is a valid light.
        (0, EIGHTH.wrapping_neg(), [ONE, -ONE, 0]),
    ];
    for (azimuth, elevation, expected) in cases {
        assert_eq!(from_angles(azimuth, elevation), expected, "azimuth {azimuth:#x} elevation {elevation:#x}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_291_at_the_poles_azimuth_has_no_effect() {
    // cos e is exactly 0 at a quarter and three quarters of a turn, so x and z are exactly 0
    // for every azimuth.
    let mut seeds = Seeds(LIGHT_DIRECTION_SEED ^ 0x0291);
    let multiples = (0..16_u64).map(|k| k << 60);
    let azimuths: Vec<u64> = multiples.chain((0..4_096).map(|_| seeds.angle())).collect();
    for &azimuth in &azimuths {
        assert_eq!(from_angles(azimuth, QUARTER), [0, ONE, 0], "azimuth {azimuth:#x}");
        assert_eq!(from_angles(azimuth, 3 * QUARTER), [0, -ONE, 0], "azimuth {azimuth:#x}");
    }
    // Near the pole every azimuth stays within the bound of the exact direction, which itself
    // turns with the azimuth by at most cos e.
    let oracle = Oracle::new();
    for offset in [1_u64, 1 << 10, 1 << 20, 1 << 30, 1 << 40, 1 << 50] {
        for &azimuth in azimuths.iter().step_by(16) {
            for elevation in [QUARTER - offset, QUARTER + offset, 3 * QUARTER + offset] {
                check_bound(&oracle, azimuth, elevation);
            }
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_293_seeded_angles_follow_the_steps_and_build_a_valid_light() {
    println!("OPTICS-TP-293 seed {LIGHT_DIRECTION_SEED:#x}, {SEEDED_ANGLES} angle pairs");
    let mut seeds = Seeds(LIGHT_DIRECTION_SEED);
    let mut below_horizon = 0;
    for case in 0..SEEDED_ANGLES {
        let (azimuth, elevation) = seeds.angle_pair(case);
        let context = format!("case {case}: azimuth {azimuth:#x} elevation {elevation:#x}");
        let direction = from_angles(azimuth, elevation);
        let largest = direction.iter().map(|c| c.unsigned_abs()).max().expect("three");
        assert_eq!(largest, ONE.unsigned_abs(), "{context}");
        assert_eq!(direction, step_model(azimuth, elevation), "{context}");
        let rebuilt = DirectionalLight::new(vec3(direction), white()).expect("accepted by new");
        assert_eq!(raw3(rebuilt.toward_light()), direction, "{context}");
        below_horizon += usize::from(direction[1] < 0);
    }
    println!("OPTICS-TP-293 below the horizon: {below_horizon}");
    assert!(below_horizon >= SEEDED_ANGLES / 4, "a quarter of the cases are below the horizon");
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_294_seeded_angles_are_within_the_bound_of_the_exact_direction() {
    println!("OPTICS-TP-294 seed {LIGHT_DIRECTION_SEED:#x}, {SEEDED_ANGLES} angle pairs");
    let oracle = Oracle::new();
    let mut seeds = Seeds(LIGHT_DIRECTION_SEED);
    let (mut worst_component, mut worst_angle) = (0_i128, 0.0_f64);
    for case in 0..SEEDED_ANGLES {
        let (azimuth, elevation) = seeds.angle_pair(case);
        let (component, angle) = check_bound(&oracle, azimuth, elevation);
        worst_component = worst_component.max(component);
        worst_angle = worst_angle.max(angle);
    }
    let scale = f64::from(1_u32 << 31) * f64::from(1_u32 << 1);
    #[allow(clippy::cast_precision_loss)] // A report of the measured margin, not an oracle.
    let measured = worst_component as f64 / scale;
    println!(
        "OPTICS-TP-294 measured: largest component error {measured:.4} raw, \
         sin(angle) {worst_angle:.4} x 2^-32"
    );
}

/// Asserts the per-component and angular bounds for one angle pair; returns the component
/// error in units of `2^-64` and `sin(angle)` in units of `2^-32`.
fn check_bound(oracle: &Oracle, azimuth: u64, elevation: u64) -> (i128, f64) {
    let direction = from_angles(azimuth, elevation).map(i128::from);
    let exact = oracle.scaled(azimuth, elevation);
    let context = format!("azimuth {azimuth:#x} elevation {elevation:#x}: {direction:?}");
    let mut worst = 0;
    for (&got, &want) in direction.iter().zip(&exact) {
        // Two units of 2^-64 cover the oracle's own floors.
        let error = ((got << (SCALED_BITS - 32)) - want).abs().saturating_sub(2);
        assert!(1_000 * error <= COMPONENT_BOUND_MILLI << 32, "component error {error}: {context}");
        worst = worst.max(error);
    }
    let angle = sin_angle(direction, exact);
    assert!(1_000.0 * angle <= ANGLE_BOUND_MILLI, "sin(angle) {angle} x 2^-32: {context}");
    (worst, angle)
}

/// `|q x t| / (|q| |t|)` in units of `2^-32`: the cross product is exact in `i128`; only the
/// final ratio is evaluated in `f64`, whose relative error near `1e-16` is far below the margin.
#[allow(clippy::cast_precision_loss)]
fn sin_angle(q: [i128; 3], t: [i128; 3]) -> f64 {
    let cross = [q[1] * t[2] - q[2] * t[1], q[2] * t[0] - q[0] * t[2], q[0] * t[1] - q[1] * t[0]];
    let norm = |v: [i128; 3]| v.iter().map(|&c| (c as f64) * (c as f64)).sum::<f64>().sqrt();
    norm(cross) / (norm(q) * norm(t)) * f64::from(1_u32 << 31) * 2.0
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_295_the_division_rounds_to_nearest_with_ties_to_even() {
    let m = 1_i64 << 33;
    let cases: [(Raw3, Raw3); 9] = [
        // Over 2^33: 1/2 and 3/2 are ties, to 0 and 2; 5/2 to 2, 7/2 to 4.
        ([m, 1, 3], [ONE, 0, 2]),
        ([m, 5, 7], [ONE, 2, 4]),
        // The rule is symmetric: a negative tie rounds as its magnitude does.
        ([-m, -1, -3], [-ONE, 0, -2]),
        // One raw unit either side of the tie at 1/2: just below rounds to 0, just above to 1.
        ([m + 1, 1, 0], [ONE, 0, 0]),
        ([m - 1, 1, 0], [ONE, 1, 0]),
        // The largest component, of either sign, is exactly 1.0; equal largest ones both are.
        ([0, 0, -1], [0, 0, -ONE]),
        ([5, -5, 1], [ONE, -ONE, 858_993_459]),
        // The extremes of i64: 2^63 - 1 over 2^63 rounds up to 1.0, 1 over 2^63 to 0.
        ([i64::MIN, i64::MAX, 1], [-ONE, ONE, 0]),
        // A direction already scaled is returned as given.
        ([ONE, -ONE / 3, 7], [ONE, -ONE / 3, 7]),
    ];
    for (given, expected) in cases {
        let light = DirectionalLight::from_direction(vec3(given), white()).expect("nonzero");
        assert_eq!(raw3(light.toward_light()), expected, "given {given:?}");
    }
    // A direction already scaled gives the light `new` gives, bit for bit.
    let given = vec3([ONE, -ONE / 3, 7]);
    let built = DirectionalLight::from_direction(given, white()).expect("nonzero");
    assert_eq!(built, DirectionalLight::new(given, white()).expect("normalized"));
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_296_seeded_directions_are_within_half_a_raw_unit_of_the_exact_quotient() {
    const CASES: usize = 20_000;
    let seed = LIGHT_DIRECTION_SEED ^ 0x0296;
    println!("OPTICS-TP-296 seed {seed:#x}, {CASES} vectors");
    let mut seeds = Seeds(seed);
    let mut multiples = 0;
    for case in 0..CASES {
        let given = seeds.vector();
        let v = given.map(i128::from);
        let light = DirectionalLight::from_direction(vec3(given), white()).expect("nonzero");
        let got = raw3(light.toward_light());
        let largest = v.iter().map(|c| c.abs()).max().expect("three");
        for (&q, &c) in got.iter().zip(&v) {
            // |q - c 2^32 / m| <= 1/2, decided exactly: |2 (q m - c 2^32)| <= m.
            let twice = 2 * (i128::from(q) * largest - (c << 32));
            assert!(twice.abs() <= largest, "case {case}: {given:?} gives {got:?}");
        }
        assert_eq!(got, scale_model(v), "case {case}: {given:?}");
        // Any positive integer multiple is the same rational quotient, so the same bits.
        let k = seeds.range(2, 1 << 20);
        if let Some(scaled) = given.iter().map(|c| c.checked_mul(k)).collect::<Option<Vec<_>>>() {
            let scaled = vec3([scaled[0], scaled[1], scaled[2]]);
            let again = DirectionalLight::from_direction(scaled, white()).expect("nonzero");
            assert_eq!(raw3(again.toward_light()), got, "case {case}: {given:?} times {k}");
            multiples += 1;
        }
    }
    println!("OPTICS-TP-296 multiples compared: {multiples}");
    assert!(multiples >= CASES / 4);
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_297_zero_and_out_of_range_irradiance_are_refused_in_order() {
    let zero = vec3([0; 3]);
    let up = vec3([0, 3, 0]);
    let refused = |direction, irradiance| DirectionalLight::from_direction(direction, irradiance);
    assert_eq!(refused(zero, white()), Err(LightDirectionError::ZeroDirection));
    // The direction is checked before the irradiance.
    assert_eq!(refused(zero, [fixed(-1); 3]), Err(LightDirectionError::ZeroDirection));
    let irradiance_cases = [
        (fixed(-1), Err(ShadingError::NegativeRadiance)),
        (fixed(MAX_LIGHT_RAW + 1), Err(ShadingError::RadianceTooLarge)),
        (fixed(MAX_LIGHT_RAW), Ok(())),
        (fixed(0), Ok(())),
    ];
    for (channel, expected) in irradiance_cases {
        let irradiance = [FixedQ32_32::ONE, channel, FixedQ32_32::ONE];
        let from_vector = refused(up, irradiance).map(|_| ());
        let azimuth = FixedAngle::from_raw(0x1234_5678_9ABC_DEF0);
        let from_angles = DirectionalLight::from_angles(azimuth, FixedAngle::ZERO, irradiance);
        let expected = expected.map_err(LightDirectionError::Light);
        assert_eq!(from_vector, expected, "irradiance {channel:?}");
        assert_eq!(from_angles.map(|_| ()), expected, "irradiance {channel:?}");
    }
    let errors = [
        LightDirectionError::ZeroDirection,
        LightDirectionError::Light(ShadingError::NegativeRadiance),
        LightDirectionError::Light(ShadingError::RadianceTooLarge),
    ];
    let messages: Vec<String> = errors.iter().map(ToString::to_string).collect();
    for (k, message) in messages.iter().enumerate() {
        assert!(!message.is_empty());
        assert!(messages[k + 1..].iter().all(|other| other != message), "{message}");
    }
    let _: &dyn std::error::Error = &errors[0];
}

struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    /// Any raw angle.
    fn angle(&mut self) -> u64 {
        (self.next() << 32) ^ self.next()
    }

    /// A value in `low..=high`.
    fn range(&mut self, low: i64, high: i64) -> i64 {
        let span = u64::try_from(high - low + 1).expect("non-empty range");
        low + i64::try_from(self.next() % span).expect("below the span")
    }

    /// A signed offset of up to `2^40` turn units.
    fn small(&mut self) -> u64 {
        let bits = u32::try_from(self.next() % 41).expect("below 41");
        let offset = self.next() & ((1_u64 << bits) - 1);
        if self.next() % 2 == 0 {
            offset
        } else {
            offset.wrapping_neg()
        }
    }

    /// An angle pair: one in eight near a pole, one in eight with the azimuth near a multiple
    /// of an eighth turn (where x and z trade the largest magnitude), one in eight with the
    /// elevation near an eighth turn (where y does), the rest uniform.
    fn angle_pair(&mut self, case: usize) -> (u64, u64) {
        let eighths = |seeds: &mut Self| (seeds.next() % 8) << 61;
        match case % 8 {
            0 => (self.angle(), (QUARTER | ((self.next() % 2) << 63)).wrapping_add(self.small())),
            1 => (eighths(self).wrapping_add(self.small()), self.angle()),
            2 => (self.angle(), eighths(self).wrapping_add(self.small())),
            _ => (self.angle(), self.angle()),
        }
    }

    /// A nonzero vector whose components have seeded magnitudes from one raw unit to `i64`.
    fn vector(&mut self) -> Raw3 {
        loop {
            let mut component = || {
                let bits = u32::try_from(self.next() % 64).expect("below 64");
                let magnitude = i64::try_from(self.next() & ((1_u64 << bits) - 1)).expect("63 bits");
                if self.next() % 2 == 0 {
                    magnitude
                } else {
                    -magnitude
                }
            };
            let v = [component(), component(), component()];
            if v != [0; 3] {
                return v;
            }
        }
    }
}
