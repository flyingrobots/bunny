//! Unsigned 256-bit integers for the exact sphere discriminant.
//!
//! The quarter discriminant of a trace-domain sphere test is below `2^147` and its square
//! root is taken after a further `2^64` scale (`docs/topics/ray-tracing/README.md#ray-sphere`),
//! so it needs more than `u128`. Only the operations that test needs are here.

use bunny_num::FixedQ32_32;

/// The low 64 bits of a `u128`.
const LOW_HALF: u128 = (1 << 64) - 1;

/// An unsigned 256-bit integer, `hi * 2^128 + lo`.
///
/// `hi` is declared first, so the derived ordering is the numeric one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct U256 {
    hi: u128,
    lo: u128,
}

impl U256 {
    /// The exact product of two `u128` values.
    pub(super) fn product(lhs: u128, rhs: u128) -> Self {
        let (a1, a0) = (lhs >> 64, lhs & LOW_HALF);
        let (b1, b0) = (rhs >> 64, rhs & LOW_HALF);
        // Each partial product of two 64-bit halves fits `u128`; only their sums carry.
        let (middle, middle_carry) = (a0 * b1).overflowing_add(a1 * b0);
        let (lo, low_carry) = (a0 * b0).overflowing_add(middle << 64);
        // The true product is below 2^256, so the high limb cannot overflow.
        let hi = a1 * b1 + (middle >> 64) + (u128::from(middle_carry) << 64);
        Self { hi: hi + u128::from(low_carry), lo }
    }

    /// `self + rhs`, or `None` when the sum is not below `2^256`.
    pub(super) fn checked_add(self, rhs: Self) -> Option<Self> {
        let (lo, carry) = self.lo.overflowing_add(rhs.lo);
        let hi = self.hi.checked_add(rhs.hi)?.checked_add(u128::from(carry))?;
        Some(Self { hi, lo })
    }

    /// `self - rhs`, or `None` when `rhs` is larger.
    pub(super) fn checked_sub(self, rhs: Self) -> Option<Self> {
        let (lo, borrow) = self.lo.overflowing_sub(rhs.lo);
        let hi = self.hi.checked_sub(rhs.hi)?.checked_sub(u128::from(borrow))?;
        Some(Self { hi, lo })
    }

    /// `self * 2^64`, or `None` when the result is not below `2^256`.
    pub(super) const fn checked_shl64(self) -> Option<Self> {
        if self.hi >> 64 != 0 {
            return None;
        }
        Some(Self { hi: (self.hi << 64) | (self.lo >> 64), lo: self.lo << 64 })
    }

    /// The floor of the square root, exactly.
    ///
    /// Below `2^128` this is `FixedQ32_32::sqrt_u128`. Above, with `s` the floor root of the
    /// high limb (`1 <= s < 2^64`), the root lies in `s * 2^64 .. (s + 1) * 2^64`, so it
    /// fits `u128`; its low 64 bits are set one at a time, from the top, whenever the
    /// square stays at most `self`.
    pub(super) fn isqrt(self) -> u128 {
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

    /// Oracle: hand-computed values of `(2^128 - 1)^2 = 2^256 - 2^129 + 1`.
    #[test]
    fn product_of_largest_values_is_exact() {
        let max = u128::MAX;
        assert_eq!(U256::product(max, max), U256 { hi: max - 1, lo: 1 });
        assert_eq!(U256::product(1 << 64, 1 << 64), U256 { hi: 1, lo: 0 });
    }

    /// Oracle: `isqrt` of `k^2 - 1`, `k^2` and `k^2 + 1` is `k - 1`, `k` and `k`.
    #[test]
    fn isqrt_is_the_floor_root_around_squares() {
        for k in [3_u128, 1 << 64, (1 << 64) + 7, 1 << 100, u128::MAX] {
            let square = U256::product(k, k);
            let one = U256 { hi: 0, lo: 1 };
            assert_eq!(square.isqrt(), k, "k = {k}");
            assert_eq!(square.checked_sub(one).map(U256::isqrt), Some(k - 1), "k = {k}");
            if let Some(above) = square.checked_add(one) {
                assert_eq!(above.isqrt(), k, "k = {k}");
            }
        }
    }

    /// Oracle: carries and borrows across the limb boundary, and the overflow limits.
    #[test]
    fn add_sub_and_shift_carry_and_refuse_overflow() {
        let low_max = U256 { hi: 0, lo: u128::MAX };
        let one = U256 { hi: 0, lo: 1 };
        assert_eq!(low_max.checked_add(one), Some(U256 { hi: 1, lo: 0 }));
        assert_eq!(U256 { hi: 1, lo: 0 }.checked_sub(one), Some(low_max));
        assert_eq!(one.checked_sub(low_max), None);
        assert_eq!(U256 { hi: u128::MAX, lo: u128::MAX }.checked_add(one), None);
        assert_eq!(
            low_max.checked_shl64(),
            Some(U256 { hi: u128::MAX >> 64, lo: u128::MAX << 64 })
        );
        assert_eq!(U256 { hi: 1 << 64, lo: 0 }.checked_shl64(), None);
    }
}
