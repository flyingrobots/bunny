#![deny(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(missing_docs)]

//! Deterministic numeric profiles for Bunny graphics primitives.

pub mod fixed_q32_32;

pub use fixed_q32_32::{FixedQ32_32, FloatConversionError};

/// Bunny's first scalar representation.
///
/// This is intentionally tiny while the deterministic profile is still being
/// designed. Public APIs should name `Scalar` instead of spelling a primitive
/// float directly.
// dojo: allow float-boundary -- scalar DTO boundary; canonical math uses FixedQ32_32
pub type Scalar = f32;

/// Returns true when the scalar can participate in canonical Bunny contracts.
#[must_use]
pub const fn is_finite(value: Scalar) -> bool {
    value.is_finite()
}

// Exact integer square root of a `u128`.
//
// Two implementations return the same floor of the square root. `isqrt_u128` chooses
// between them at compile time (see the performance chapter, "Target-specific
// implementations"). Both are ordinary functions that the unit tests call on every target.
//
// - Core's `u128::isqrt`, chosen on every target. Measured on aarch64 (Apple M5 Pro,
//   container): Bunny's version below makes `sqrt` and `length` about 1.3x faster but
//   `normalize` about 4 % slower, so aarch64 keeps core's. wasm32 and x86-64 are
//   unmeasured, so they keep it too.
// - `isqrt_u128_two_divides`: the top 64 bits go through `u64::isqrt`, whose steps divide
//   in hardware, and one Karatsuba square-root step (Zimmermann, INRIA RR-3805) finishes
//   the low half. Core's `u128` version ends that step with a `u128 / u128` division,
//   which aarch64 and wasm32 run in software; here the divisor is below `2^33`, so the
//   division splits into two `u64` divisions. No target selects it yet.

/// Floor of the square root of `n`, by the implementation chosen for this target.
#[inline]
pub(crate) const fn isqrt_u128(n: u128) -> u128 {
    // No `cfg(target_arch)` arm selects `isqrt_u128_two_divides` until a target measures
    // as a net win; add the arm together with that target's ledger numbers.
    n.isqrt()
}

// Exact integer square root of a `u128` without 128-bit division.
//
// The result equals `u128::isqrt`: the floor of the square root. The top 64 bits go
// through `u64::isqrt`, whose steps divide in hardware, and one Karatsuba square-root
// step (Zimmermann, INRIA RR-3805) finishes the low half. Core's `u128` version ends that
// step with a `u128 / u128` division, which aarch64 and wasm32 run in software; here the
// divisor is below `2^33`, so the division splits into two `u64` divisions.

/// Floor of the square root of `n` without 128-bit division.
#[allow(clippy::cast_possible_truncation)]
#[cfg_attr(not(test), allow(dead_code))]
#[inline]
pub(crate) const fn isqrt_u128_two_divides(n: u128) -> u128 {
    if n <= u64::MAX as u128 {
        return (n as u64).isqrt() as u128;
    }
    // An even shift keeps the root exact: sqrt(n << 2k) == sqrt(n) << k. After it, one of
    // the two top bits is set, which the Karatsuba step requires.
    let shift = n.leading_zeros() & !1;
    normalized_root(n << shift) >> (shift / 2)
}

/// Floor of the square root of `n`, where one of the two top bits of `n` is set.
#[allow(clippy::cast_possible_truncation)]
#[cfg_attr(not(test), allow(dead_code))]
#[inline]
const fn normalized_root(n: u128) -> u128 {
    let high = (n >> 64) as u64;
    // high >= 2^62, so 2^31 <= root < 2^32 and remainder <= 2 * root < 2^33.
    let root = high.isqrt();
    let remainder = high - root * root;
    let next_digits = (n as u64) >> 32;
    let quotient = divide_by_double_root(remainder, next_digits, root << 1);
    let candidate = ((root as u128) << 32) + quotient as u128;
    // The quotient can overshoot by one; correct exactly as core does.
    match candidate.checked_mul(candidate) {
        Some(square) if square <= n => candidate,
        _ => candidate - 1,
    }
}

/// `(remainder * 2^32 + digits) / divisor` for `remainder < 2^33`, `digits < 2^32` and
/// `2^32 <= divisor < 2^33`, by long division in 31-bit digits so every step fits a `u64`.
#[cfg_attr(not(test), allow(dead_code))]
#[inline]
const fn divide_by_double_root(remainder: u64, digits: u64, divisor: u64) -> u64 {
    const DIGIT_BITS: u32 = 31;
    const DIGIT_MASK: u64 = (1 << DIGIT_BITS) - 1;
    // Upper digits: remainder * 2 + the top bit of `digits`, below 2^34.
    let upper = (remainder << 1) | (digits >> DIGIT_BITS);
    let high_quotient = upper / divisor;
    // The partial remainder is below the divisor (< 2^33), so this is below 2^64.
    let lower = ((upper % divisor) << DIGIT_BITS) | (digits & DIGIT_MASK);
    (high_quotient << DIGIT_BITS) + lower / divisor
}

#[cfg(test)]
mod tests {
    use super::{isqrt_u128, isqrt_u128_two_divides};
    use wasm_bindgen_test::wasm_bindgen_test;

    fn next_seed(state: &mut u64) -> u64 {
        *state =
            state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        *state
    }

    /// Inputs spread across every magnitude: a random value shifted right by a random amount.
    fn spread_u128(state: &mut u64) -> u128 {
        let wide = (u128::from(next_seed(state)) << 64) | u128::from(next_seed(state));
        wide >> (next_seed(state) % 128)
    }

    #[wasm_bindgen_test(unsupported = test)]
    fn two_divides_matches_core_isqrt_on_many_seeded_inputs() {
        let mut state = 0xB002_5051_0000_0001_u64 ^ 0x0C0E_0000_0000_0001;
        for case in 0..200_000 {
            let value = spread_u128(&mut state);
            assert_eq!(isqrt_u128_two_divides(value), value.isqrt(), "case={case}; value={value}");
        }
    }

    #[wasm_bindgen_test(unsupported = test)]
    fn two_divides_matches_core_isqrt_around_the_split_and_correction_points() {
        let mut cases = vec![
            0_u128,
            u128::MAX,
            u128::from(u64::MAX),
            u128::from(u64::MAX) + 1,
            1_u128 << 126,
            (1_u128 << 126) - 1,
        ];
        // Squares of roots around 2^32 multiples exercise the Karatsuba quotient overshoot.
        for high in [1_u128 << 31, (1 << 31) + 1, (1 << 32) - 1, 3 << 30] {
            for low in [0_u128, 1, 0xFFFF_FFFF, 0x8000_0000] {
                let root = (high << 32) | low;
                let square = root * root;
                cases.extend([square - 1, square, square + 1, square + 2 * root]);
            }
        }
        for value in cases {
            assert_eq!(isqrt_u128_two_divides(value), value.isqrt(), "value={value}");
        }
    }

    #[wasm_bindgen_test(unsupported = test)]
    fn the_chosen_implementation_matches_the_other_one() {
        let mut state = 0x5E1E_C7ED_0000_0001_u64;
        for case in 0..20_000 {
            let value = spread_u128(&mut state);
            assert_eq!(isqrt_u128(value), isqrt_u128_two_divides(value), "case={case}");
        }
    }
}
