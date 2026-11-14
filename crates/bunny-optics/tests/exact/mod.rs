//! Exact comparisons for the unit-vector oracles: an unsigned big integer and the test
//! `|r - s 2^32 / |S|| <= bound` decided by comparing squares, with no square root.

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

    pub(crate) fn compare(&self, other: &Self) -> Ordering {
        self.0.len().cmp(&other.0.len()).then_with(|| self.0.iter().rev().cmp(other.0.iter().rev()))
    }
}

/// Whether `a * sqrt(q) <= s * 2^32`, exactly.
pub(crate) fn at_most(a: i128, s: i128, q: &Big) -> bool {
    let a_magnitude = Big::new(a.unsigned_abs());
    let lhs = a_magnitude.mul(&a_magnitude).mul(q);
    let s_scaled = Big::new(s.unsigned_abs()).mul(&Big::new(1 << 32));
    let rhs = s_scaled.mul(&s_scaled);
    if s >= 0 {
        a <= 0 || lhs.compare(&rhs) != Ordering::Greater
    } else {
        a < 0 && lhs.compare(&rhs) != Ordering::Less
    }
}

/// `|S|^2` for an exact vector `S`.
pub(crate) fn squared_length(sum: [i128; 3]) -> Big {
    let square = |s: i128| Big::new(s.unsigned_abs()).mul(&Big::new(s.unsigned_abs()));
    sum.iter().fold(Big::new(0), |q, &s| q.add(&square(s)))
}

/// Whether every raw component `r` of `raw` is within `bound` raw units of `s 2^32 / |S|`, the
/// exact unit vector along the nonzero `sum`.
pub(crate) fn within_unit(raw: [i64; 3], sum: [i128; 3], bound: i128) -> bool {
    let q = squared_length(sum);
    raw.iter().zip(sum).all(|(component, s)| {
        let r = i128::from(*component);
        at_most(r - bound, s, &q) && at_most(-(r + bound), -s, &q)
    })
}
