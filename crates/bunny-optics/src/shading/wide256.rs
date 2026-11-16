//! Unsigned 256-bit integers for the exact dielectric boundary test.
//!
//! The comparison that decides total internal reflection multiplies three values near `2^66`
//! (`docs/topics/optics/README.md#smooth-dielectric`), so it needs more than `u128`. Only the
//! operations that kernel needs are here; `bunny-query` keeps a private type of the same shape
//! for the sphere discriminant.

use bunny_num::FixedQ32_32;

/// The low 64 bits of a `u128`.
const LOW_HALF: u128 = (1 << 64) - 1;

/// An unsigned 256-bit integer, `hi * 2^128 + lo`.
///
/// `hi` is declared first, so the derived ordering is the numeric one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct U256 {
    hi: u128,
    lo: u128,
}

impl U256 {
    /// Zero.
    pub const ZERO: Self = Self { hi: 0, lo: 0 };

    /// A value below `2^128`.
    pub const fn from_u128(lo: u128) -> Self {
        Self { hi: 0, lo }
    }

    /// The exact product of two `u128` values.
    pub fn product(lhs: u128, rhs: u128) -> Self {
        let (a1, a0) = (lhs >> 64, lhs & LOW_HALF);
        let (b1, b0) = (rhs >> 64, rhs & LOW_HALF);
        // Each partial product of two 64-bit halves fits `u128`; only their sums carry.
        let (middle, middle_carry) = (a0 * b1).overflowing_add(a1 * b0);
        let (lo, low_carry) = (a0 * b0).overflowing_add(middle << 64);
        // The true product is below 2^256, so the high limb cannot overflow.
        let hi = a1 * b1 + (middle >> 64) + (u128::from(middle_carry) << 64);
        Self { hi: hi + u128::from(low_carry), lo }
    }

    /// `self * rhs`, or `None` when the product is not below `2^256`.
    pub fn checked_times(self, rhs: u128) -> Option<Self> {
        let low = Self::product(self.lo, rhs);
        let hi = self.hi.checked_mul(rhs)?.checked_add(low.hi)?;
        Some(Self { hi, lo: low.lo })
    }

    /// `self + rhs`, or `None` when the sum is not below `2^256`.
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        let (lo, carry) = self.lo.overflowing_add(rhs.lo);
        let hi = self.hi.checked_add(rhs.hi)?.checked_add(u128::from(carry))?;
        Some(Self { hi, lo })
    }

    /// `self - rhs`, or `None` when `rhs` is larger.
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        let (lo, borrow) = self.lo.overflowing_sub(rhs.lo);
        let hi = self.hi.checked_sub(rhs.hi)?.checked_sub(u128::from(borrow))?;
        Some(Self { hi, lo })
    }

    /// `|self - rhs|`.
    pub fn abs_diff(self, rhs: Self) -> Self {
        let (large, small) = if self >= rhs { (self, rhs) } else { (rhs, self) };
        large.checked_sub(small).unwrap_or(Self::ZERO)
    }

    /// The number of bits needed to write the value: zero for zero, else `floor(log2) + 1`.
    pub const fn bit_length(self) -> u32 {
        if self.hi == 0 {
            u128::BITS - self.lo.leading_zeros()
        } else {
            2 * u128::BITS - self.hi.leading_zeros()
        }
    }

    /// The low 128 bits of `floor(self / 2^shift)`, for any `shift`.
    pub const fn shifted_right(self, shift: u32) -> u128 {
        match shift {
            0 => self.lo,
            1..=127 => (self.lo >> shift) | (self.hi << (u128::BITS - shift)),
            128..=255 => self.hi >> (shift - u128::BITS),
            _ => 0,
        }
    }

    /// The floor of the square root, exactly.
    ///
    /// Below `2^128` this is `FixedQ32_32::sqrt_u128`. Above, with `s` the floor root of the
    /// high limb (`1 <= s < 2^64`), the root lies in `s * 2^64 .. (s + 1) * 2^64`, so it
    /// fits `u128`; its low 64 bits are set one at a time, from the top, whenever the
    /// square stays at most `self`.
    pub fn isqrt(self) -> u128 {
        if self.hi == 0 {
            return FixedQ32_32::sqrt_u128(self.lo);
        }
        let mut root = FixedQ32_32::sqrt_u128(self.hi) << 64;
        let mut bit = 1_u128 << 63;
        while bit != 0 {
            let candidate = root | bit;
            if Self::product(candidate, candidate) <= self {
                root = candidate;
            }
            bit >>= 1;
        }
        root
    }
}

#[cfg(test)]
mod tests {
    use super::U256;
    use wasm_bindgen_test::wasm_bindgen_test;

    const ONE: U256 = U256 { hi: 0, lo: 1 };
    const LOW_MAX: U256 = U256 { hi: 0, lo: u128::MAX };
    const TOP: U256 = U256 { hi: u128::MAX, lo: u128::MAX };

    /// Oracle: hand-computed products, `(2^128 - 1)^2 = 2^256 - 2^129 + 1`,
    /// `2^64 * 2^64 = 2^128` and `(2^128 - 1) * 2^64 * 2^64 = 2^256 - 2^128`.
    #[wasm_bindgen_test(unsupported = test)]
    fn products_are_exact_and_refuse_overflow() {
        let max = u128::MAX;
        let got = (U256::product(max, max), U256::product(1 << 64, 1 << 64));
        // dojo: allow panic-path -- test assertion against hand-computed values
        assert_eq!(got, (U256 { hi: max - 1, lo: 1 }, U256 { hi: 1, lo: 0 }));
        let shifted = U256 { hi: max >> 64, lo: max << 64 };
        let got = [LOW_MAX.checked_times(1 << 64), shifted.checked_times(1 << 64)];
        // dojo: allow panic-path -- test assertion against hand-computed values
        assert_eq!(got, [Some(shifted), Some(U256 { hi: max, lo: 0 })]);
        // dojo: allow panic-path -- test assertion: 2^256 and 2^255 * 2 are not representable
        assert_eq!(
            [TOP.checked_times(2), U256 { hi: 1 << 127, lo: 0 }.checked_times(2)],
            [None; 2]
        );
    }

    /// Oracle: `isqrt` of `k^2 - 1`, `k^2` and `k^2 + 1` is `k - 1`, `k` and `k`.
    #[wasm_bindgen_test(unsupported = test)]
    fn isqrt_is_the_floor_root_around_squares() {
        for k in [3_u128, 1 << 64, (1 << 64) + 7, 1 << 100, u128::MAX] {
            let square = U256::product(k, k);
            let below = square.checked_sub(ONE).map(U256::isqrt);
            let above = square.checked_add(ONE).map(U256::isqrt);
            // dojo: allow panic-path -- test assertion against the defining property
            assert_eq!((below, square.isqrt(), above), (Some(k - 1), k, Some(k)), "k = {k}");
        }
    }

    /// Oracle: hand-computed carries and borrows across the limb boundary, and the overflow
    /// limits.
    #[wasm_bindgen_test(unsupported = test)]
    fn add_sub_and_difference_carry_and_refuse_overflow() {
        let high = U256 { hi: 1, lo: 0 };
        let got = [
            LOW_MAX.checked_add(ONE),
            high.checked_sub(ONE),
            ONE.checked_sub(LOW_MAX),
            TOP.checked_add(ONE),
            Some(ONE.abs_diff(high)),
            Some(high.abs_diff(ONE)),
        ];
        let expected = [Some(high), Some(LOW_MAX), None, None, Some(LOW_MAX), Some(LOW_MAX)];
        // dojo: allow panic-path -- test assertion against hand-computed values
        assert_eq!(got, expected);
    }

    /// Oracle: hand-computed bit lengths and shifts of `2^200 + 5` and of small values.
    #[wasm_bindgen_test(unsupported = test)]
    fn bit_length_and_right_shift_cross_the_limb_boundary() {
        let value = U256 { hi: 1 << 72, lo: 5 };
        let lengths = [U256::ZERO, ONE, LOW_MAX, value, TOP].map(U256::bit_length);
        // dojo: allow panic-path -- test assertion against hand-computed values
        assert_eq!(lengths, [0, 1, 128, 201, 256]);
        let shifts = [0, 1, 2, 128, 139, 200, 201, 256].map(|s| value.shifted_right(s));
        // dojo: allow panic-path -- test assertion against hand-computed values
        assert_eq!(shifts, [5, 2, 1, 1 << 72, 1 << 61, 1, 0, 0]);
        // dojo: allow panic-path -- test assertion: 3 * 2^128 / 2 = 2^128 + 2^127
        assert_eq!(U256 { hi: 3, lo: 0 }.shifted_right(1), 1 << 127);
    }
}
