//! Unsigned 256-bit integers for the exact intermediates of `bunny-quaternion/1`.
//!
//! Only the operations the quaternion kernels need, each exact under the bound its caller
//! states: the full product of two `u128`, a sum, a difference, a product by a `u128`, a left
//! shift and a division by a divisor below `2^96`.

/// An unsigned 256-bit integer `hi * 2^128 + lo`.
#[derive(Clone, Copy, Debug)]
pub(super) struct U256 {
    hi: u128,
    lo: u128,
}

const LOW_64: u128 = u64::MAX as u128;

impl U256 {
    /// Zero.
    pub(super) const ZERO: Self = Self { hi: 0, lo: 0 };

    /// The full product `a * b`, exact for every pair.
    pub(super) const fn product(a: u128, b: u128) -> Self {
        let (a1, a0) = (a >> 64, a & LOW_64);
        let (b1, b0) = (b >> 64, b & LOW_64);
        // Each partial product is below 2^128. `middle` may carry into bit 128.
        let (middle, middle_carry) = (a1 * b0).overflowing_add(a0 * b1);
        let (lo, low_carry) = (a0 * b0).overflowing_add(middle << 64);
        let carries = if middle_carry { 1 << 64 } else { 0 } + if low_carry { 1 } else { 0 };
        Self { hi: a1 * b1 + (middle >> 64) + carries, lo }
    }

    /// Whether the value is zero.
    pub(super) const fn is_zero(self) -> bool {
        self.hi == 0 && self.lo == 0
    }

    /// Whether `self <= rhs`.
    pub(super) const fn le(self, rhs: Self) -> bool {
        self.hi < rhs.hi || (self.hi == rhs.hi && self.lo <= rhs.lo)
    }

    /// `self + rhs`; the caller proves the sum is below `2^256`.
    pub(super) const fn add(self, rhs: Self) -> Self {
        let (lo, carry) = self.lo.overflowing_add(rhs.lo);
        Self { hi: self.hi.wrapping_add(rhs.hi).wrapping_add(if carry { 1 } else { 0 }), lo }
    }

    /// `self - rhs`; the caller proves `rhs <= self`.
    pub(super) const fn sub(self, rhs: Self) -> Self {
        let (lo, borrow) = self.lo.overflowing_sub(rhs.lo);
        Self { hi: self.hi.wrapping_sub(rhs.hi).wrapping_sub(if borrow { 1 } else { 0 }), lo }
    }

    /// `self * b`; the caller proves the product is below `2^256`.
    pub(super) const fn times(self, b: u128) -> Self {
        let low = Self::product(self.lo, b);
        Self { hi: low.hi.wrapping_add(self.hi.wrapping_mul(b)), lo: low.lo }
    }

    /// `self * 2^shift` for `shift` in `1..128`; the caller proves it is below `2^256`.
    pub(super) const fn shl(self, shift: u32) -> Self {
        Self { hi: (self.hi << shift) | (self.lo >> (128 - shift)), lo: self.lo << shift }
    }

    /// `(self / divisor, self % divisor)` for `0 < divisor < 2^96`; the caller proves the
    /// quotient is below `2^128`. Long division by 32-bit digits, each step exact in `u128`
    /// because the running remainder is below the divisor.
    pub(super) const fn div_rem(self, divisor: u128) -> (u128, u128) {
        let (mut quotient, mut remainder) = (0_u128, 0_u128);
        let mut digit = 8;
        while digit > 0 {
            digit -= 1;
            let word = if digit >= 4 { self.hi } else { self.lo };
            let current = (remainder << 32) | ((word >> (32 * (digit % 4))) & 0xFFFF_FFFF);
            quotient = (quotient << 32) | (current / divisor);
            remainder = current % divisor;
        }
        (quotient, remainder)
    }
}

#[cfg(test)]
mod tests {
    use super::U256;
    use wasm_bindgen_test::wasm_bindgen_test;

    const MAX: u128 = u128::MAX;

    /// Oracle: hand-computed values. `(2^128 - 1)^2 = 2^256 - 2^129 + 1` carries out of both
    /// the middle and the low word; `2^64 2^64 = 2^128`.
    #[wasm_bindgen_test(unsupported = test)]
    fn quat_tp_016_wide_products_sums_and_quotients_are_exact() {
        let square = U256::product(MAX, MAX);
        let words = |v: U256| (v.hi, v.lo);
        // dojo: allow panic-path -- test assertion against hand-computed values
        assert_eq!(words(square), (MAX - 1, 1));
        // dojo: allow panic-path -- test assertion against hand-computed values
        assert_eq!(words(U256::product(1 << 64, 1 << 64)), (1, 0));
        // dojo: allow panic-path -- test assertion: a times by a u128 is the full product
        assert_eq!(words(U256::product(MAX, 1).times(MAX)), (MAX - 1, 1));
        let sum = U256::product(MAX, 1).add(U256::product(1, 1));
        // dojo: allow panic-path -- test assertion: the carry into the high word
        assert_eq!(words(sum), (1, 0));
        // dojo: allow panic-path -- test assertion: the borrow from the high word
        assert_eq!(words(sum.sub(U256::product(1, 1))), (0, MAX));
        // dojo: allow panic-path -- test assertion: 2^127 shifted by 66 is 2^193
        assert_eq!(words(U256::product(1 << 127, 1).shl(66)), (1 << 65, 0));
        // dojo: allow panic-path -- test assertion against the comparison's definition
        assert!(sum.le(sum) && !sum.le(U256::product(MAX, 1)) && U256::ZERO.is_zero());
        let divisor = (1 << 90) + 5;
        let dividend = U256::product((1 << 100) + 7, divisor).add(U256::product(11, 1));
        // dojo: allow panic-path -- test assertion: (q d + r) / d = (q, r)
        assert_eq!(dividend.div_rem(divisor), ((1 << 100) + 7, 11));
    }
}
