//! Pixel-value contract tests (`docs/topics/optics/test-plan.md`, OPTICS-TP-022 to 032):
//! exact accumulation and its limits, the mean's rounding, the sRGB and linear tone curves
//! against an `f64` reference oracle, 8-bit and 16-bit quantization, and the RGBA8 writer.
//!
//! Floating point appears only here, as a reference oracle; the library is integer-only.

use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_optics::{
    quantize_16, quantize_8, write_rgba8, PixelAccumulator, PixelError, ToneCurve,
    MAX_SAMPLES_PER_PIXEL, MAX_SAMPLE_RADIANCE_RAW, RGBA8_OPAQUE_IDENTITY, UNORM16_IDENTITY,
    UNORM8_IDENTITY,
};
use wasm_bindgen_test::wasm_bindgen_test;

const PIXEL_SEED: u64 = 0xB002_0971_C500_0222;
const ONE: i64 = ONE_RAW;
const MAX: i64 = MAX_SAMPLE_RADIANCE_RAW;
/// First raw input of the sRGB power segment, `ceil(0.0031308 * 2^32)`.
const TOE_END: i64 = 13_446_684;
/// Stated bound on `|SrgbV1.apply(x) - srgb(x)|`, raw Q32.32 display units (`1.49 * 10^-6`).
const SRGB_BOUND_RAW: f64 = 6_400.0;
/// Stated bounds on where a code begins, in raw input units, relative to the correctly
/// rounded code: at most this many inputs later ...
const CODE_LAG_LIMIT: i64 = 9_600;
/// ... and at most this many earlier.
const CODE_LEAD_LIMIT: i64 = 2;

struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    /// A value in `0..=high`.
    fn upto(&mut self, high: i64) -> i64 {
        let span = u64::try_from(high + 1).expect("non-negative bound");
        i64::try_from(self.next() % span).expect("below the span")
    }

    fn rgb(&mut self, high: i64) -> [FixedQ32_32; 3] {
        [fixed(self.upto(high)), fixed(self.upto(high)), fixed(self.upto(high))]
    }
}

fn fixed(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

fn raws(values: [FixedQ32_32; 3]) -> [i64; 3] {
    values.map(FixedQ32_32::raw)
}

fn accumulate(samples: &[[FixedQ32_32; 3]]) -> PixelAccumulator {
    let mut pixel = PixelAccumulator::new();
    for &sample in samples {
        pixel.add_sample(sample).expect("accepted sample");
    }
    pixel
}

/// `raw / 2^32` as `f64`; exact for every raw value used here (all below `2^53`).
#[allow(clippy::cast_precision_loss)]
fn to_f64(raw: i64) -> f64 {
    raw as f64 / 4_294_967_296.0
}

/// The sRGB transfer function (IEC 61966-2-1) in `f64`: the reference oracle.
fn srgb_reference(linear: f64) -> f64 {
    if linear <= 0.0 {
        0.0
    } else if linear >= 1.0 {
        1.0
    } else if linear < 0.003_130_8 {
        12.92 * linear
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

/// The smallest raw input in `0..=2^32` whose value satisfies a monotone predicate.
fn first_raw(reaches: impl Fn(i64) -> bool) -> i64 {
    let (mut low, mut high) = (0_i64, ONE);
    while low < high {
        let mid = low + (high - low) / 2;
        if reaches(mid) {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    low
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_022_golden_accumulation_and_mean() {
    let pixel = accumulate(&[[fixed(1), fixed(2), fixed(3)], [fixed(4), fixed(5), fixed(6)]]);
    assert_eq!((raws(pixel.sums()), pixel.count()), ([5, 7, 9], 2));
    // 2.5, 3.5 and 4.5 raw: ties go to the even neighbour.
    assert_eq!(raws(pixel.resolve().expect("two samples")), [2, 4, 4]);

    let three = accumulate(&[[fixed(4), fixed(5), fixed(6)], [fixed(0); 3], [fixed(0); 3]]);
    // 1.33, 1.67 and 2 raw.
    assert_eq!(raws(three.resolve().expect("three samples")), [1, 2, 2]);

    let four = accumulate(&[
        [fixed(6), fixed(10), fixed(14)],
        [fixed(0); 3],
        [fixed(0); 3],
        [fixed(0); 3],
    ]);
    // 1.5, 2.5 and 3.5 raw.
    assert_eq!(raws(four.resolve().expect("four samples")), [2, 2, 4]);

    let whole = accumulate(&[[fixed(ONE), fixed(3 * ONE), fixed(ONE / 2)]; 16]);
    assert_eq!(raws(whole.sums()), [16 * ONE, 48 * ONE, 8 * ONE]);
    assert_eq!(raws(whole.resolve().expect("sixteen samples")), [ONE, 3 * ONE, ONE / 2]);
    assert_eq!(PixelAccumulator::new(), PixelAccumulator::EMPTY);
    assert_eq!(PixelAccumulator::EMPTY.count(), 0);
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_023_accumulation_is_order_independent() {
    let mut seeds = Seeds(PIXEL_SEED);
    for case in 0..64 {
        let count = 1 + usize::try_from(seeds.upto(255)).expect("small count");
        let high = if case % 2 == 0 { MAX } else { 3 };
        let samples: Vec<[FixedQ32_32; 3]> = (0..count).map(|_| seeds.rgb(high)).collect();
        let forward = accumulate(&samples);

        let mut reversed = samples.clone();
        reversed.reverse();
        let mut shuffled = samples.clone();
        for i in (1..shuffled.len()).rev() {
            let j = usize::try_from(seeds.upto(i64::try_from(i).expect("small"))).expect("small");
            shuffled.swap(i, j);
        }
        assert_eq!(accumulate(&reversed), forward, "reverse; case={case}");
        assert_eq!(accumulate(&shuffled), forward, "shuffled; case={case}");

        let split =
            usize::try_from(seeds.upto(i64::try_from(count).expect("small"))).expect("small");
        let (mut left, right) = (accumulate(&samples[..split]), accumulate(&samples[split..]));
        let mut right_first = right;
        right_first.merge(&left).expect("within the count");
        left.merge(&right).expect("within the count");
        assert_eq!(left, forward, "merge left+right; case={case}");
        assert_eq!(right_first, forward, "merge right+left; case={case}");

        // The sums equal an independent i128 sum.
        for channel in 0..3 {
            let exact: i128 = samples.iter().map(|s| i128::from(s[channel].raw())).sum();
            assert_eq!(i128::from(forward.sums()[channel].raw()), exact, "case={case}");
        }
        assert_eq!(forward.count(), u32::try_from(count).expect("small"), "case={case}");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_024_accumulation_limits_are_exact_at_their_edges() {
    let mut full = accumulate(&vec![[fixed(MAX); 3]; 1024]);
    assert_eq!(MAX_SAMPLES_PER_PIXEL, 1024);
    assert_eq!(MAX, 1 << 52);
    assert_eq!(raws(full.sums()), [1 << 62; 3]);
    assert_eq!(raws(full.resolve().expect("full pixel")), [MAX; 3]);

    let before = full;
    assert_eq!(full.add_sample([fixed(0); 3]), Err(PixelError::TooManySamples));
    assert_eq!(full, before, "a refused sample leaves the pixel unchanged");

    let mut half = accumulate(&vec![[fixed(MAX); 3]; 512]);
    let other_half = half;
    half.merge(&other_half).expect("1024 samples are accepted");
    assert_eq!(half, before);
    let one = accumulate(&[[fixed(0); 3]]);
    assert_eq!(half.merge(&one), Err(PixelError::TooManySamples));
    assert_eq!(half, before, "a refused merge leaves the pixel unchanged");

    let mut pixel = accumulate(&[[fixed(1); 3]]);
    let start = pixel;
    let refusals = [
        ([MAX + 1, 0, 0], PixelError::RadianceTooLarge),
        ([0, 0, MAX + 1], PixelError::RadianceTooLarge),
        ([-1, 0, 0], PixelError::NegativeRadiance),
        ([0, i64::MIN, 0], PixelError::NegativeRadiance),
        ([MAX + 1, -1, 0], PixelError::RadianceTooLarge),
        ([-1, MAX + 1, 0], PixelError::NegativeRadiance),
        ([0, 0, i64::MAX], PixelError::RadianceTooLarge),
    ];
    for (sample, error) in refusals {
        assert_eq!(pixel.add_sample(sample.map(fixed)), Err(error), "{sample:?}");
        assert_eq!(pixel, start, "{sample:?} leaves the pixel unchanged");
    }
    pixel.add_sample([fixed(MAX), fixed(0), fixed(MAX)]).expect("the bounds are accepted");
    assert_eq!(PixelAccumulator::new().resolve(), Err(PixelError::NoSamples));
}

/// The mean by a different route: of the two neighbours of `sum / count`, the closer one,
/// the even one on a tie.
fn mean_oracle(sum: i64, count: u32) -> i64 {
    let (sum, count) = (i128::from(sum), i128::from(count));
    let low = sum.div_euclid(count);
    let (below, above) = (sum - low * count, (low + 1) * count - sum);
    let pick = if below < above || (below == above && low % 2 == 0) { low } else { low + 1 };
    i64::try_from(pick).expect("the mean fits")
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_025_mean_rounds_to_nearest_with_ties_to_even() {
    let mut seeds = Seeds(PIXEL_SEED ^ 0x13);
    for high in [1, 3, 1 << 20, MAX] {
        let mut pixel = PixelAccumulator::new();
        for count in 1..=MAX_SAMPLES_PER_PIXEL {
            pixel.add_sample(seeds.rgb(high)).expect("within the limits");
            let mean = raws(pixel.resolve().expect("non-empty"));
            for (sum, value) in raws(pixel.sums()).into_iter().zip(mean) {
                assert_eq!(value, mean_oracle(sum, count), "sum={sum} count={count}");
            }
            if count.is_power_of_two() {
                // The power-of-two case is a shift with the same rounding.
                let shift = count.trailing_zeros();
                for (sum, value) in raws(pixel.sums()).into_iter().zip(mean) {
                    let (floor, rest, half) =
                        (sum >> shift, sum & (i64::from(count) - 1), i64::from(count) / 2);
                    let up = rest > half || (rest == half && rest != 0 && floor & 1 == 1);
                    assert_eq!(value, floor + i64::from(up), "shift; sum={sum} count={count}");
                }
            }
        }
    }
}

/// The raw input of knot `i`: `2^(31 - e) + j * 2^(24 - e)`, `e = 8 - i / 128`, `j = i % 128`.
fn knot_input(i: i64) -> i64 {
    if i == 9 * 128 {
        return ONE;
    }
    let (e, j) = (8 - i / 128, i % 128);
    (1 << (31 - e)) + j * (1 << (24 - e))
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_026_srgb_knots_are_the_correctly_rounded_curve() {
    let curve = ToneCurve::SrgbV1;
    let mut previous = i64::MIN;
    for i in 0..=9 * 128 {
        let x = knot_input(i);
        let exact = (1.055 * to_f64(x).powf(1.0 / 2.4) - 0.055) * 4_294_967_296.0;
        let tie_distance = (exact - exact.floor() - 0.5).abs();
        assert!(tie_distance > 1e-4, "knot {i} is too close to a tie for the f64 oracle");
        #[allow(clippy::cast_possible_truncation)]
        let rounded = exact.round() as i64;
        let value = curve.apply(fixed(x)).raw();
        // Knots above the toe are table values; the hold below TOE_LAST never reaches a knot.
        if x >= TOE_END {
            assert_eq!(value, rounded, "knot {i} at raw {x}");
        }
        assert!(rounded > previous, "knots increase; knot {i}");
        previous = rounded;
    }
}

/// `(input raw, SrgbV1 display raw, 8-bit code, 16-bit code)`, from an independent integer
/// model of the curve with 60-digit knots.
const SRGB_GOLDEN: &[(i64, i64, u8, u16)] = &[
    (0, 0, 0, 0),
    (1, 13, 0, 0),
    (8_388_608, 108_380_815, 6, 1654),          // 2^-9, in the toe
    (13_446_683, 173_731_144, 10, 2651),        // last toe input, round(12.92 x)
    (13_446_684, 173_731_144, 10, 2651),        // first power input, held at the toe's last value
    (13_446_706, 173_731_144, 10, 2651),        // last held input (table value 173 731 139)
    (13_446_707, 173_731_152, 10, 2651),        // first input above the hold
    (42_949_673, 428_864_202, 25, 6544),        // 0.01
    (773_094_113, 1_981_507_763, 118, 30235),   // 0.18, mid grey
    (919_299_696, 2_147_482_764, 127, 32767),   // 0.2140, just below display one half
    (2_147_483_648, 3_158_334_193, 188, 48192), // 0.5, a knot
    (2_155_884_601, 3_163_854_763, 188, 48276), // inside a segment
    (3_221_225_472, 3_783_114_659, 225, 57725), // 0.75
    (4_294_967_295, 4_294_967_295, 255, 65535), // one raw unit below 1.0
    (4_294_967_296, 4_294_967_296, 255, 65535), // 1.0
    (4_294_967_297, 4_294_967_296, 255, 65535), // one raw unit above 1.0: clamped
    (1 << 52, 4_294_967_296, 255, 65535),       // the sample radiance cap
    (-1, 0, 0, 0),                              // below zero: clamped
    (i64::MIN, 0, 0, 0),
];

/// `(input raw, LinearClampV1 display raw, 8-bit code, 16-bit code)`.
const LINEAR_GOLDEN: &[(i64, i64, u8, u16)] = &[
    (0, 0, 0, 0),
    (1, 1, 0, 0),
    (773_094_113, 773_094_113, 46, 11796),
    (2_147_483_648, 2_147_483_648, 128, 32768),
    (4_294_967_296, 4_294_967_296, 255, 65535),
    (4_294_967_297, 4_294_967_296, 255, 65535),
    (-1, 0, 0, 0),
    (i64::MAX, 4_294_967_296, 255, 65535),
];

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_027_golden_tone_vectors_lock_exact_outputs() {
    for (curve, golden) in
        [(ToneCurve::SrgbV1, SRGB_GOLDEN), (ToneCurve::LinearClampV1, LINEAR_GOLDEN)]
    {
        for &(input, display, code_8, code_16) in golden {
            let linear = fixed(input);
            assert_eq!(curve.apply(linear).raw(), display, "{curve:?} input={input}");
            assert_eq!(curve.encode_8(linear), code_8, "{curve:?} input={input}");
            assert_eq!(curve.encode_16(linear), code_16, "{curve:?} input={input}");
        }
    }
    assert_eq!(ToneCurve::SrgbV1.identity(), "bunny-tone-srgb/1");
    assert_eq!(ToneCurve::LinearClampV1.identity(), "bunny-tone-linear-clamp/1");
}

/// Inputs for the dense sweeps: every `2^12`-th raw value over `[-2^20, 2^32 + 2^20]`, every
/// knot and its neighbours, segment midpoints, and every input within `2^16` of the toe end.
fn dense_inputs() -> Vec<i64> {
    let mut inputs: Vec<i64> = (-(1_i64 << 8)..=(1 << 20) + (1 << 8)).map(|k| k << 12).collect();
    for i in 0..=9 * 128 {
        let x = knot_input(i);
        inputs.extend([x - 1, x, x + 1]);
        if i < 9 * 128 {
            inputs.push(x + (knot_input(i + 1) - x) / 2);
        }
    }
    inputs.extend(TOE_END - (1 << 16)..TOE_END + (1 << 16));
    inputs.sort_unstable();
    inputs.dedup();
    inputs
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_028_srgb_is_within_its_bound_of_the_exact_curve() {
    let mut worst = 0.0_f64;
    for x in dense_inputs() {
        #[allow(clippy::cast_precision_loss)]
        let value = ToneCurve::SrgbV1.apply(fixed(x)).raw() as f64;
        let error = (value - srgb_reference(to_f64(x)) * 4_294_967_296.0).abs();
        assert!(error <= SRGB_BOUND_RAW, "input={x} error={error} raw");
        worst = worst.max(error);
    }
    // The bound is not loose by more than about 3 %.
    assert!(worst > 6_200.0, "worst error {worst} raw");
}

/// Checks where every code of `max_code` begins against the correctly rounded reference.
fn check_code_thresholds(max_code: i64, encode: impl Fn(i64) -> i64) -> (i64, i64) {
    let scale = f64::from(u32::try_from(max_code).expect("small"));
    let (mut lag, mut lead) = (0_i64, 0_i64);
    let mut previous_reference = 0;
    for code in 1..=max_code {
        #[allow(clippy::cast_precision_loss)]
        let boundary = code as f64 - 0.5;
        let reference = first_raw(|x| srgb_reference(to_f64(x)) * scale >= boundary);
        let actual = first_raw(|x| encode(x) >= code);
        lag = lag.max(actual - reference);
        lead = lead.max(reference - actual);
        // Within one code everywhere: code c starts no later than the reference's c + 1 and
        // no earlier than the reference's c - 1.
        assert!(
            actual >= previous_reference || code == 1,
            "code {code} starts before the reference's code {}",
            code - 1
        );
        previous_reference = reference;
        assert!(
            actual - reference <= CODE_LAG_LIMIT,
            "code {code}: starts {} inputs late",
            actual - reference
        );
        assert!(
            reference - actual <= CODE_LEAD_LIMIT,
            "code {code}: starts {} inputs early",
            reference - actual
        );
        if code < max_code {
            #[allow(clippy::cast_precision_loss)]
            let next = first_raw(|x| srgb_reference(to_f64(x)) * scale >= code as f64 + 0.5);
            assert!(actual <= next, "code {code} starts after the reference's code {}", code + 1);
        }
    }
    (lag, lead)
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_029_codes_are_within_one_of_the_correctly_rounded_codes() {
    let curve = ToneCurve::SrgbV1;
    let (lag_8, lead_8) = check_code_thresholds(255, |x| i64::from(curve.encode_8(fixed(x))));
    let (lag_16, lead_16) = check_code_thresholds(65_535, |x| i64::from(curve.encode_16(fixed(x))));
    // Measured with a 60-digit reference: 9 447 and 9 522 inputs late, at most 1 early.
    assert!((9_000..=CODE_LAG_LIMIT).contains(&lag_8), "8-bit lag {lag_8}");
    assert!((9_000..=CODE_LAG_LIMIT).contains(&lag_16), "16-bit lag {lag_16}");
    assert!(lead_8 <= CODE_LEAD_LIMIT && lead_16 <= CODE_LEAD_LIMIT, "leads {lead_8} {lead_16}");
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_030_tone_curves_and_quantization_are_monotone() {
    // Within a segment the power curve is a floored affine function with a non-negative
    // slope, so it is monotone; these checks cover every place where pieces meet.
    for curve in [ToneCurve::SrgbV1, ToneCurve::LinearClampV1] {
        let mut previous = (i64::MIN, 0_u8, 0_u16);
        for x in dense_inputs() {
            let linear = fixed(x);
            let now = (curve.apply(linear).raw(), curve.encode_8(linear), curve.encode_16(linear));
            assert!(
                now.0 >= previous.0 && now.1 >= previous.1 && now.2 >= previous.2,
                "{curve:?} input={x}"
            );
            previous = now;
        }
    }
    // Quantization: every display value within two raw units of every 8-bit and 16-bit
    // boundary, plus a sweep of every 2^12-th raw value.
    let mut displays: Vec<i64> = (-(1_i64 << 8)..=(1 << 20) + (1 << 8)).map(|k| k << 12).collect();
    for (max_code, scale) in [(255_i64, 255_i128), (65_535, 65_535)] {
        for code in 0..max_code {
            let boundary =
                i64::try_from((2 * i128::from(code) + 1) * (1_i128 << 31) / scale).expect("fits");
            displays.extend(boundary - 2..=boundary + 2);
        }
    }
    displays.sort_unstable();
    let mut previous = (0_u8, 0_u16);
    for v in displays {
        let now = (quantize_8(fixed(v)), quantize_16(fixed(v)));
        assert!(now.0 >= previous.0 && now.1 >= previous.1, "display={v}");
        previous = now;
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_031_quantization_rounds_to_nearest_with_ties_to_even() {
    // (display raw, 8-bit code, 16-bit code)
    let golden: &[(i64, u8, u16)] = &[
        (0, 0, 0),
        (32_768, 0, 0),      // 65535 v = 0.49998..: below one half of a 16-bit code
        (32_769, 0, 1),      // 65535 v = 0.50001..
        (8_421_504, 0, 128), // 255 v = 0.49999..; 65535 v = 128.49998..
        (8_421_505, 1, 129), // 255 v = 0.50000..; 65535 v = 128.50001..
        (2_147_483_648, 128, 32768), // 0.5: 127.5 and 32767.5 exactly, ties to even
        (2_147_483_647, 127, 32767),
        (2_147_483_649, 128, 32768),
        (4_294_967_295, 255, 65535),
        (4_294_967_296, 255, 65535),
        (4_294_967_297, 255, 65535), // above 1.0: clamped
        (i64::MAX, 255, 65535),
        (-1, 0, 0), // below 0: clamped
        (i64::MIN, 0, 0),
    ];
    for &(display, code_8, code_16) in golden {
        assert_eq!(quantize_8(fixed(display)), code_8, "display={display}");
        assert_eq!(quantize_16(fixed(display)), code_16, "display={display}");
    }
    // Every code value is the image of its own exact display value c / max_code.
    for code in 0..=255_u8 {
        let display = (i64::from(code) * ONE + 127) / 255;
        assert_eq!(quantize_8(fixed(display)), code, "code {code}");
    }
    for code in 0..=65_535_u16 {
        let display = (i64::from(code) * ONE + 32_767) / 65_535;
        assert_eq!(quantize_16(fixed(display)), code, "code {code}");
    }
    assert_eq!(UNORM8_IDENTITY, "bunny-unorm8/1");
    assert_eq!(UNORM16_IDENTITY, "bunny-unorm16/1");
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_032_rgba8_writes_four_opaque_bytes_or_refuses() {
    let mut out = [7_u8; 4];
    write_rgba8(&mut out, ToneCurve::SrgbV1, [fixed(773_094_113), fixed(ONE / 2), fixed(2 * ONE)])
        .expect("four bytes");
    assert_eq!(out, [118, 188, 255, 255]);
    write_rgba8(&mut out, ToneCurve::LinearClampV1, [fixed(ONE / 2), fixed(-ONE), fixed(ONE)])
        .expect("four bytes");
    assert_eq!(out, [128, 0, 255, 255]);

    // A pixel inside a caller's row buffer.
    let mut row = [0_u8; 12];
    let pixel = row.chunks_exact_mut(4).nth(1).expect("second pixel");
    write_rgba8(pixel, ToneCurve::SrgbV1, [fixed(0); 3]).expect("four bytes");
    assert_eq!(row, [0, 0, 0, 0, 0, 0, 0, 255, 0, 0, 0, 0]);

    for length in [0, 1, 3, 5, 8] {
        let mut buffer = vec![9_u8; length];
        let result = write_rgba8(&mut buffer, ToneCurve::SrgbV1, [fixed(ONE); 3]);
        assert_eq!(result, Err(PixelError::WrongSliceLength { expected: 4, actual: length }));
        assert!(buffer.iter().all(|&byte| byte == 9), "length {length} is left unchanged");
    }
    assert_eq!(RGBA8_OPAQUE_IDENTITY, "bunny-rgba8-opaque/1");
}
