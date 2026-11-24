//! Signed integers of 512 bits for the exact unprojection.
//!
//! Every quantity of `bunny-unproject/1` is a ratio of products of raw Q32.32 values, viewport
//! sizes and `2^32`. The largest numerator is below `2^478` and the largest denominator below
//! `2^509` (`docs/topics/coordinate-law/README.md#unprojection`), so 512 bits of magnitude hold
//! every intermediate exactly. Only the operations the kernel needs are here.

/// Number of 64-bit limbs in a magnitude.
const LIMBS: usize = 8;

/// A 512-bit magnitude in little-endian 64-bit limbs.
type Limbs = [u64; LIMBS];

/// A sign and a 512-bit magnitude; zero is never negative.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Wide {
    negative: bool,
    magnitude: Limbs,
}

impl Wide {
    /// Zero.
    pub(super) const ZERO: Self = Self { negative: false, magnitude: [0; LIMBS] };

    /// The exact value of an `i128`.
    pub(super) fn from_i128(value: i128) -> Self {
        let abs = value.unsigned_abs();
        let mut magnitude = [0; LIMBS];
        for (k, limb) in magnitude.iter_mut().take(2).enumerate() {
            *limb = low(abs >> (64 * k));
        }
        Self { negative: value < 0, magnitude }
    }

    /// Whether the value is zero.
    pub(super) fn is_zero(&self) -> bool {
        is_zero(&self.magnitude)
    }

    fn signed(negative: bool, magnitude: Limbs) -> Self {
        Self { negative: negative && !is_zero(&magnitude), magnitude }
    }

    /// `-self`.
    pub(super) fn negated(self) -> Self {
        Self::signed(!self.negative, self.magnitude)
    }

    /// `self * rhs`, or `None` when the magnitude would need more than 512 bits.
    pub(super) fn checked_mul(self, rhs: Self) -> Option<Self> {
        let mut out = [0_u64; 2 * LIMBS];
        for (i, &a) in self.magnitude.iter().enumerate() {
            let mut carry = 0_u128;
            for (slot, &b) in out.iter_mut().skip(i).zip(&rhs.magnitude) {
                let t = u128::from(*slot) + u128::from(a) * u128::from(b) + carry;
                *slot = low(t);
                carry = t >> 64;
            }
            if let Some(slot) = out.get_mut(i + LIMBS) {
                *slot = low(carry);
            }
        }
        let (kept, spilled) = out.split_at(LIMBS);
        if !is_zero(spilled) {
            return None;
        }
        let mut magnitude = [0; LIMBS];
        magnitude.copy_from_slice(kept);
        Some(Self::signed(self.negative != rhs.negative, magnitude))
    }

    /// `self + rhs`, or `None` when the magnitude would need more than 512 bits.
    pub(super) fn checked_add(self, rhs: Self) -> Option<Self> {
        if self.negative == rhs.negative {
            return Some(Self::signed(self.negative, add(&self.magnitude, &rhs.magnitude)?));
        }
        let (large, small) =
            if less(&self.magnitude, &rhs.magnitude) { (rhs, self) } else { (self, rhs) };
        Some(Self::signed(large.negative, sub(&large.magnitude, &small.magnitude)))
    }

    /// `self - rhs`, or `None` when the magnitude would need more than 512 bits.
    pub(super) fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.checked_add(rhs.negated())
    }

    /// `self / divisor` rounded to nearest, ties to even, as an `i64`; `None` when the divisor
    /// is zero, its magnitude is `2^511` or more, or the quotient is outside `i64`.
    pub(super) fn round_div(self, divisor: Self) -> Option<i64> {
        let top = divisor.magnitude.last().copied().unwrap_or(0);
        if divisor.is_zero() || top >> 63 != 0 {
            return None;
        }
        let (quotient, remainder) = divide(&self.magnitude, &divisor.magnitude);
        let rounded = round_half_even(quotient, &remainder, &divisor.magnitude)?;
        let (first, rest) = rounded.split_first()?;
        if !is_zero(rest) {
            return None;
        }
        let value = i128::from(*first);
        i64::try_from(if self.negative == divisor.negative { value } else { -value }).ok()
    }
}

/// The low 64 bits.
#[allow(clippy::cast_possible_truncation)]
const fn low(value: u128) -> u64 {
    value as u64
}

fn is_zero(limbs: &[u64]) -> bool {
    limbs.iter().all(|&limb| limb == 0)
}

/// `quotient`, plus one when twice the remainder exceeds the divisor, or equals it and the
/// quotient is odd.
fn round_half_even(quotient: Limbs, remainder: &Limbs, divisor: &Limbs) -> Option<Limbs> {
    let twice = add(remainder, remainder)?;
    let odd = quotient.first().is_some_and(|limb| limb & 1 == 1);
    if less(divisor, &twice) || (twice == *divisor && odd) {
        let mut one = [0; LIMBS];
        *one.first_mut()? = 1;
        add(&quotient, &one)
    } else {
        Some(quotient)
    }
}

/// `a < b` on magnitudes.
fn less(a: &Limbs, b: &Limbs) -> bool {
    a.iter().rev().cmp(b.iter().rev()).is_lt()
}

/// `a + b`, or `None` on a carry out of the top limb.
fn add(a: &Limbs, b: &Limbs) -> Option<Limbs> {
    let mut out = [0; LIMBS];
    let mut carry = 0_u128;
    for (slot, (&x, &y)) in out.iter_mut().zip(a.iter().zip(b)) {
        let t = u128::from(x) + u128::from(y) + carry;
        *slot = low(t);
        carry = t >> 64;
    }
    (carry == 0).then_some(out)
}

/// `a - b` for `b <= a`.
fn sub(a: &Limbs, b: &Limbs) -> Limbs {
    let mut out = [0; LIMBS];
    let mut borrow = false;
    for (slot, (&x, &y)) in out.iter_mut().zip(a.iter().zip(b)) {
        let (step, first) = x.overflowing_sub(y);
        let (step, second) = step.overflowing_sub(u64::from(borrow));
        *slot = step;
        borrow = first || second;
    }
    out
}

/// `2 limbs + bit`, dropping the top bit, which is zero while the remainder is below `2^511`.
fn shift_in(limbs: &mut Limbs, bit: u64) {
    let mut carry = bit;
    for limb in limbs {
        let next = *limb >> 63;
        *limb = (*limb << 1) | carry;
        carry = next;
    }
}

/// Schoolbook binary long division of magnitudes, `divisor` nonzero and below `2^511`, so the
/// doubled remainder never leaves 512 bits.
fn divide(numerator: &Limbs, divisor: &Limbs) -> (Limbs, Limbs) {
    let mut quotient = [0_u64; LIMBS];
    let mut remainder = [0_u64; LIMBS];
    for (&limb, digit) in numerator.iter().zip(quotient.iter_mut()).rev() {
        for bit in (0..64).rev() {
            shift_in(&mut remainder, (limb >> bit) & 1);
            if !less(&remainder, divisor) {
                remainder = sub(&remainder, divisor);
                *digit |= 1 << bit;
            }
        }
    }
    (quotient, remainder)
}
