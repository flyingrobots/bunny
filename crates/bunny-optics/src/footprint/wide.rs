//! Signed 320-bit integers for the exact footprint quotient.
//!
//! The numerator of one texture-coordinate derivative is a sum of two products of a 64-bit
//! texture-coordinate difference and a 239-bit determinant, below `2^305` in magnitude
//! (`docs/topics/optics/README.md#ray-footprint`), so it needs more than the 256 bits of the
//! shading kernels' `U256`. Only the operations the footprint needs are here.

use core::cmp::Ordering;

/// The number of 64-bit limbs.
const LIMBS: usize = 5;

/// A signed integer in two's complement, little-endian 64-bit limbs: `-2^319..2^319`.
///
/// Arithmetic wraps modulo `2^320`; every caller keeps its true values far inside the range,
/// so no result wraps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct I320([u64; LIMBS]);

impl I320 {
    /// The value of an `i128`, sign-extended.
    pub const fn from_i128(value: i128) -> Self {
        let bits = value.cast_unsigned();
        let extension = if value < 0 { u64::MAX } else { 0 };
        Self([low(bits), low(bits >> 64), extension, extension, extension])
    }

    /// The exact product of two `i128` values (below `2^254` in magnitude).
    pub fn product(lhs: i128, rhs: i128) -> Self {
        Self::from_i128(lhs).times(Self::from_i128(rhs))
    }

    /// `self * rhs` modulo `2^320`, which is the signed product when it fits.
    pub fn times(self, rhs: Self) -> Self {
        let mut out = [0_u64; LIMBS];
        for (shift, &a) in self.0.iter().enumerate() {
            let mut carry = 0_u128;
            // Limbs at or past `LIMBS` are dropped: the product is taken modulo 2^320.
            for (slot, &b) in out.iter_mut().skip(shift).zip(rhs.0.iter()) {
                // At most (2^64 - 1) + (2^64 - 1)^2 + (2^64 - 1) = 2^128 - 1.
                let term = u128::from(*slot) + u128::from(a) * u128::from(b) + carry;
                *slot = low(term);
                carry = term >> 64;
            }
        }
        Self(out)
    }

    /// `self + rhs` modulo `2^320`.
    pub fn plus(self, rhs: Self) -> Self {
        let mut out = [0_u64; LIMBS];
        let mut carry = 0_u128;
        for ((slot, &a), &b) in out.iter_mut().zip(self.0.iter()).zip(rhs.0.iter()) {
            let term = u128::from(a) + u128::from(b) + carry;
            *slot = low(term);
            carry = term >> 64;
        }
        Self(out)
    }

    /// `-self` modulo `2^320`.
    pub fn negated(self) -> Self {
        Self(self.0.map(|limb| !limb)).plus(Self::from_i128(1))
    }

    /// `self - rhs` modulo `2^320`.
    pub fn minus(self, rhs: Self) -> Self {
        self.plus(rhs.negated())
    }

    /// Whether the value is below zero.
    pub fn is_negative(&self) -> bool {
        self.0.last().is_some_and(|top| top >> 63 == 1)
    }

    /// The order of the two values read as unsigned integers.
    fn cmp_unsigned(&self, other: &Self) -> Ordering {
        self.0.iter().rev().cmp(other.0.iter().rev())
    }

    /// `floor(self / 2)` of the value read as an unsigned integer.
    fn halved(self) -> Self {
        let mut out = self.0;
        let mut carried = 0_u64;
        for slot in out.iter_mut().rev() {
            let limb = *slot;
            *slot = (limb >> 1) | (carried << 63);
            carried = limb & 1;
        }
        Self(out)
    }
}

/// `numerator / denominator` rounded once to nearest, ties to even, or `None` when the rounded
/// value is outside `i64`.
///
/// Requires `denominator > 0` and `|numerator|`, `denominator * 2^64` below `2^319`. The
/// magnitude is divided exactly, so a negative quotient rounds as its magnitude does (ties to
/// even are symmetric).
pub fn rounded_quotient(numerator: I320, denominator: I320) -> Option<i64> {
    let negative = numerator.is_negative();
    let magnitude = if negative { numerator.negated() } else { numerator };
    let (quotient, remainder) = long_division(magnitude, denominator)?;
    let twice = remainder.plus(remainder);
    let up = match twice.cmp_unsigned(&denominator) {
        Ordering::Greater => true,
        Ordering::Equal => quotient & 1 == 1,
        Ordering::Less => false,
    };
    let rounded = i128::try_from(quotient + u128::from(up)).ok()?;
    i64::try_from(if negative { -rounded } else { rounded }).ok()
}

/// `floor(magnitude / denominator)` and the remainder, or `None` when the quotient is at least
/// `2^64` (then the rounded value is outside `i64`). Restoring binary division, 64 steps.
fn long_division(magnitude: I320, denominator: I320) -> Option<(u128, I320)> {
    let mut step = denominator.times(I320::from_i128(1 << 64));
    if magnitude.cmp_unsigned(&step) != Ordering::Less {
        return None;
    }
    let (mut quotient, mut remainder) = (0_u128, magnitude);
    for _ in 0..64 {
        // `remainder < 2 * step` before each halving, so one subtraction settles each bit.
        step = step.halved();
        quotient <<= 1;
        if remainder.cmp_unsigned(&step) != Ordering::Less {
            remainder = remainder.minus(step);
            quotient |= 1;
        }
    }
    Some((quotient, remainder))
}

/// The low 64 bits.
#[allow(clippy::cast_possible_truncation)] // Keeping only the low 64 bits is the point.
const fn low(value: u128) -> u64 {
    value as u64
}

#[cfg(test)]
mod tests {
    use super::{rounded_quotient, I320};
    use wasm_bindgen_test::wasm_bindgen_test;

    fn quotient(numerator: I320, denominator: i128) -> Option<i64> {
        rounded_quotient(numerator, I320::from_i128(denominator))
    }

    /// Oracle: hand-computed quotients of small values, ties both ways and both signs.
    #[wasm_bindgen_test(unsupported = test)]
    fn small_quotients_round_to_nearest_with_ties_to_even() {
        let cases = [(7, 2, 4), (5, 2, 2), (-5, 2, -2), (-7, 2, -4), (7, 3, 2), (-8, 3, -3)];
        for (n, d, expected) in cases {
            // dojo: allow panic-path -- test assertion against hand-computed values
            assert_eq!(quotient(I320::from_i128(n), d), Some(expected), "{n} / {d}");
        }
        // dojo: allow panic-path -- test assertion: 0 / d is 0
        assert_eq!(quotient(I320::from_i128(0), 9), Some(0));
    }

    /// Oracle: `(2^63 - 1/2) d`, one unit either side, and its negation, against the `i64`
    /// range. `2^63 - 1/2` is a tie between `2^63 - 1` (odd) and `2^63` (even, outside).
    #[wasm_bindgen_test(unsupported = test)]
    fn the_range_boundary_is_exact_one_unit_either_side() {
        let d: i128 = (1 << 100) + 12_345;
        let tie = I320::product((1 << 64) - 1, d); // (2^64 - 1) d = 2 (2^63 - 1/2) d
        let half = |n: I320| quotient(n, 2 * d);
        let one = I320::from_i128(1);
        let got = [half(tie.minus(one)), half(tie), half(tie.plus(one))];
        // dojo: allow panic-path -- test assertion against hand-computed values
        assert_eq!(got, [Some(i64::MAX), None, None]);
        // -(2^63 - 1/2) is a tie between -2^63 + 1 (odd) and -2^63 (even, inside).
        let low = [tie.minus(one), tie, tie.plus(one)].map(|n| half(n.negated()));
        // dojo: allow panic-path -- test assertion against hand-computed values
        assert_eq!(low, [Some(i64::MIN + 1), Some(i64::MIN), Some(i64::MIN)]);
        // -(2^63 + 1/2) is a tie between -2^63 (even, inside) and -2^63 - 1 (outside).
        let edge = I320::product((1 << 64) + 1, d);
        let low = [edge.minus(one), edge, edge.plus(one)].map(|n| half(n.negated()));
        // dojo: allow panic-path -- test assertion against hand-computed values
        assert_eq!(low, [Some(i64::MIN), Some(i64::MIN), None]);
        let beyond = I320::product(1 << 64, d);
        // dojo: allow panic-path -- test assertion: a quotient of 2^64 is refused early
        assert_eq!([quotient(beyond, d), quotient(beyond.negated(), d)], [None, None]);
    }

    /// Oracle: hand-computed wide products and sums across every limb boundary.
    #[wasm_bindgen_test(unsupported = test)]
    fn wide_products_and_sums_are_exact() {
        let max = i128::MAX;
        // (2^127 - 1)^2 = 2^254 - 2^128 + 1 = 2^128 (2^126 - 1) + 1.
        let square = I320::product(max, max);
        // dojo: allow panic-path -- test assertion against hand-computed values
        assert_eq!(square, I320([1, 0, u64::MAX, (1 << 62) - 1, 0]));
        // dojo: allow panic-path -- test assertion: (-x) x = -(x x), and x x is positive
        assert!(I320::product(-max, max) == square.negated() && !square.is_negative());
        // dojo: allow panic-path -- test assertion: 2^127 2^126 2^64 = 2^317, the top limb
        assert_eq!(
            I320::product(1 << 126, 2).times(I320::product(1 << 126, 1 << 64)),
            I320([0, 0, 0, 0, 1 << 61])
        );
        let one = I320::from_i128(1);
        // dojo: allow panic-path -- test assertion: the carries and borrows of 2^128 - 1 + 1
        assert_eq!(
            I320::from_i128(u64::MAX.into())
                .times(I320::from_i128(1 << 64))
                .plus(I320::from_i128(u64::MAX.into()))
                .plus(one),
            I320([0, 0, 1, 0, 0])
        );
        // dojo: allow panic-path -- test assertion: 0 - 1 is -1 in every limb
        assert_eq!(I320::from_i128(0).minus(one), I320([u64::MAX; 5]));
    }
}
