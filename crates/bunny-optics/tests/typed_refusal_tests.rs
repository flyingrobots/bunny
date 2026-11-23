//! Typed refusals of `PinholeCamera::try_ray` and `Albedo::try_reflect`
//! (flyingrobots/bunny#445).
//!
//! The oracle is each refusal rule written out here from the raw inputs, in the order the
//! functions' documentation gives; it shares no helper with the kernels. Every boundary is
//! tried on, and one raw unit either side of, its threshold. The seeded cases check that each
//! `Option` form returns `Some(x)` exactly when its `try_` form returns `Ok(x)`, with the error
//! the oracle names otherwise; accepted reflections are also checked against an exact `i128`
//! product rounded to nearest, ties to even.

use core::error::Error;

use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use bunny_optics::{
    Albedo, AlbedoReflectError, CameraRayError, CameraSpec, PinholeCamera, PixelSample,
};
use bunny_query::TraceRayError;
use wasm_bindgen_test::wasm_bindgen_test;

/// The seed of the generated cases; printed in every failure message.
const REFUSAL_SEED: u64 = 0xB002_0445_0000_0C0A;
const CAMERAS: usize = 200;
const SAMPLES_PER_CAMERA: usize = 500;
const REFLECT_CASES: usize = 200_000;
/// One in Q32.32, written as a literal: `2^32` raw.
const ONE: i64 = 1 << 32;

fn fixed(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

fn vec3([x, y, z]: [i64; 3]) -> FixedVec3 {
    FixedVec3::new(fixed(x), fixed(y), fixed(z))
}

fn camera(width: u32, height: u32) -> PinholeCamera {
    let spec = CameraSpec {
        position: vec3([ONE, -2 * ONE, 3 * ONE]),
        look: vec3([0, 0, -ONE]),
        up: vec3([0, ONE, 0]),
        tan_half_fov: fixed(ONE),
        width,
        height,
    };
    PinholeCamera::new(&spec).expect("a valid camera")
}

fn sample(x: u32, y: u32, sub_x: i64, sub_y: i64) -> PixelSample {
    PixelSample::new(x, y, fixed(sub_x), fixed(sub_y))
}

/// The camera refusal rule from the raw sample, first failing condition first.
fn ray_rule(width: u32, height: u32, s: PixelSample) -> Option<CameraRayError> {
    let unit = |raw: i64| (0..ONE).contains(&raw);
    if s.x >= width {
        Some(CameraRayError::ColumnOutsideImage { column: s.x, width })
    } else if !unit(s.sub_x.raw()) {
        Some(CameraRayError::SubXOutsideUnitInterval { sub_x: s.sub_x })
    } else if s.y >= height {
        Some(CameraRayError::RowOutsideImage { row: s.y, height })
    } else if !unit(s.sub_y.raw()) {
        Some(CameraRayError::SubYOutsideUnitInterval { sub_y: s.sub_y })
    } else {
        None
    }
}

/// Checks one sample against the oracle and the `Option` form; returns the outcome's slot.
fn check_ray(camera: &PinholeCamera, s: PixelSample, context: &str) -> usize {
    let result = camera.try_ray(s);
    let expected = ray_rule(camera.width(), camera.height(), s);
    assert_eq!(result.err(), expected, "{context}: sample {s:?}");
    assert_eq!(camera.ray(s), result.ok(), "{context}: Option form differs for {s:?}");
    match expected {
        None => 0,
        Some(CameraRayError::ColumnOutsideImage { .. }) => 1,
        Some(CameraRayError::SubXOutsideUnitInterval { .. }) => 2,
        Some(CameraRayError::RowOutsideImage { .. }) => 3,
        Some(_) => 4,
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_330_a_column_outside_the_image_is_named() {
    let camera = camera(64, 48);
    assert_eq!(check_ray(&camera, sample(63, 0, 0, 0), "last column"), 0);
    assert_eq!(check_ray(&camera, sample(64, 0, 0, 0), "one past"), 1);
    assert_eq!(check_ray(&camera, sample(65, 0, 0, 0), "two past"), 1);
    assert_eq!(check_ray(&camera, sample(u32::MAX, 0, 0, 0), "u32::MAX"), 1);
    let named = camera.try_ray(sample(64, 3, ONE / 2, ONE / 2));
    assert_eq!(named, Err(CameraRayError::ColumnOutsideImage { column: 64, width: 64 }));
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_331_a_row_outside_the_image_is_named() {
    let camera = camera(64, 48);
    assert_eq!(check_ray(&camera, sample(0, 47, 0, 0), "last row"), 0);
    assert_eq!(check_ray(&camera, sample(0, 48, 0, 0), "one past"), 3);
    assert_eq!(check_ray(&camera, sample(0, 49, 0, 0), "two past"), 3);
    assert_eq!(check_ray(&camera, sample(0, u32::MAX, 0, 0), "u32::MAX"), 3);
    let named = camera.try_ray(sample(3, 48, ONE / 2, ONE / 2));
    assert_eq!(named, Err(CameraRayError::RowOutsideImage { row: 48, height: 48 }));
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_332_a_sub_pixel_offset_outside_the_unit_interval_is_named() {
    let camera = camera(64, 48);
    for (sub, slot) in [(-1, 2), (0, 0), (1, 0), (ONE - 1, 0), (ONE, 2), (ONE + 1, 2)] {
        assert_eq!(check_ray(&camera, sample(5, 5, sub, 0), "sub_x"), slot, "sub_x {sub}");
        let row_slot = if slot == 0 { 0 } else { 4 };
        assert_eq!(check_ray(&camera, sample(5, 5, 0, sub), "sub_y"), row_slot, "sub_y {sub}");
    }
    for sub in [i64::MIN, i64::MAX] {
        assert_eq!(check_ray(&camera, sample(5, 5, sub, 0), "extreme sub_x"), 2);
        assert_eq!(check_ray(&camera, sample(5, 5, 0, sub), "extreme sub_y"), 4);
    }
    let sub_x = camera.try_ray(sample(5, 5, ONE, 0));
    assert_eq!(sub_x, Err(CameraRayError::SubXOutsideUnitInterval { sub_x: fixed(ONE) }));
    let sub_y = camera.try_ray(sample(5, 5, 0, -1));
    assert_eq!(sub_y, Err(CameraRayError::SubYOutsideUnitInterval { sub_y: fixed(-1) }));
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_333_the_first_failing_sample_condition_is_reported() {
    let camera = camera(64, 48);
    // Column, then sub_x, then row, then sub_y: the order the image-plane offset reads them.
    assert_eq!(check_ray(&camera, sample(64, 48, ONE, ONE), "all bad"), 1);
    assert_eq!(check_ray(&camera, sample(0, 48, ONE, ONE), "sub_x before row"), 2);
    assert_eq!(check_ray(&camera, sample(0, 48, 0, ONE), "row before sub_y"), 3);
    assert_eq!(check_ray(&camera, sample(0, 0, 0, ONE), "sub_y alone"), 4);
}

/// Exact `round_half_even(a * s / 2^32)` in `i128`, independent of `FixedQ32_32`'s multiply.
fn product_oracle(a: i64, s: i128) -> i64 {
    let p = i128::from(a) * s;
    let (q, r) = (p >> 32, p & ((1 << 32) - 1));
    let half = 1_i128 << 31;
    let q = if r > half || (r == half && q & 1 == 1) { q + 1 } else { q };
    i64::try_from(q).expect("albedo is at most 1.0, so the product fits")
}

/// The reflect rule from the raw channels: the first channel whose exact sum leaves `i64`.
fn reflect_rule(channels: [i64; 3], direct: [i64; 3], indirect: [i64; 3]) -> Result<[i64; 3], usize> {
    let sums = [0, 1, 2].map(|c| i128::from(direct[c]) + i128::from(indirect[c]));
    let fits = |s: i128| i64::try_from(s).is_ok();
    match sums.iter().position(|&s| !fits(s)) {
        Some(channel) => Err(channel),
        None => Ok([0, 1, 2].map(|c| product_oracle(channels[c], sums[c]))),
    }
}

/// Checks one reflection against the oracle and the `Option` form; returns the outcome's slot.
fn check_reflect(channels: [i64; 3], direct: [i64; 3], indirect: [i64; 3], context: &str) -> usize {
    let albedo = Albedo::new(channels.map(fixed)).expect("a valid albedo");
    let (d, i) = (direct.map(fixed), indirect.map(fixed));
    let result = albedo.try_reflect(d, i);
    let rule = reflect_rule(channels, direct, indirect);
    let expected = rule.map(|raw| raw.map(fixed)).map_err(|channel| {
        AlbedoReflectError::SumUnrepresentable { channel, direct: d[channel], indirect: i[channel] }
    });
    assert_eq!(result, expected, "{context}: {channels:?} {direct:?} {indirect:?}");
    assert_eq!(albedo.reflect(d, i), result.ok(), "{context}: Option form differs");
    rule.map_or_else(|channel| channel + 1, |_| 0)
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_334_a_channel_sum_outside_q32_32_is_named_by_channel() {
    let channels = [ONE, ONE / 2, 1];
    let zero = [0; 3];
    for channel in 0..3 {
        let mut direct = zero;
        let mut indirect = zero;
        direct[channel] = i64::MAX;
        assert_eq!(check_reflect(channels, direct, indirect, "MAX + 0"), 0);
        indirect[channel] = 1;
        assert_eq!(check_reflect(channels, direct, indirect, "MAX + 1"), channel + 1);
        indirect[channel] = 2;
        assert_eq!(check_reflect(channels, direct, indirect, "MAX + 2"), channel + 1);
        direct[channel] = i64::MIN;
        indirect[channel] = 0;
        assert_eq!(check_reflect(channels, direct, indirect, "MIN + 0"), 0);
        indirect[channel] = -1;
        assert_eq!(check_reflect(channels, direct, indirect, "MIN - 1"), channel + 1);
        indirect[channel] = -2;
        assert_eq!(check_reflect(channels, direct, indirect, "MIN - 2"), channel + 1);
    }
    // The first channel whose sum overflows is named.
    let all = [i64::MAX; 3];
    assert_eq!(check_reflect(channels, all, [1; 3], "all overflow"), 1);
    let albedo = Albedo::new([fixed(ONE); 3]).expect("valid");
    let named = albedo.try_reflect([fixed(0), fixed(i64::MIN), fixed(i64::MIN)], [fixed(-1); 3]);
    let error = AlbedoReflectError::SumUnrepresentable {
        channel: 1,
        direct: fixed(i64::MIN),
        indirect: fixed(-1),
    };
    assert_eq!(named, Err(error));
}

struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    fn below(&mut self, bound: u64) -> u64 {
        ((self.next() << 21) ^ self.next()) % bound
    }

    fn index(&mut self, size: u32) -> u32 {
        let near = [0, size.saturating_sub(1), size, size.saturating_add(1), u32::MAX];
        if self.next() % 4 == 0 {
            near[usize::try_from(self.next() % 5).expect("small")]
        } else {
            u32::try_from(self.below(u64::from(size))).expect("below a u32")
        }
    }

    fn sub(&mut self) -> i64 {
        let near = [-1, 0, 1, ONE - 1, ONE, ONE + 1, i64::MIN, i64::MAX];
        if self.next() % 6 == 0 {
            near[usize::try_from(self.next() % 8).expect("small")]
        } else {
            i64::try_from(self.below(1 << 32)).expect("below 2^32")
        }
    }

    /// A raw value of any magnitude, often near the ends of `i64`.
    fn radiance(&mut self) -> i64 {
        let raw = ((self.next() << 43) ^ (self.next() << 21) ^ self.next()).cast_signed();
        match self.next() % 4 {
            0 => raw,
            1 => i64::MAX - (raw & 0xFF),
            2 => i64::MIN + (raw & 0xFF),
            _ => raw >> 16,
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_335_camera_option_and_result_forms_agree_on_generated_samples() {
    let mut seeds = Seeds(REFUSAL_SEED);
    let mut counts = [0_usize; 5];
    for case in 0..CAMERAS {
        let width = u32::try_from(1 + seeds.below(4096)).expect("small");
        let height = u32::try_from(1 + seeds.below(4096)).expect("small");
        let camera = camera(width, height);
        for draw in 0..SAMPLES_PER_CAMERA {
            let (x, y) = (seeds.index(width), seeds.index(height));
            let s = sample(x, y, seeds.sub(), seeds.sub());
            let context = format!("seed {REFUSAL_SEED:#x}, camera {case} ({width}x{height}), draw {draw}");
            counts[check_ray(&camera, s, &context)] += 1;
        }
    }
    let total = CAMERAS * SAMPLES_PER_CAMERA;
    println!("seed {REFUSAL_SEED:#x}: {total} samples, [ray, column, sub_x, row, sub_y] {counts:?}");
    assert!(counts.iter().all(|&n| n >= 1_000), "every outcome is reached often: {counts:?}");
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_336_reflect_option_and_result_forms_agree_on_generated_radiance() {
    let mut seeds = Seeds(REFUSAL_SEED ^ 0xA1B);
    let mut counts = [0_usize; 4];
    for case in 0..REFLECT_CASES {
        let channels = [0; 3].map(|_| i64::try_from(seeds.below(1 + (1 << 32))).expect("small"));
        let direct = [0; 3].map(|_| seeds.radiance());
        let indirect = [0; 3].map(|_| seeds.radiance());
        let context = format!("seed {:#x}, case {case}", REFUSAL_SEED ^ 0xA1B);
        counts[check_reflect(channels, direct, indirect, &context)] += 1;
    }
    println!(
        "seed {:#x}: {REFLECT_CASES} reflections, [reflected, r, g, b] {counts:?}",
        REFUSAL_SEED ^ 0xA1B
    );
    assert!(counts.iter().all(|&n| n >= 1_000), "every outcome is reached often: {counts:?}");
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_337_optics_refusals_display_their_values_and_are_errors() {
    let column = CameraRayError::ColumnOutsideImage { column: 64, width: 64 };
    assert_eq!(column.to_string(), "sample column 64 is outside an image 64 pixels wide");
    let row = CameraRayError::RowOutsideImage { row: 50, height: 48 };
    assert_eq!(row.to_string(), "sample row 50 is outside an image 48 pixels high");
    let sub_x = CameraRayError::SubXOutsideUnitInterval { sub_x: fixed(ONE) };
    assert_eq!(sub_x.to_string(), "sub-pixel x raw 4294967296 is outside [0, 1)");
    let sub_y = CameraRayError::SubYOutsideUnitInterval { sub_y: fixed(-1) };
    assert_eq!(sub_y.to_string(), "sub-pixel y raw -1 is outside [0, 1)");
    let wide = CameraRayError::DirectionUnrepresentable;
    assert_eq!(wide.to_string(), "ray direction is not representable in Q32.32");
    let refused = CameraRayError::RayRefused { reason: TraceRayError::ZeroDirection };
    assert_eq!(refused.to_string(), "camera ray is not a trace ray: ray direction is zero");
    let sum = AlbedoReflectError::SumUnrepresentable {
        channel: 2,
        direct: fixed(i64::MAX),
        indirect: fixed(1),
    };
    assert_eq!(
        sum.to_string(),
        "radiance sum of channel 2, raw 9223372036854775807 + 1, is not representable in Q32.32"
    );
    let plain: [&dyn Error; 6] = [&column, &row, &sub_x, &sub_y, &wide, &sum];
    assert!(plain.iter().all(|e| e.source().is_none()), "no underlying cause");
    let source = (&refused as &dyn Error).source().map(ToString::to_string);
    assert_eq!(source.as_deref(), Some("ray direction is zero"), "the trace refusal is the source");
}
