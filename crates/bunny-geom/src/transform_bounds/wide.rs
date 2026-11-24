//! How far, in whole raw units, an ellipsoid reaches past its centre on one axis, decided
//! exactly in 256-bit integers.
//!
//! In raw units the reach is `R sqrt(S) / 2^32`, with `R` the radius and `S` the squared
//! length of one row of the linear part. Widths: `R^2 < 2^126`, `S <= 3 * 2^126 < 2^128`, so
//! `X = R^2 S` is below `2^254`. Each candidate `k 2^32 +- b` is below `2^128` (`k < 2^96`),
//! so its square is below `2^256`. Both are `(high, low)` pairs of `u128`, compared as tuples.

const LOW_64: u128 = u128::MAX >> 64;

/// The squared reach `X = R^2 S` and a `k` known to clear it.
#[derive(Clone, Copy)]
pub(super) struct Reach {
    squared: (u128, u128),
    ceiling: u128,
}

impl Reach {
    /// The reach of radius `radius` raw along a row of raw entries `row`.
    pub(super) fn new(radius: u64, row: [i64; 3]) -> Self {
        let length_squared: u128 = row.iter().map(|m| square(m.unsigned_abs())).sum();
        let radius = u128::from(radius);
        // `isqrt(S) + 1 > sqrt(S)` and is at most `2^64`, so `R (isqrt(S) + 1) < 2^127` and
        // `ceiling 2^32 - 2^32 > R sqrt(S)`: the ceiling clears the reach on either side.
        let above_root = bunny_num::FixedQ32_32::sqrt_u128(length_squared) + 1;
        Self {
            squared: wide_mul(radius * radius, length_squared),
            ceiling: ((radius * above_root) >> 32) + 2,
        }
    }

    /// The least `k >= 0` with `(k 2^32 + b)^2 >= X`: the floor of the exact low extreme is
    /// the centre's floor minus `k`, when `b` is the centre's fraction.
    pub(super) fn below(&self, b: i128) -> i128 {
        let b = b.unsigned_abs();
        self.least(|k| (k << 32).checked_add(b).is_some_and(|d| wide_mul(d, d) >= self.squared))
    }

    /// The least `k >= 0` with `k 2^32 >= b` and `(k 2^32 - b)^2 >= X`: the ceiling of the
    /// exact high extreme is the centre's floor plus `k`.
    pub(super) fn above(&self, b: i128) -> i128 {
        let b = b.unsigned_abs();
        self.least(|k| (k << 32).checked_sub(b).is_some_and(|d| wide_mul(d, d) >= self.squared))
    }

    /// The least `k` in `[0, ceiling]` for which the monotone `clears` holds, by bisection;
    /// `clears(ceiling)` holds by construction. At most 97 steps.
    fn least(&self, clears: impl Fn(u128) -> bool) -> i128 {
        let (mut lo, mut hi) = (0_u128, self.ceiling);
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if clears(mid) {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        // `lo <= ceiling < 2^96`, so it fits.
        i128::try_from(lo).unwrap_or(i128::MAX)
    }
}

fn square(value: u64) -> u128 {
    u128::from(value) * u128::from(value)
}

/// The exact 256-bit product `a * b` as `(high, low)`, from four 64-bit by 64-bit products.
fn wide_mul(a: u128, b: u128) -> (u128, u128) {
    let (a_hi, a_lo) = (a >> 64, a & LOW_64);
    let (b_hi, b_lo) = (b >> 64, b & LOW_64);
    let (middle, middle_carry) = (a_hi * b_lo).overflowing_add(a_lo * b_hi);
    let (low, low_carry) = (a_lo * b_lo).overflowing_add(middle << 64);
    // Every product here is below 2^256, so these additions cannot overflow.
    let high = a_hi * b_hi + (middle >> 64) + (u128::from(middle_carry) << 64);
    (high + u128::from(low_carry), low)
}
