//! Signed integers of 256 bits for the exact predicates.
//!
//! Every value of `bunny-predicates/1` is built from differences of two raw `i64` values,
//! each of magnitude at most `2^64 - 1`. A product of two is below `2^128`, a product of
//! three below `2^192`, and the largest sum, the six terms of the 3D determinant, below
//! `6 2^192 < 2^195`. A barycentric numerator times `2^32` is below `2^160`. So 256 bits of
//! magnitude hold every intermediate exactly, with room to spare, and no operation here can
//! carry out of the top limb for any input the predicates pass it.
//!
//! The 2D queries of `bunny-planar/1` (`crate::planar`) use it too, under the same kind of
//! bound: a squared distance or squared radius sum is below `2^129`, and the largest value,
//! a closest-point numerator `a d + (b - a) n`, is below `2^63 2^129 + 2^64 2^129 < 2^194`.
//! The ray queries' largest value is `2^64` times a ray-circle discriminant, below `2^255`,
//! whose integer square root ([`Wide::isqrt`]) is below `2^128`.
//!
//! This is one more private wide integer in the workspace; flyingrobots/bunny#424 tracks
//! sharing one. Only the operations the predicates and the 2D queries need are here.

/// Number of 64-bit limbs in a magnitude.
const LIMBS: usize = 4;

/// A 256-bit magnitude in little-endian 64-bit limbs.
pub type Limbs = [u64; LIMBS];

/// A sign and a 256-bit magnitude; zero is never negative.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Wide {
    negative: bool,
    magnitude: Limbs,
}

impl Wide {
    /// The exact value `a - b`, whose magnitude is at most `2^64 - 1`.
    pub fn difference(a: i64, b: i64) -> Self {
        let value = i128::from(a) - i128::from(b);
        Self::from_u128(value < 0, value.unsigned_abs())
    }

    /// The value `2^k` for `k < 64`.
    pub const fn power_of_two(k: u32) -> Self {
        Self { negative: false, magnitude: [1 << k, 0, 0, 0] }
    }

    /// The value `magnitude`.
    pub fn from_unsigned(magnitude: u128) -> Self {
        Self::from_u128(false, magnitude)
    }

    fn from_u128(negative: bool, magnitude: u128) -> Self {
        Self::signed(negative, [low(magnitude), low(magnitude >> 64), 0, 0])
    }

    fn signed(negative: bool, magnitude: Limbs) -> Self {
        Self { negative: negative && !is_zero(&magnitude), magnitude }
    }

    /// Whether the value is zero.
    pub fn is_zero(&self) -> bool {
        is_zero(&self.magnitude)
    }

    /// Whether the value is below zero.
    pub const fn is_negative(&self) -> bool {
        self.negative
    }

    /// The magnitude, little-endian.
    pub const fn magnitude(&self) -> Limbs {
        self.magnitude
    }

    /// `-self`.
    pub fn negated(self) -> Self {
        Self::signed(!self.negative, self.magnitude)
    }

    /// `self * rhs`. The exact product must be below `2^256`, as every product the predicates
    /// form is (below `2^192`); limbs above the fourth are not formed.
    pub fn times(self, rhs: Self) -> Self {
        let mut out = [0_u64; LIMBS];
        for (i, &a) in self.magnitude.iter().enumerate() {
            let mut carry = 0_u128;
            for (slot, &b) in out.iter_mut().skip(i).zip(&rhs.magnitude) {
                let t = u128::from(*slot) + u128::from(a) * u128::from(b) + carry;
                *slot = low(t);
                carry = t >> 64;
            }
        }
        Self::signed(self.negative != rhs.negative, out)
    }

    /// `self + rhs`. The exact sum must be below `2^256` in magnitude, as every sum the
    /// predicates form is (below `2^195`).
    pub fn plus(self, rhs: Self) -> Self {
        if self.negative == rhs.negative {
            return Self::signed(self.negative, add(&self.magnitude, &rhs.magnitude));
        }
        let (large, small) =
            if less(&self.magnitude, &rhs.magnitude) { (rhs, self) } else { (self, rhs) };
        Self::signed(large.negative, sub(&large.magnitude, &small.magnitude))
    }

    /// `self - rhs`, under the bound of [`Self::plus`].
    pub fn minus(self, rhs: Self) -> Self {
        self.plus(rhs.negated())
    }

    /// `floor(sqrt(self))` for a value in `[0, 2^256)`, digit by digit from the top: each
    /// candidate is below `2^128`, so its square is below `2^256` and exact. A negative value
    /// is read by its magnitude; no caller passes one.
    pub fn isqrt(self) -> u128 {
        let mut root = 0_u128;
        for bit in (0..128).rev() {
            let candidate = Self::from_unsigned(root | (1 << bit));
            if !less(&self.magnitude, &candidate.times(candidate).magnitude) {
                root |= 1 << bit;
            }
        }
        root
    }

    /// `self / divisor` rounded once to nearest, ties to even, as an `i64`; `None` when the
    /// rounded quotient is outside `i64`. `divisor` is a positive magnitude below `2^255`.
    pub fn round_div(self, divisor: &Limbs) -> Option<i64> {
        let (quotient, remainder) = divide(&self.magnitude, divisor);
        let twice = add(&remainder, &remainder);
        let odd = quotient.first().is_some_and(|limb| limb & 1 == 1);
        let up = less(divisor, &twice) || (twice == *divisor && odd);
        let rounded = if up { add(&quotient, &[1, 0, 0, 0]) } else { quotient };
        let (first, rest) = rounded.split_first()?;
        if !is_zero(rest) {
            return None;
        }
        let magnitude = i128::from(*first);
        i64::try_from(if self.negative { -magnitude } else { magnitude }).ok()
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

/// `a < b` on magnitudes.
fn less(a: &Limbs, b: &Limbs) -> bool {
    a.iter().rev().cmp(b.iter().rev()).is_lt()
}

/// `a + b`; the carry out of the top limb is zero under every caller's bound.
fn add(a: &Limbs, b: &Limbs) -> Limbs {
    let mut out = [0; LIMBS];
    let mut carry = 0_u128;
    for (slot, (&x, &y)) in out.iter_mut().zip(a.iter().zip(b)) {
        let t = u128::from(x) + u128::from(y) + carry;
        *slot = low(t);
        carry = t >> 64;
    }
    out
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

/// `2 limbs + bit`, dropping the top bit, which is zero while the remainder is below `2^255`.
fn shift_in(limbs: &mut Limbs, bit: u64) {
    let mut carry = bit;
    for limb in limbs {
        let next = *limb >> 63;
        *limb = (*limb << 1) | carry;
        carry = next;
    }
}

/// Schoolbook binary long division of magnitudes, `divisor` nonzero and below `2^255`, so
/// the doubled remainder never leaves 256 bits.
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

#[cfg(test)]
mod tests {
    use super::Wide;
    use wasm_bindgen_test::wasm_bindgen_test;

    /// Oracle: squares built by multiplication, and one either side, from zero to the top of
    /// the 256-bit range, where `(2^128 - 1)^2 + 2 (2^128 - 1) = 2^256 - 1`.
    #[wasm_bindgen_test(unsupported = test)]
    fn planar_tp_022_isqrt_is_exact_at_squares_and_one_either_side() {
        let one = Wide::power_of_two(0);
        for k in [1, 2, 3, 1 << 32, (1 << 64) - 1, 1 << 64, 1 << 127, u128::MAX - 1, u128::MAX] {
            let root = Wide::from_unsigned(k);
            let square = root.times(root);
            // dojo: allow panic-path -- test assertion against squares built by multiplication
            assert_eq!(square.isqrt(), k, "{k}");
            // dojo: allow panic-path -- test assertion: one below a square has the root below
            assert_eq!(square.minus(one).isqrt(), k - 1, "{k}");
            // dojo: allow panic-path -- test assertion: up to 2k above a square keeps the root
            assert_eq!(square.plus(root).plus(root).isqrt(), k, "{k}");
        }
        // dojo: allow panic-path -- test assertion: zero, and the values below four
        assert_eq!([0, 1, 2, 3, 4].map(|v| Wide::from_unsigned(v).isqrt()), [0, 1, 1, 1, 2]);
    }
}
