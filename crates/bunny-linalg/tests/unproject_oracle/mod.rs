//! Independent exact oracle for `Viewport::try_unproject`.
//!
//! It shares nothing with the kernel. The kernel inverts the viewport map, the projection's
//! two supported shapes analytically and the view's linear part by its adjugate; this oracle
//! instead forms the full `clip_from_world = clip_from_view * view_from_world` product in
//! unbounded integers and solves `clip_from_world h = (x_ndc, y_ndc, z_ndc, 1)` by Cramer's
//! rule at depths 0 and 1, the general matrix inverse. Each world point is `h_xyz / h_w`.

// Some helpers serve only some of the test files that include this module.
#![allow(dead_code)]
// Limbs are below 2^32 by construction, so the `as u32` conversions keep every bit.
#![allow(clippy::cast_possible_truncation)]

use std::cmp::Ordering;

pub(crate) const ONE: i128 = 1 << 32;

/// A signed integer of any size: sign and little-endian base-`2^32` limbs, no trailing zero.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Int {
    negative: bool,
    limbs: Vec<u32>,
}

impl Int {
    pub(crate) fn new(value: i128) -> Self {
        let abs = value.unsigned_abs();
        let limbs = (0..4).map(|k| (abs >> (32 * k)) as u32).collect();
        Self::from_parts(value < 0, limbs)
    }

    fn from_parts(negative: bool, mut limbs: Vec<u32>) -> Self {
        while limbs.last() == Some(&0) {
            limbs.pop();
        }
        Self { negative: negative && !limbs.is_empty(), limbs }
    }

    pub(crate) fn sign(&self) -> Ordering {
        match (self.limbs.is_empty(), self.negative) {
            (true, _) => Ordering::Equal,
            (false, true) => Ordering::Less,
            (false, false) => Ordering::Greater,
        }
    }

    pub(crate) fn neg(&self) -> Self {
        Self::from_parts(!self.negative, self.limbs.clone())
    }

    pub(crate) fn abs(&self) -> Self {
        Self::from_parts(false, self.limbs.clone())
    }

    pub(crate) fn mul(&self, other: &Self) -> Self {
        let mut out = vec![0_u64; self.limbs.len() + other.limbs.len() + 1];
        for (i, &a) in self.limbs.iter().enumerate() {
            let mut carry = 0_u64;
            for (j, &b) in other.limbs.iter().enumerate() {
                let t = out[i + j] + u64::from(a) * u64::from(b) + carry;
                out[i + j] = t & 0xFFFF_FFFF;
                carry = t >> 32;
            }
            out[i + other.limbs.len()] += carry;
        }
        let limbs = out.into_iter().map(|limb| limb as u32).collect();
        Self::from_parts(self.negative != other.negative, limbs)
    }

    pub(crate) fn add(&self, other: &Self) -> Self {
        if self.negative == other.negative {
            return Self::from_parts(self.negative, add_limbs(&self.limbs, &other.limbs));
        }
        match compare_limbs(&self.limbs, &other.limbs) {
            Ordering::Less => {
                Self::from_parts(other.negative, sub_limbs(&other.limbs, &self.limbs))
            }
            _ => Self::from_parts(self.negative, sub_limbs(&self.limbs, &other.limbs)),
        }
    }

    pub(crate) fn sub(&self, other: &Self) -> Self {
        self.add(&other.neg())
    }

    pub(crate) fn compare(&self, other: &Self) -> Ordering {
        self.sub(other).sign()
    }

    /// The value as an `i128`, when it fits.
    pub(crate) fn to_i128(&self) -> Option<i128> {
        if self.limbs.len() > 4 {
            return None;
        }
        let magnitude =
            self.limbs.iter().rev().fold(0_u128, |acc, &limb| (acc << 32) | u128::from(limb));
        let value = i128::try_from(magnitude).ok()?;
        Some(if self.negative { -value } else { value })
    }
}

fn add_limbs(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut out = Vec::new();
    let mut carry = 0_u64;
    for k in 0..=a.len().max(b.len()) {
        let t = u64::from(*a.get(k).unwrap_or(&0)) + u64::from(*b.get(k).unwrap_or(&0)) + carry;
        out.push(t as u32);
        carry = t >> 32;
    }
    out
}

/// `a - b` for `b <= a`.
fn sub_limbs(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut out = Vec::new();
    let mut borrow = 0_i64;
    for (k, &x) in a.iter().enumerate() {
        let t = i64::from(x) - i64::from(*b.get(k).unwrap_or(&0)) - borrow;
        borrow = i64::from(t < 0);
        out.push((t + (borrow << 32)) as u32);
    }
    out
}

fn compare_limbs(a: &[u32], b: &[u32]) -> Ordering {
    let trimmed = |v: &[u32]| v.len() - v.iter().rev().take_while(|&&l| l == 0).count();
    let (la, lb) = (trimmed(a), trimmed(b));
    la.cmp(&lb).then_with(|| a[..la].iter().rev().cmp(b[..lb].iter().rev()))
}

/// An exact rational with a positive denominator.
#[derive(Clone, Debug)]
pub(crate) struct Ratio {
    pub(crate) num: Int,
    pub(crate) den: Int,
}

impl Ratio {
    pub(crate) fn new(num: Int, den: Int) -> Self {
        assert_ne!(den.sign(), Ordering::Equal, "zero denominator");
        if den.sign() == Ordering::Less {
            Self { num: num.neg(), den: den.neg() }
        } else {
            Self { num, den }
        }
    }

    pub(crate) fn int(value: i128) -> Self {
        Self::new(Int::new(value), Int::new(1))
    }

    pub(crate) fn frac(num: i128, den: i128) -> Self {
        Self::new(Int::new(num), Int::new(den))
    }

    pub(crate) fn add(&self, other: &Self) -> Self {
        let num = self.num.mul(&other.den).add(&other.num.mul(&self.den));
        Self::new(num, self.den.mul(&other.den))
    }

    pub(crate) fn sub(&self, other: &Self) -> Self {
        self.add(&other.neg())
    }

    pub(crate) fn neg(&self) -> Self {
        Self { num: self.num.neg(), den: self.den.clone() }
    }

    pub(crate) fn abs(&self) -> Self {
        Self { num: self.num.abs(), den: self.den.clone() }
    }

    pub(crate) fn mul(&self, other: &Self) -> Self {
        Self::new(self.num.mul(&other.num), self.den.mul(&other.den))
    }

    pub(crate) fn div(&self, other: &Self) -> Self {
        Self::new(self.num.mul(&other.den), self.den.mul(&other.num))
    }

    pub(crate) fn sign(&self) -> Ordering {
        self.num.sign()
    }

    pub(crate) fn compare(&self, other: &Self) -> Ordering {
        self.sub(other).sign()
    }

    /// The nearest integer, ties to even, or `None` when it is outside `i64`.
    pub(crate) fn round(&self) -> Option<i64> {
        let (quotient, remainder) = divide(&self.num.abs().limbs, &self.den.limbs)?;
        let has_remainder = !remainder.is_empty();
        // Floor and the remainder above it, for the positive denominator.
        let (floor, rest) = if self.num.sign() == Ordering::Less {
            let rest = if has_remainder { sub_limbs(&self.den.limbs, &remainder) } else { vec![] };
            (-quotient - i128::from(has_remainder), rest)
        } else {
            (quotient, remainder)
        };
        let up = match compare_limbs(&add_limbs(&rest, &rest), &self.den.limbs) {
            Ordering::Greater => true,
            Ordering::Equal => floor & 1 == 1,
            Ordering::Less => false,
        };
        i64::try_from(floor + i128::from(up)).ok()
    }
}

fn bit_length(v: &[u32]) -> usize {
    let top = v.iter().rposition(|&l| l != 0);
    top.map_or(0, |k| 32 * k + 32 - v[k].leading_zeros() as usize)
}

/// `v * 2^shift`.
fn shifted(v: &[u32], shift: usize) -> Vec<u32> {
    let mut out = vec![0_u32; shift / 32];
    let mut carry = 0_u64;
    for &limb in v {
        let t = (u64::from(limb) << (shift % 32)) | carry;
        out.push(t as u32);
        carry = t >> 32;
    }
    out.push(carry as u32);
    out
}

/// Quotient (when below `2^120`) and trimmed remainder of magnitudes, divisor nonzero.
fn divide(numerator: &[u32], divisor: &[u32]) -> Option<(i128, Vec<u32>)> {
    let (n, d) = (bit_length(numerator), bit_length(divisor));
    let mut remainder = numerator.to_vec();
    let mut quotient = 0_i128;
    if n >= d {
        if n - d > 120 {
            return None;
        }
        for k in (0..=n - d).rev() {
            let step = shifted(divisor, k);
            if compare_limbs(&step, &remainder) != Ordering::Greater {
                remainder = sub_limbs(&remainder, &step);
                quotient |= 1 << k;
            }
        }
    }
    while remainder.last() == Some(&0) {
        remainder.pop();
    }
    Some((quotient, remainder))
}

/// A raw 4 x 4 matrix, row-major.
pub(crate) type Raw4 = [[i64; 4]; 4];

/// The exact integer product `a * b` of raw matrices: entries in units of `2^-64`.
fn product(a: &Raw4, b: &Raw4) -> Vec<Vec<Int>> {
    (0..4)
        .map(|i| {
            (0..4)
                .map(|j| {
                    (0..4).fold(Int::new(0), |acc, k| {
                        acc.add(&Int::new(i128::from(a[i][k])).mul(&Int::new(i128::from(b[k][j]))))
                    })
                })
                .collect()
        })
        .collect()
}

fn det(m: &[Vec<Int>]) -> Int {
    if m.len() == 1 {
        return m[0][0].clone();
    }
    let mut total = Int::new(0);
    for (column, entry) in m[0].iter().enumerate() {
        let minor: Vec<Vec<Int>> = m[1..]
            .iter()
            .map(|row| {
                row.iter()
                    .enumerate()
                    .filter(|(j, _)| *j != column)
                    .map(|(_, v)| v.clone())
                    .collect()
            })
            .collect();
        let term = entry.mul(&det(&minor));
        total = if column % 2 == 0 { total.add(&term) } else { total.sub(&term) };
    }
    total
}

/// One unprojection problem, all raw.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Case {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) x: i64,
    pub(crate) y: i64,
    pub(crate) projection: Raw4,
    pub(crate) view: Raw4,
}

impl Case {
    /// The clip vector at NDC depth `depth` (0 or 1), scaled by `width height 2^32`.
    fn clip(&self, depth: i128) -> [Int; 4] {
        let (w, h) = (i128::from(self.width), i128::from(self.height));
        let ax = 2 * i128::from(self.x) - w * ONE;
        let ay = h * ONE - 2 * i128::from(self.y);
        [Int::new(ax * h), Int::new(ay * w), Int::new(depth * w * h * ONE), Int::new(w * h * ONE)]
    }

    /// The exact world point at NDC depth `depth`, in units, by Cramer's rule.
    pub(crate) fn world_point(&self, depth: i128) -> Option<[Ratio; 3]> {
        let m = product(&self.projection, &self.view);
        let clip = self.clip(depth);
        let replaced = |column: usize| {
            let mut copy = m.clone();
            for (row, value) in copy.iter_mut().zip(&clip) {
                row[column] = value.clone();
            }
            det(&copy)
        };
        let w = replaced(3);
        if w.sign() == Ordering::Equal {
            return None;
        }
        Some([0, 1, 2].map(|i| Ratio::new(replaced(i), w.clone())))
    }

    /// The exact ray: the depth-0 point and the depth-1 point minus it, in raw units.
    pub(crate) fn exact_ray(&self) -> Option<([Ratio; 3], [Ratio; 3])> {
        let scale = Ratio::int(ONE);
        let near = self.world_point(0)?;
        let far = self.world_point(1)?;
        let direction = [0, 1, 2].map(|i| far[i].sub(&near[i]).mul(&scale));
        Some((near.map(|c| c.mul(&scale)), direction))
    }
}

/// Each exact component rounded to nearest, ties to even; `None` outside `i64`.
pub(crate) fn rounded(v: &[Ratio; 3]) -> Option<[i64; 3]> {
    Some([v[0].round()?, v[1].round()?, v[2].round()?])
}
