//! The independent oracle of the quaternion tests (`bunny-quaternion/1`).
//!
//! Shares no code with the kernel: signed arbitrary-precision integers over 32-bit limbs (the
//! kernel uses two `u128` words), the Hamilton product written out from `i^2 = j^2 = k^2 = ijk
//! = -1`, vector rotation as the sandwich `q v q*` of two such products (the kernel expands a
//! matrix of polynomials), and normalisation checked by its defining inequality instead of
//! computed.

use std::cmp::Ordering;

/// A signed integer of any size: `negative` and little-endian 32-bit limbs, no trailing zeros.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Big {
    negative: bool,
    limbs: Vec<u32>,
}

impl Big {
    pub(crate) fn from_i128(value: i128) -> Self {
        let mut magnitude = value.unsigned_abs();
        let mut limbs = Vec::new();
        while magnitude > 0 {
            limbs.push(u32::try_from(magnitude & 0xFFFF_FFFF).expect("one limb"));
            magnitude >>= 32;
        }
        Self { negative: value < 0, limbs }
    }

    pub(crate) fn is_zero(&self) -> bool {
        self.limbs.is_empty()
    }

    pub(crate) fn is_negative(&self) -> bool {
        self.negative && !self.is_zero()
    }

    pub(crate) fn abs(&self) -> Self {
        Self { negative: false, limbs: self.limbs.clone() }
    }

    pub(crate) fn neg(&self) -> Self {
        Self { negative: !self.negative && !self.is_zero(), limbs: self.limbs.clone() }
    }

    pub(crate) fn mul(&self, rhs: &Self) -> Self {
        let mut out = vec![0_u64; self.limbs.len() + rhs.limbs.len() + 1];
        for (i, &a) in self.limbs.iter().enumerate() {
            let mut carry = 0_u64;
            for (j, &b) in rhs.limbs.iter().enumerate() {
                let cell = out[i + j] + u64::from(a) * u64::from(b) + carry;
                out[i + j] = cell & 0xFFFF_FFFF;
                carry = cell >> 32;
            }
            let mut k = i + rhs.limbs.len();
            while carry > 0 {
                let cell = out[k] + carry;
                out[k] = cell & 0xFFFF_FFFF;
                carry = cell >> 32;
                k += 1;
            }
        }
        let limbs = out.into_iter().map(|l| u32::try_from(l).expect("one limb")).collect();
        Self::trimmed(self.negative != rhs.negative, limbs)
    }

    pub(crate) fn add(&self, rhs: &Self) -> Self {
        if self.negative == rhs.negative {
            return Self::trimmed(self.negative, add_magnitudes(&self.limbs, &rhs.limbs));
        }
        match cmp_magnitudes(&self.limbs, &rhs.limbs) {
            Ordering::Less => Self::trimmed(rhs.negative, sub_magnitudes(&rhs.limbs, &self.limbs)),
            _ => Self::trimmed(self.negative, sub_magnitudes(&self.limbs, &rhs.limbs)),
        }
    }

    pub(crate) fn sub(&self, rhs: &Self) -> Self {
        self.add(&rhs.neg())
    }

    pub(crate) fn shl(&self, bits: u32) -> Self {
        self.mul(&Self::from_i128(1_i128 << bits))
    }

    pub(crate) fn cmp(&self, rhs: &Self) -> Ordering {
        match (self.is_negative(), rhs.is_negative()) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (false, false) => cmp_magnitudes(&self.limbs, &rhs.limbs),
            (true, true) => cmp_magnitudes(&rhs.limbs, &self.limbs),
        }
    }

    pub(crate) fn lt(&self, rhs: &Self) -> bool {
        self.cmp(rhs) == Ordering::Less
    }

    pub(crate) fn le(&self, rhs: &Self) -> bool {
        self.cmp(rhs) != Ordering::Greater
    }

    fn trimmed(negative: bool, mut limbs: Vec<u32>) -> Self {
        while limbs.last() == Some(&0) {
            limbs.pop();
        }
        let negative = negative && !limbs.is_empty();
        Self { negative, limbs }
    }
}

fn add_magnitudes(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut out = Vec::new();
    let mut carry = 0_u64;
    for i in 0..a.len().max(b.len()) {
        let cell = u64::from(*a.get(i).unwrap_or(&0)) + u64::from(*b.get(i).unwrap_or(&0)) + carry;
        out.push(u32::try_from(cell & 0xFFFF_FFFF).expect("one limb"));
        carry = cell >> 32;
    }
    out.push(u32::try_from(carry).expect("one limb"));
    out
}

/// `a - b` for `|a| >= |b|`.
fn sub_magnitudes(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut out = Vec::new();
    let mut borrow = 0_i64;
    for (i, &limb) in a.iter().enumerate() {
        let mut cell = i64::from(limb) - i64::from(*b.get(i).unwrap_or(&0)) - borrow;
        borrow = 0;
        if cell < 0 {
            cell += 1 << 32;
            borrow = 1;
        }
        out.push(u32::try_from(cell).expect("one limb"));
    }
    out
}

fn cmp_magnitudes(a: &[u32], b: &[u32]) -> Ordering {
    let significant = |v: &[u32]| v.iter().rposition(|&l| l != 0).map_or(0, |p| p + 1);
    let (la, lb) = (significant(a), significant(b));
    if la != lb {
        return la.cmp(&lb);
    }
    for i in (0..la).rev() {
        match a[i].cmp(&b[i]) {
            Ordering::Equal => {}
            other => return other,
        }
    }
    Ordering::Equal
}

pub(crate) fn big(value: i128) -> Big {
    Big::from_i128(value)
}

/// A quaternion `[x, y, z, w]` of exact integers: `w + x i + y j + z k`.
pub(crate) type Quat = [Big; 4];

pub(crate) fn quat(raw: [i64; 4]) -> Quat {
    raw.map(|c| big(i128::from(c)))
}

/// The Hamilton product from the unit rules `ij = k`, `jk = i`, `ki = j`, `ji = -k`, `kj = -i`,
/// `ik = -j` and `i^2 = j^2 = k^2 = -1`, summed over the sixteen pairs of basis elements.
pub(crate) fn hamilton(a: &Quat, b: &Quat) -> Quat {
    // Basis indices 0 = i, 1 = j, 2 = k, 3 = 1. table[p][q] = (sign, index) of e_p e_q.
    const TABLE: [[(i128, usize); 4]; 4] = [
        [(-1, 3), (1, 2), (-1, 1), (1, 0)],
        [(-1, 2), (-1, 3), (1, 0), (1, 1)],
        [(1, 1), (-1, 0), (-1, 3), (1, 2)],
        [(1, 0), (1, 1), (1, 2), (1, 3)],
    ];
    let mut out: Quat = [big(0), big(0), big(0), big(0)];
    for (p, ap) in a.iter().enumerate() {
        for (q, bq) in b.iter().enumerate() {
            let (sign, index) = TABLE[p][q];
            out[index] = out[index].add(&ap.mul(bq).mul(&big(sign)));
        }
    }
    out
}

pub(crate) fn conjugate(a: &Quat) -> Quat {
    [a[0].neg(), a[1].neg(), a[2].neg(), a[3].clone()]
}

pub(crate) fn norm_squared(a: &Quat) -> Big {
    a.iter().fold(big(0), |sum, c| sum.add(&c.mul(c)))
}

/// `q v q*` for the pure quaternion `v`: `|q|^2` times the rotated vector, exactly.
pub(crate) fn sandwich(q: [i64; 4], v: [i64; 3]) -> [Big; 3] {
    let q = quat(q);
    let pure: Quat = [big(i128::from(v[0])), big(i128::from(v[1])), big(i128::from(v[2])), big(0)];
    let out = hamilton(&hamilton(&q, &pure), &conjugate(&q));
    assert!(out[3].is_zero(), "the scalar part of q v q* is zero");
    [out[0].clone(), out[1].clone(), out[2].clone()]
}

/// Whether `n` is the integer nearest `numerator / denominator` (`denominator > 0`), an exact
/// half going to the even one: `|2 numerator - 2 n denominator| <= denominator`, with equality
/// only for even `n`.
pub(crate) fn is_nearest(numerator: &Big, denominator: &Big, n: i64) -> bool {
    let twice_error = numerator.sub(&denominator.mul(&big(i128::from(n)))).mul(&big(2)).abs();
    match twice_error.cmp(denominator) {
        Ordering::Less => true,
        Ordering::Equal => n % 2 == 0,
        Ordering::Greater => false,
    }
}

/// Whether `n` is the integer nearest `r 2^32 / sqrt(s)` with the sign of `r`, and not at an
/// exact half: `(2|n| - 1)^2 s < 4 r^2 2^64 < (2|n| + 1)^2 s` (the left side only for `n != 0`).
pub(crate) fn is_nearest_normalised(r: &Big, s: &Big, n: i64) -> bool {
    if n != 0 && (n < 0) != r.is_negative() {
        return false;
    }
    let target = r.mul(r).shl(66);
    let m = big(i128::from(n.unsigned_abs()));
    let above = m.mul(&big(2)).add(&big(1));
    let below = m.mul(&big(2)).sub(&big(1));
    let upper = target.lt(&above.mul(&above).mul(s));
    let lower = n == 0 || below.mul(&below).mul(s).lt(&target);
    upper && lower
}

/// Whether every component of `out` is the nearest normalisation of `p`.
pub(crate) fn is_normalisation(p: &Quat, out: [i64; 4]) -> bool {
    let s = norm_squared(p);
    p.iter().zip(out).all(|(r, n)| is_nearest_normalised(r, &s, n))
}
