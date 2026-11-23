//! Signed 256-bit integers for the exact point-to-triangle query.
//!
//! Every value the query builds is below `2^252` in magnitude
//! (`docs/topics/mesh-queries/README.md#widths`), so 256 bits hold it exactly; a product of
//! two such values is compared in 512 bits. Only the operations the query needs are here.

use core::cmp::Ordering;

/// Limbs of a 256-bit value.
const LIMBS: usize = 4;

/// A signed 256-bit integer in two's complement, as little-endian 64-bit limbs.
///
/// Arithmetic wraps modulo `2^256`; every caller keeps true values below `2^255` in magnitude,
/// so a wrapped result is the true result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct I256([u64; LIMBS]);

impl I256 {
    pub(super) const ONE: Self = Self([1, 0, 0, 0]);

    /// The value of an `i128`, sign-extended.
    // The casts keep the low and high 64 bits of the two's complement bits on purpose.
    #[allow(clippy::cast_possible_truncation)]
    pub(super) const fn from_i128(value: i128) -> Self {
        let bits = value.cast_unsigned();
        let extension = if value < 0 { u64::MAX } else { 0 };
        Self([bits as u64, (bits >> 64) as u64, extension, extension])
    }

    /// `self + rhs`, wrapping.
    pub(super) fn add(self, rhs: Self) -> Self {
        let mut out = [0; LIMBS];
        let mut carry = false;
        for ((slot, a), b) in out.iter_mut().zip(self.0).zip(rhs.0) {
            let (sum, first) = a.overflowing_add(b);
            let (sum, second) = sum.overflowing_add(u64::from(carry));
            *slot = sum;
            carry = first | second;
        }
        Self(out)
    }

    /// `-self`, wrapping.
    pub(super) fn neg(self) -> Self {
        Self(self.0.map(|limb| !limb)).add(Self::ONE)
    }

    /// `self - rhs`, wrapping.
    pub(super) fn sub(self, rhs: Self) -> Self {
        self.add(rhs.neg())
    }

    /// `self * rhs`, wrapping: the low 256 bits of the product, which for two's complement
    /// operands are the low bits of the signed product.
    pub(super) fn mul(self, rhs: Self) -> Self {
        Self(product::<LIMBS>(self.0, rhs.0))
    }

    /// Whether the value is below zero.
    pub(super) const fn is_negative(self) -> bool {
        let [.., top] = self.0;
        top >> 63 == 1
    }

    /// `|self|`, for a value above `-2^255`.
    pub(super) fn abs(self) -> Self {
        if self.is_negative() {
            self.neg()
        } else {
            self
        }
    }

    /// `self * 2^n` for `n < 64`, wrapping.
    pub(super) fn shl(self, n: u32) -> Self {
        let mut out = [0; LIMBS];
        let mut carry = 0;
        for (slot, limb) in out.iter_mut().zip(self.0) {
            *slot = (limb << n) | carry;
            carry = limb.checked_shr(64 - n).unwrap_or(0);
        }
        Self(out)
    }

    /// Compares two values that are both at least zero.
    pub(super) fn cmp_unsigned(self, rhs: Self) -> Ordering {
        cmp_limbs(&self.0, &rhs.0)
    }

    /// Compares `self * rhs` with `other * other_rhs`, all four at least zero, exactly in
    /// 512 bits.
    pub(super) fn cmp_products(self, rhs: Self, other: Self, other_rhs: Self) -> Ordering {
        let lhs = product::<{ 2 * LIMBS }>(self.0, rhs.0);
        let rhs = product::<{ 2 * LIMBS }>(other.0, other_rhs.0);
        cmp_limbs(&lhs, &rhs)
    }
}

/// The low `N` limbs of the product of two 256-bit values.
// The casts keep the low 64 bits of each 128-bit partial sum on purpose.
#[allow(clippy::cast_possible_truncation)]
fn product<const N: usize>(lhs: [u64; LIMBS], rhs: [u64; LIMBS]) -> [u64; N] {
    let mut out = [0; N];
    for (i, a) in lhs.into_iter().enumerate() {
        let mut carry = 0_u128;
        for (slot, b) in out.iter_mut().skip(i).zip(rhs) {
            // At most (2^64 - 1)^2 + 2 (2^64 - 1) = 2^128 - 1: no overflow.
            let t = u128::from(a) * u128::from(b) + u128::from(*slot) + carry;
            *slot = t as u64;
            carry = t >> 64;
        }
        // Row `i` wrote limbs `i..i + 4`; limb `i + 4` is still zero, so the carry is its value.
        if let Some(slot) = out.get_mut(i + LIMBS) {
            *slot = carry as u64;
        }
    }
    out
}

/// Compares two little-endian unsigned limb arrays of the same length.
fn cmp_limbs(lhs: &[u64], rhs: &[u64]) -> Ordering {
    lhs.iter().rev().cmp(rhs.iter().rev())
}

/// `round(numerator / denominator)` to nearest, ties to even, for a positive denominator, a
/// quotient magnitude below `2^bits`, `bits <= 63`, and `denominator * 2^bits` below `2^255`.
///
/// The magnitude is divided by restoring long division from bit `bits - 1` down; the sign is
/// applied to the rounded magnitude, which is the same as rounding the signed quotient,
/// because ties to even is symmetric about zero.
pub(super) fn round_quotient(numerator: I256, denominator: I256, bits: u32) -> i64 {
    let mut remainder = numerator.abs();
    let mut quotient = 0_u64;
    for bit in (0..bits).rev() {
        let step = denominator.shl(bit);
        if remainder.cmp_unsigned(step) != Ordering::Less {
            remainder = remainder.sub(step);
            quotient |= 1 << bit;
        }
    }
    let twice = remainder.shl(1);
    let up = match twice.cmp_unsigned(denominator) {
        Ordering::Greater => true,
        Ordering::Equal => quotient & 1 == 1,
        Ordering::Less => false,
    };
    let magnitude = (quotient + u64::from(up)).cast_signed();
    if numerator.is_negative() {
        -magnitude
    } else {
        magnitude
    }
}
