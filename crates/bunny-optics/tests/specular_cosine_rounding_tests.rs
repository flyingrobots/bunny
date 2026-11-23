//! Evidence for the incidence cosine of `bunny-specular-reflection/1` (OPTICS-REQ-090): the
//! exact `-(u . n)` is rounded once to nearest raw unit, ties to even, before Schlick's term.
//!
//! Each case is a kernel hit whose exact cosine lies on a rounding boundary or one unit of
//! `2^-64` beside it, on a plane whose frame normal is not an axis, at a cosine low enough that
//! the throughput changes with one raw unit of cosine. Every expected value is derived in the
//! comments by hand from the pinned inputs and recomputed here by an oracle that shares no code
//! with the crate: exact `i128` for the cosine and big integers for Schlick's term. The
//! throughputs were also checked against a separate Python model (unbounded integers).

mod exact;

use std::cmp::Ordering;

use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_optics::{Albedo, SurfaceHit, TriangleSurface};
use bunny_query::{TraceRay, TraceTriangle};
use wasm_bindgen_test::wasm_bindgen_test;

use exact::{within, Big, Signed};

const ONE: i64 = ONE_RAW;
const MAX: FixedQ32_32 = FixedQ32_32::from_raw(i64::MAX);
/// Raw `2^32` as a wide value, and half of it: the rounding boundary of the cosine.
const UNIT: i128 = 1 << 32;
const HALF: i128 = 1 << 31;
/// The base reflectance of the three channels: 0, 0.04 and 0.5.
const F0: Raw3 = [0, 171_798_692, 1 << 31];

type Raw3 = [i64; 3];

/// The plane `x - y + z = 0`, centred on the origin. Its edges' cross product is
/// `(48, -48, 48)`, and the frame normal facing the rays below is `g (1, -1, 1)` with
/// `g = 2 479 700 525` (`2^32 / sqrt(3)` is `2 479 700 524.506...`), asserted as a premise.
const SLANT: [Raw3; 3] = [[4 * ONE, 4 * ONE, 0], [-4 * ONE, 0, 4 * ONE], [0, -4 * ONE, -4 * ONE]];
const G: i64 = 2_479_700_525;
/// The plane `x = y`, centred on the origin. Its cross product is `(48, -48, 0)`, and the
/// facing frame normal is `h (1, -1, 0)` with `h = 3 037 000 500 = 4 * 759 250 125`.
const DIAGONAL: [Raw3; 3] =
    [[4 * ONE, 4 * ONE, 0], [-4 * ONE, -4 * ONE, 4 * ONE], [0, 0, -4 * ONE]];
const H: i64 = 3_037_000_500;

/// One pinned hit: the ray from `-d` along `d` meets the triangle at the origin.
struct Case {
    name: &'static str,
    triangle: [Raw3; 3],
    normal: Raw3,
    /// The incoming direction; its largest component is exactly 1.0, so the prescale to
    /// `[2^61, 2^62)` is the exact shift by 29.
    d: Raw3,
    /// The unit vector along `d` (a premise: `FixedUnitVec3::new` of the prescaled `d`, and
    /// within one raw unit of the exact unit vector).
    u: Raw3,
    /// The exact `-(u . n)`, in units of `2^-64`.
    exact: i128,
    /// The cosine the documented rule gives: `exact / 2^32` to nearest, ties to even.
    cosine: i64,
    /// The cosines other rules give where they differ from `cosine`, each named.
    wrong: &'static [(&'static str, i64)],
    /// The throughput for `F0`.
    throughput: Raw3,
}

const CASES: [Case; 5] = [
    // u_x - u_y + u_z = -3 371 901 746 - 674 380 349 + 2 573 370 132 = -1 472 911 963, so
    // -(u . n) = 1 472 911 963 g = 3 652 380 567 929 880 575 = 850 386 118 * 2^32 + 2^31 - 1:
    // one unit of 2^-64 under the tie, so 850 386 118; a ceiling gives ...119.
    Case {
        name: "below a tie",
        triangle: SLANT,
        normal: [G, -G, G],
        d: [-ONE, 858_993_459, 3_277_835_889],
        u: [-3_371_901_746, 674_380_349, 2_573_370_132],
        exact: 3_652_380_567_929_880_575,
        cosine: 850_386_118,
        wrong: &[("ceiling", 850_386_119)],
        throughput: [1_425_091_258, 1_539_886_300, 2_860_029_277],
    },
    // u_x - u_y + u_z = -3 619 490 936 - 723 898 187 + 2 195 905 475 = -2 147 483 648 = -2^31,
    // so -(u . n) = 2^31 g = 1 239 850 262 * 2^32 + 2^31: a tie on an even floor, so
    // 1 239 850 262; a ceiling, ties away from zero and ties to odd give ...263.
    Case {
        name: "a tie on an even floor (slant)",
        triangle: SLANT,
        normal: [G, -G, G],
        d: [-ONE, 858_993_459, 2_605_709_578],
        u: [-3_619_490_936, 723_898_187, 2_195_905_475],
        exact: 5_325_116_329_374_515_200,
        cosine: 1_239_850_262,
        wrong: &[
            ("ceiling", 1_239_850_263),
            ("ties away", 1_239_850_263),
            ("ties to odd", 1_239_850_263),
        ],
        throughput: [782_167_584, 922_679_573, 2_538_567_440],
    },
    // u_x - u_y + u_z = -3 833 613 914 - 766 722 783 + 1 778 281 364 = -2 822 055 333, so
    // -(u . n) = 2 822 055 333 g = 6 997 852 090 819 149 825 = 1 629 314 406 * 2^32 + 2^31 + 1:
    // one unit of 2^-64 over the tie, so 1 629 314 407; a floor (or truncation) gives ...406.
    Case {
        name: "above a tie",
        triangle: SLANT,
        normal: [G, -G, G],
        d: [-ONE, 858_993_459, 1_992_287_297],
        u: [-3_833_613_914, 766_722_783, 1_778_281_364],
        exact: 6_997_852_090_819_149_825,
        cosine: 1_629_314_407,
        wrong: &[("floor", 1_629_314_406)],
        throughput: [395_529_188, 551_506_712, 2_345_248_242],
    },
    // u_x - u_y = -1 200 274 985 - 410 337 751 = -1 610 612 736 = -3 * 2^29, so
    // -(u . n) = 3 * 2^29 h = 3 h / 8 * 2^32 = 1 138 875 187.5 * 2^32: a tie on an odd floor,
    // so 1 138 875 188; a floor, ties toward zero and ties to odd give ...187.
    Case {
        name: "a tie on an odd floor (diagonal)",
        triangle: DIAGONAL,
        normal: [H, -H, 0],
        d: [-1_256_316_729, 429_496_730, ONE],
        u: [-1_200_274_985, 410_337_751, 4_103_377_507],
        exact: 4_891_431_684_538_368_000,
        cosine: 1_138_875_188,
        wrong: &[
            ("floor", 1_138_875_187),
            ("ties toward zero", 1_138_875_187),
            ("ties to odd", 1_138_875_187),
        ],
        throughput: [920_256_516, 1_055_244_947, 2_607_611_906],
    },
    // u_x - u_y = -109 644 975 - 427 225 937 = -536 870 912 = -2^29, so -(u . n) = 2^29 h =
    // h / 8 * 2^32 = 379 625 062.5 * 2^32: a tie on an even floor at cosine 0.088, so
    // 379 625 062; a ceiling, ties away from zero and ties to odd give ...063.
    Case {
        name: "a tie on an even floor (diagonal)",
        triangle: DIAGONAL,
        normal: [H, -H, 0],
        d: [-110_227_761, 429_496_730, ONE],
        u: [-109_644_975, 427_225_937, 4_272_259_362],
        exact: 1_630_477_228_179_456_000,
        cosine: 379_625_062,
        wrong: &[
            ("ceiling", 379_625_063),
            ("ties away", 379_625_063),
            ("ties to odd", 379_625_063),
        ],
        throughput: [2_704_015_647, 2_767_653_713, 3_499_491_472],
    },
];

fn fixed(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

fn vec3(raw: Raw3) -> FixedVec3 {
    FixedVec3::new(fixed(raw[0]), fixed(raw[1]), fixed(raw[2]))
}

fn raw3(v: FixedVec3) -> Raw3 {
    [v.x.raw(), v.y.raw(), v.z.raw()]
}

fn big(value: i128) -> Big {
    Big::new(u128::try_from(value).expect("non-negative"))
}

/// `2^shift` as a big integer.
fn power_of_two(shift: u32) -> Big {
    (0..shift / 64).fold(Big::new(1 << (shift % 64)), |b, _| b.mul(&Big::new(1 << 64)))
}

/// `value / 2^32` to nearest, ties to even, for `value >= 0`: the documented rule, written
/// from its definition with a Euclidean quotient and remainder.
fn nearest_even(value: i128) -> i128 {
    let (quotient, remainder) = (value.div_euclid(UNIT), value.rem_euclid(UNIT));
    match remainder.cmp(&HALF) {
        Ordering::Less => quotient,
        Ordering::Greater => quotient + 1,
        Ordering::Equal => quotient + quotient % 2,
    }
}

/// Schlick's term as `bunny-specular-reflection/1` defines it, at raw `f0` and raw `cosine`
/// (each in `0..=2^32`): `w = m^5 / 2^128` to nearest (no tie can occur) with `m = 2^32 -
/// cosine`, then `(f0 2^32 + (2^32 - f0) w) / 2^32` to nearest, ties to even.
fn schlick_oracle(f0: i64, cosine: i64) -> i64 {
    let m = big(UNIT - i128::from(cosine));
    let fifth = m.mul(&m).mul(&m).mul(&m).mul(&m);
    let (mut low, mut high) = (0_i128, UNIT + 1);
    while high - low > 1 {
        let mid = i128::midpoint(low, high);
        if big(mid).mul(&power_of_two(128)).compare(&fifth) == Ordering::Greater {
            high = mid;
        } else {
            low = mid;
        }
    }
    let half_up = big(2 * low + 1).mul(&power_of_two(127));
    let order = fifth.compare(&half_up);
    assert_ne!(order, Ordering::Equal, "no tie in the weight");
    let weight = low + i128::from(order == Ordering::Greater);
    let f0 = i128::from(f0);
    i64::try_from(nearest_even(f0 * UNIT + (UNIT - f0) * weight)).expect("in 0..=1.0")
}

/// The kernel's hit and sample for `case`, after asserting the case's premises.
fn sample(case: &Case) -> Raw3 {
    let ray = TraceRay::new(vec3(case.d.map(|c| -c)), vec3(case.d)).expect("a trace ray");
    let [a, b, c] = case.triangle.map(vec3);
    let triangle = TraceTriangle::new(a, b, c).expect("a triangle");
    let surface = TriangleSurface::new(&triangle).expect("a surface");
    let hit = ray.hit_triangle(&triangle, fixed(0), MAX).expect("the ray hits");
    let hit = SurfaceHit::new(&surface, &ray, &hit).expect("a surface hit");
    assert_eq!(raw3(hit.frame().normal()), case.normal, "{}: the frame normal", case.name);
    let reflectance = Albedo::new(F0.map(fixed)).expect("channels in 0..=1");
    let sample = hit.try_specular_reflection(&ray, &reflectance).expect("a mirror sample");
    sample.throughput().map(FixedQ32_32::raw)
}

/// The premises on `u`: the documented normalization of `d`, and within one raw unit of the
/// exact unit vector `d / |d|` (decided by exact squares).
fn assert_unit_premise(case: &Case) {
    let prescaled = vec3(case.d.map(|c| c << 29));
    let u = FixedUnitVec3::new(prescaled).expect("a unit vector").into_inner();
    assert_eq!(raw3(u), case.u, "{}: the unit vector along d", case.name);
    let exact = case.d.map(|c| Signed::new(i128::from(c)));
    assert!(within(case.u, &exact, 1, &Signed::new(0)), "{}: u within one raw unit", case.name);
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_180_the_incidence_cosine_rounds_once_to_nearest_with_ties_to_even() {
    // Every case is checked before the verdict, so a wrong rule names each case it breaks.
    let wrong: Vec<String> = CASES
        .iter()
        .filter_map(|case| {
            let throughput = sample(case);
            (throughput != case.throughput).then(|| format!("{}: {throughput:?}", case.name))
        })
        .collect();
    assert!(wrong.is_empty(), "throughputs off the documented rounding: {wrong:#?}");
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_181_each_pinned_case_is_derived_independently_of_the_kernel() {
    for case in &CASES {
        assert_unit_premise(case);
        let dot: i128 = (0..3).map(|i| i128::from(case.u[i]) * i128::from(case.normal[i])).sum();
        assert_eq!(-dot, case.exact, "{}: the exact cosine", case.name);
        assert_eq!(nearest_even(case.exact), i128::from(case.cosine), "{}", case.name);
        let oracle = |c: i64| F0.map(|f0| schlick_oracle(f0, c));
        assert_eq!(oracle(case.cosine), case.throughput, "{}: the oracle", case.name);
        for (rule, cosine) in case.wrong {
            assert_ne!(oracle(*cosine), case.throughput, "{}: {rule} is observable", case.name);
        }
    }
    // The remainders below the cosine's raw unit: one under the tie, three ties, one over.
    let remainders = CASES.map(|case| case.exact.rem_euclid(UNIT) - HALF);
    assert_eq!(remainders, [-1, 0, 1, 0, 0]);
    // The tie cases' floors: even, odd, even.
    let floors = [1, 3, 4].map(|k| CASES[k].exact.div_euclid(UNIT) % 2);
    assert_eq!(floors, [0, 1, 0]);
    // The cosines are below 0.38, where one raw unit of cosine moves Schlick's term.
    assert!(CASES.iter().all(|case| i128::from(case.cosine) < UNIT * 38 / 100));
}
