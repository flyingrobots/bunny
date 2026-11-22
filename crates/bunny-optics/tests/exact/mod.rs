//! Exact comparisons for the unit-vector oracles: unsigned and signed big integers and the test
//! `|r - s 2^32 / |S|| <= bound (+ slack / |S|)` decided by comparing squares, with no square root.

// Limbs are below 2^32 by construction, so the `as u32` conversions keep every bit.
#![allow(clippy::cast_possible_truncation)]

use std::cmp::Ordering;

/// An unsigned integer of any size: little-endian base-`2^32` limbs, no trailing zero limb.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Big(Vec<u32>);

impl Big {
    pub(crate) fn new(value: u128) -> Self {
        let limbs = (0..4).map(|k| (value >> (32 * k)) as u32).collect();
        Self(limbs).trim()
    }

    fn trim(mut self) -> Self {
        while self.0.last() == Some(&0) {
            self.0.pop();
        }
        self
    }

    pub(crate) fn mul(&self, other: &Self) -> Self {
        let mut out = vec![0_u64; self.0.len() + other.0.len() + 1];
        for (i, &a) in self.0.iter().enumerate() {
            let mut carry = 0_u64;
            for (j, &b) in other.0.iter().enumerate() {
                let t = out[i + j] + u64::from(a) * u64::from(b) + carry;
                out[i + j] = t & 0xFFFF_FFFF;
                carry = t >> 32;
            }
            out[i + other.0.len()] += carry;
        }
        Self(out.into_iter().map(|limb| limb as u32).collect()).trim()
    }

    pub(crate) fn add(&self, other: &Self) -> Self {
        let mut out = Vec::new();
        let mut carry = 0_u64;
        for k in 0..=self.0.len().max(other.0.len()) {
            let limb = |b: &Self| u64::from(b.0.get(k).copied().unwrap_or(0));
            let t = limb(self) + limb(other) + carry;
            out.push((t & 0xFFFF_FFFF) as u32);
            carry = t >> 32;
        }
        Self(out).trim()
    }

    /// `self - other`, for `other <= self`.
    fn sub(&self, other: &Self) -> Self {
        let mut out = Vec::new();
        let mut borrow = 0_i64;
        for (k, &a) in self.0.iter().enumerate() {
            let t = i64::from(a) - i64::from(other.0.get(k).copied().unwrap_or(0)) - borrow;
            borrow = i64::from(t < 0);
            out.push(u32::try_from(t + (borrow << 32)).expect("a limb"));
        }
        Self(out).trim()
    }

    pub(crate) fn compare(&self, other: &Self) -> Ordering {
        self.0.len().cmp(&other.0.len()).then_with(|| self.0.iter().rev().cmp(other.0.iter().rev()))
    }
}

/// A signed integer of any size.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Signed {
    negative: bool,
    magnitude: Big,
}

impl Signed {
    pub(crate) fn new(value: i128) -> Self {
        Self { negative: value < 0, magnitude: Big::new(value.unsigned_abs()) }
    }

    /// `a * b * 2^shift`, exactly.
    pub(crate) fn product(a: i128, b: i128, shift: u32) -> Self {
        let scale = Big::new(1 << shift);
        let magnitude = Big::new(a.unsigned_abs()).mul(&Big::new(b.unsigned_abs())).mul(&scale);
        Self { negative: (a < 0) != (b < 0) && magnitude != Big::new(0), magnitude }
    }

    pub(crate) fn add(&self, other: &Self) -> Self {
        if self.negative == other.negative {
            return Self {
                negative: self.negative,
                magnitude: self.magnitude.add(&other.magnitude),
            };
        }
        let (big, small) = match self.magnitude.compare(&other.magnitude) {
            Ordering::Less => (other, self),
            _ => (self, other),
        };
        let magnitude = big.magnitude.sub(&small.magnitude);
        Self { negative: big.negative && magnitude != Big::new(0), magnitude }
    }

    pub(crate) fn negated(&self) -> Self {
        Self {
            negative: !self.negative && self.magnitude != Big::new(0),
            magnitude: self.magnitude.clone(),
        }
    }

    pub(crate) fn square(&self) -> Big {
        self.magnitude.mul(&self.magnitude)
    }
}

/// Whether `a * sqrt(q) <= t`, exactly.
fn at_most(a: i128, t: &Signed, q: &Big) -> bool {
    let a_magnitude = Big::new(a.unsigned_abs());
    let lhs = a_magnitude.mul(&a_magnitude).mul(q);
    let rhs = t.square();
    if t.negative {
        a < 0 && lhs.compare(&rhs) != Ordering::Less
    } else {
        a <= 0 || lhs.compare(&rhs) != Ordering::Greater
    }
}

/// `|S|^2` for an exact vector `S`.
pub(crate) fn squared_length(sum: &[Signed; 3]) -> Big {
    sum.iter().fold(Big::new(0), |q, s| q.add(&s.square()))
}

/// Whether every raw component `r` of `raw` satisfies `|r |S| - s 2^32| <= bound |S| + slack`
/// for the nonzero exact vector `S = sum`: within `bound + slack / |S|` raw units of the exact
/// unit vector.
pub(crate) fn within(raw: [i64; 3], sum: &[Signed; 3], bound: i128, slack: &Signed) -> bool {
    let q = squared_length(sum);
    raw.iter().zip(sum).all(|(component, s)| {
        let r = i128::from(*component);
        let scaled = Signed::product(1, 1, 32).magnitude.mul(&s.magnitude);
        let scaled = Signed { negative: s.negative, magnitude: scaled };
        at_most(r - bound, &scaled.add(slack), &q)
            && at_most(-(r + bound), &scaled.negated().add(slack), &q)
    })
}
