//! A signed arbitrary-precision integer for the transform-bounds oracle, written for these
//! tests and sharing nothing with the kernel: sign and magnitude, base `2^32` limbs, schoolbook
//! addition, subtraction and multiplication. It is slow and simple on purpose.

use core::cmp::Ordering;

/// A signed integer of any size. Zero is never negative; the magnitude has no high zero limbs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Big {
    negative: bool,
    magnitude: Vec<u32>,
}

impl Big {
    pub(crate) fn from_i128(value: i128) -> Self {
        let mut rest = value.unsigned_abs();
        let mut magnitude = Vec::new();
        while rest != 0 {
            magnitude.push(u32::try_from(rest & 0xFFFF_FFFF).expect("one limb"));
            rest >>= 32;
        }
        Self { negative: value < 0, magnitude }
    }

    pub(crate) fn add(&self, other: &Self) -> Self {
        if self.negative == other.negative {
            return Self::signed(self.negative, add_magnitudes(&self.magnitude, &other.magnitude));
        }
        match compare_magnitudes(&self.magnitude, &other.magnitude) {
            Ordering::Less => {
                Self::signed(other.negative, sub_magnitudes(&other.magnitude, &self.magnitude))
            }
            _ => Self::signed(self.negative, sub_magnitudes(&self.magnitude, &other.magnitude)),
        }
    }

    pub(crate) fn sub(&self, other: &Self) -> Self {
        self.add(&Self {
            negative: !other.negative && !other.magnitude.is_empty(),
            ..other.clone()
        })
    }

    pub(crate) fn mul(&self, other: &Self) -> Self {
        let mut out = vec![0_u64; self.magnitude.len() + other.magnitude.len() + 1];
        for (i, &a) in self.magnitude.iter().enumerate() {
            let mut carry = 0_u64;
            for (j, &b) in other.magnitude.iter().enumerate() {
                let cell = out[i + j] + u64::from(a) * u64::from(b) + carry;
                out[i + j] = cell & 0xFFFF_FFFF;
                carry = cell >> 32;
            }
            out[i + other.magnitude.len()] += carry;
        }
        let limbs = out.into_iter().map(|c| u32::try_from(c).expect("one limb")).collect();
        Self::signed(self.negative != other.negative, limbs)
    }

    pub(crate) fn is_negative(&self) -> bool {
        self.negative
    }

    fn signed(negative: bool, mut magnitude: Vec<u32>) -> Self {
        while magnitude.last() == Some(&0) {
            magnitude.pop();
        }
        Self { negative: negative && !magnitude.is_empty(), magnitude }
    }
}

impl Ord for Big {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.negative, other.negative) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (false, false) => compare_magnitudes(&self.magnitude, &other.magnitude),
            (true, true) => compare_magnitudes(&other.magnitude, &self.magnitude),
        }
    }
}

impl PartialOrd for Big {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn compare_magnitudes(a: &[u32], b: &[u32]) -> Ordering {
    a.len().cmp(&b.len()).then_with(|| a.iter().rev().cmp(b.iter().rev()))
}

fn add_magnitudes(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut out = Vec::new();
    let mut carry = 0_u64;
    for i in 0..a.len().max(b.len()) {
        let sum = u64::from(*a.get(i).unwrap_or(&0)) + u64::from(*b.get(i).unwrap_or(&0)) + carry;
        out.push(u32::try_from(sum & 0xFFFF_FFFF).expect("one limb"));
        carry = sum >> 32;
    }
    out.push(u32::try_from(carry).expect("one limb"));
    out
}

/// `a - b` for `a >= b`.
fn sub_magnitudes(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut out = Vec::new();
    let mut borrow = 0_i64;
    for (i, &limb) in a.iter().enumerate() {
        let mut cell = i64::from(limb) - i64::from(*b.get(i).unwrap_or(&0)) - borrow;
        borrow = i64::from(cell < 0);
        cell += borrow << 32;
        out.push(u32::try_from(cell).expect("one limb"));
    }
    out
}
