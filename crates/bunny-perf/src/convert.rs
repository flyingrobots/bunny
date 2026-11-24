//! Q32.32 conversions at the `f32` boundary: `FixedQ32_32::try_from_f32` and
//! `FixedQ32_32::to_f32`, one call per value over 1 024 values built before timing.
//!
//! The inputs are seeded Q32.32 values in `[-1000, 1000)` units from the fixed seed
//! [`CONVERT_SEED`]; `num/try_from_f32` converts their `f32` images (from `to_f32`, so every
//! input is finite and in range) and `num/to_f32` converts the Q32.32 values. Each result
//! passes through `black_box`, and its raw value or its `f32` bits are summed into the checksum.

use std::hint::black_box;

use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;

use crate::cases::{Case, BATCH};

/// The generator's fixed seed.
const CONVERT_SEED: u64 = 0xB002_0130_C0F3_0001;

/// The seeded Q32.32 values, from [`CONVERT_SEED`].
pub(crate) fn values() -> Vec<FixedQ32_32> {
    let mut state = CONVERT_SEED;
    let span = (2000 * ONE_RAW).unsigned_abs();
    (0..BATCH)
        .map(|_| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            FixedQ32_32::from_raw(i64::try_from(state % span).unwrap_or(0) - 1000 * ONE_RAW)
        })
        .collect()
}

/// Every case of the `convert` group.
pub(crate) fn convert_cases() -> Vec<Case> {
    let fixed = values();
    let floats: Vec<f32> = fixed.iter().map(|v| v.to_f32()).collect();
    let from_f32 = move || {
        floats
            .iter()
            .map(|&f| {
                black_box(FixedQ32_32::try_from_f32(black_box(f))).map_or(0, FixedQ32_32::raw)
            })
            .fold(0, i64::wrapping_add)
    };
    let to_f32 = move || {
        fixed
            .iter()
            .map(|&v| i64::from(black_box(black_box(v).to_f32()).to_bits()))
            .fold(0, i64::wrapping_add)
    };
    vec![
        Case { name: "num/try_from_f32", calls: BATCH, run: Box::new(from_f32) },
        Case { name: "num/to_f32", calls: BATCH, run: Box::new(to_f32) },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The values are the same on every construction, and these (a sum recorded from this
    /// generator); every `f32` image converts back.
    #[test]
    fn the_values_are_the_same_on_every_construction() {
        let (first, second) = (values(), values());
        assert_eq!(first, second);
        assert_eq!(first.len(), BATCH);
        let sum = first.iter().map(|v| v.raw()).fold(0_i64, i64::wrapping_add);
        assert_eq!(sum, VALUES_SUM);
        assert!(first.iter().all(|v| FixedQ32_32::try_from_f32(v.to_f32()).is_ok()));
    }

    /// Each case runs once (no timing) and returns the same checksum on every run and every
    /// construction.
    #[test]
    fn every_convert_case_runs_and_its_checksum_is_stable() {
        let (first, second) = (convert_cases(), convert_cases());
        let names: Vec<&str> = first.iter().map(|case| case.name).collect();
        assert_eq!(names, ["num/try_from_f32", "num/to_f32"]);
        for (a, b) in first.iter().zip(&second) {
            let value = (a.run)();
            assert_eq!(value, (a.run)(), "{}", a.name);
            assert_eq!(value, (b.run)(), "{}", a.name);
            assert_ne!(value, 0, "{} does work", a.name);
        }
    }

    const VALUES_SUM: i64 = 0;
}
