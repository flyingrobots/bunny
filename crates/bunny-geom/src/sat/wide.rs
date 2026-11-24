//! A 256-bit two's complement integer for the separating-axis kernels.
//!
//! Every value the kernels form is below `2^170` in magnitude (the widths are stated at each
//! use in `sat.rs` and in `docs/topics/convex-collision/README.md`), so the wrapping
//! operations here never wrap: addition, subtraction and the low 256 bits of a product are
//! the exact results. Only the operations the kernels need are here; flyingrobots/bunny#424
//! tracks sharing one wide integer across the workspace.

use core::cmp::Ordering;

use crate::ExactDeterminant;

/// A signed integer in four little-endian 64-bit limbs, two's complement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct I256([u64; 4]);

impl I256 {
    /// Zero.
    pub(super) const ZERO: Self = Self([0; 4]);

    /// The exact value of `v`, sign extended.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub(super) const fn from_i128(v: i128) -> Self {
        let fill = if v < 0 { u64::MAX } else { 0 };
        Self([v as u64, (v >> 64) as u64, fill, fill])
    }

    /// The exact value of `v`.
    pub(super) fn from_i64(v: i64) -> Self {
        Self::from_i128(i128::from(v))
    }

    /// The exact value of a predicate's determinant, whose magnitude is below `2^194`.
    pub(super) fn from_exact(d: &ExactDeterminant) -> Self {
        let value = Self(d.magnitude());
        if d.is_negative() {
            value.neg()
        } else {
            value
        }
    }

    /// `self + rhs`.
    pub(super) fn add(self, rhs: Self) -> Self {
        let mut out = [0_u64; 4];
        let mut carry = false;
        for (slot, (&a, &b)) in out.iter_mut().zip(self.0.iter().zip(&rhs.0)) {
            let (sum, first) = a.overflowing_add(b);
            let (sum, second) = sum.overflowing_add(u64::from(carry));
            *slot = sum;
            carry = first || second;
        }
        Self(out)
    }

    /// `-self`.
    pub(super) fn neg(self) -> Self {
        Self(self.0.map(|limb| !limb)).add(Self::from_i64(1))
    }

    /// `self - rhs`.
    pub(super) fn sub(self, rhs: Self) -> Self {
        self.add(rhs.neg())
    }

    /// The low 256 bits of `self * rhs`, which is the exact product while it is below `2^255`
    /// in magnitude.
    pub(super) fn mul(self, rhs: Self) -> Self {
        let mut out = [0_u64; 4];
        for (i, &a) in self.0.iter().enumerate() {
            let mut carry = 0_u128;
            for (slot, &b) in out.iter_mut().skip(i).zip(&rhs.0) {
                let t = u128::from(*slot) + u128::from(a) * u128::from(b) + carry;
                *slot = low(t);
                carry = t >> 64;
            }
        }
        Self(out)
    }

    /// `self * 2^32`.
    pub(super) fn shl32(self) -> Self {
        self.mul(Self::from_i64(1 << 32))
    }

    /// Whether the value is below zero.
    pub(super) const fn is_negative(self) -> bool {
        let [_, _, _, top] = self.0;
        top >> 63 == 1
    }

    /// Whether the value is zero.
    pub(super) fn is_zero(self) -> bool {
        self == Self::ZERO
    }

    /// `|self|`.
    pub(super) fn abs(self) -> Self {
        if self.is_negative() {
            self.neg()
        } else {
            self
        }
    }
}

impl Ord for I256 {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.is_negative(), other.is_negative()) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => self.0.iter().rev().cmp(other.0.iter().rev()),
        }
    }
}

impl PartialOrd for I256 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The low 64 bits.
#[allow(clippy::cast_possible_truncation)]
const fn low(value: u128) -> u64 {
    value as u64
}
