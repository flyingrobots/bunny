//! Independent exact oracle for the geometric predicates.
//!
//! An unbounded signed integer in 32-bit digits, written for the tests: schoolbook addition,
//! subtraction, multiplication and comparison, and no division. It shares no helper with the
//! kernel, which holds a fixed sign and four 64-bit limbs. Rounded results are checked by
//! multiplying back, never by dividing.

#![allow(dead_code)]

use core::cmp::Ordering;

use bunny_geom::ExactDeterminant;
use bunny_linalg::{FixedVec2, FixedVec3};

/// A sign and a magnitude in little-endian base-2^32 digits with no trailing zero digit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Big {
    negative: bool,
    digits: Vec<u32>,
}

impl Big {
    pub fn zero() -> Self {
        Self { negative: false, digits: Vec::new() }
    }

    pub fn from_i128(value: i128) -> Self {
        let mut magnitude = value.unsigned_abs();
        let mut digits = Vec::new();
        while magnitude != 0 {
            digits.push(u32::try_from(magnitude & 0xFFFF_FFFF).expect("one digit"));
            magnitude >>= 32;
        }
        Self::signed(value < 0, digits)
    }

    pub fn from_i64(value: i64) -> Self {
        Self::from_i128(i128::from(value))
    }

    fn signed(negative: bool, mut digits: Vec<u32>) -> Self {
        while digits.last() == Some(&0) {
            digits.pop();
        }
        let negative = negative && !digits.is_empty();
        Self { negative, digits }
    }

    pub fn is_zero(&self) -> bool {
        self.digits.is_empty()
    }

    pub fn signum(&self) -> i32 {
        if self.is_zero() {
            0
        } else if self.negative {
            -1
        } else {
            1
        }
    }

    pub fn neg(&self) -> Self {
        Self::signed(!self.negative, self.digits.clone())
    }

    pub fn abs(&self) -> Self {
        Self::signed(false, self.digits.clone())
    }

    pub fn add(&self, rhs: &Self) -> Self {
        if self.negative == rhs.negative {
            return Self::signed(self.negative, add_digits(&self.digits, &rhs.digits));
        }
        match cmp_digits(&self.digits, &rhs.digits) {
            Ordering::Less => Self::signed(rhs.negative, sub_digits(&rhs.digits, &self.digits)),
            _ => Self::signed(self.negative, sub_digits(&self.digits, &rhs.digits)),
        }
    }

    pub fn sub(&self, rhs: &Self) -> Self {
        self.add(&rhs.neg())
    }

    pub fn mul(&self, rhs: &Self) -> Self {
        let mut out = vec![0_u64; self.digits.len() + rhs.digits.len() + 1];
        for (i, &x) in self.digits.iter().enumerate() {
            let mut carry = 0_u64;
            for (j, &y) in rhs.digits.iter().enumerate() {
                let t = out[i + j] + u64::from(x) * u64::from(y) + carry;
                out[i + j] = t & 0xFFFF_FFFF;
                carry = t >> 32;
            }
            out[i + rhs.digits.len()] += carry;
        }
        let digits = out.into_iter().map(|d| u32::try_from(d).expect("one digit")).collect();
        Self::signed(self.negative != rhs.negative, digits)
    }

    /// `self * 2^k`.
    pub fn shl(&self, k: u32) -> Self {
        let mut factor = Self::from_i128(1);
        for _ in 0..k {
            factor = factor.add(&factor);
        }
        self.mul(&factor)
    }

    pub fn cmp(&self, rhs: &Self) -> Ordering {
        match (self.negative, rhs.negative) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (false, false) => cmp_digits(&self.digits, &rhs.digits),
            (true, true) => cmp_digits(&rhs.digits, &self.digits),
        }
    }

    /// The magnitude as four little-endian 64-bit limbs, or `None` above `2^256`.
    pub fn limbs(&self) -> Option<[u64; 4]> {
        if self.digits.len() > 8 {
            return None;
        }
        let mut limbs = [0_u64; 4];
        for (k, &digit) in self.digits.iter().enumerate() {
            limbs[k / 2] |= u64::from(digit) << (32 * (k % 2));
        }
        Some(limbs)
    }

    pub fn is_even(&self) -> bool {
        self.digits.first().is_none_or(|d| d % 2 == 0)
    }
}

fn add_digits(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut out = Vec::new();
    let mut carry = 0_u64;
    for k in 0..a.len().max(b.len()) {
        let t = u64::from(*a.get(k).unwrap_or(&0)) + u64::from(*b.get(k).unwrap_or(&0)) + carry;
        out.push(u32::try_from(t & 0xFFFF_FFFF).expect("one digit"));
        carry = t >> 32;
    }
    out.push(u32::try_from(carry).expect("one digit"));
    out
}

/// `a - b` for `b <= a`.
fn sub_digits(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut out = Vec::new();
    let mut borrow = 0_i64;
    for (k, &x) in a.iter().enumerate() {
        let mut t = i64::from(x) - i64::from(*b.get(k).unwrap_or(&0)) - borrow;
        borrow = 0;
        if t < 0 {
            t += 1_i64 << 32;
            borrow = 1;
        }
        out.push(u32::try_from(t).expect("one digit"));
    }
    out
}

fn cmp_digits(a: &[u32], b: &[u32]) -> Ordering {
    a.len().cmp(&b.len()).then_with(|| a.iter().rev().cmp(b.iter().rev()))
}

fn d(a: i64, b: i64) -> Big {
    Big::from_i64(a).sub(&Big::from_i64(b))
}

/// `(b - a) x (c - a)` on raw values.
pub fn det2(a: FixedVec2, b: FixedVec2, c: FixedVec2) -> Big {
    let (ex, ey) = (d(b.x.raw(), a.x.raw()), d(b.y.raw(), a.y.raw()));
    let (fx, fy) = (d(c.x.raw(), a.x.raw()), d(c.y.raw(), a.y.raw()));
    ex.mul(&fy).sub(&ey.mul(&fx))
}

/// `(d - a) . ((b - a) x (c - a))` on raw values.
pub fn det3(a: FixedVec3, b: FixedVec3, c: FixedVec3, p: FixedVec3) -> Big {
    let e = [d(b.x.raw(), a.x.raw()), d(b.y.raw(), a.y.raw()), d(b.z.raw(), a.z.raw())];
    let f = [d(c.x.raw(), a.x.raw()), d(c.y.raw(), a.y.raw()), d(c.z.raw(), a.z.raw())];
    let g = [d(p.x.raw(), a.x.raw()), d(p.y.raw(), a.y.raw()), d(p.z.raw(), a.z.raw())];
    let nx = e[1].mul(&f[2]).sub(&e[2].mul(&f[1]));
    let ny = e[2].mul(&f[0]).sub(&e[0].mul(&f[2]));
    let nz = e[0].mul(&f[1]).sub(&e[1].mul(&f[0]));
    g[0].mul(&nx).add(&g[1].mul(&ny)).add(&g[2].mul(&nz))
}

/// Whether the kernel's exact value equals the oracle's, sign and magnitude.
pub fn same(kernel: ExactDeterminant, oracle: &Big) -> bool {
    let sign = match kernel.sign() {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    };
    sign == oracle.signum() && Some(kernel.magnitude()) == oracle.limbs()
}

/// Whether `result` is `round_half_even(num / den)` as an `i64`, or `None` exactly when that
/// quotient is outside `i64`. `den` must be nonzero.
pub fn rounds_to(num: &Big, den: &Big, result: Option<i64>) -> bool {
    let (num, den) = if den.negative { (num.neg(), den.neg()) } else { (num.clone(), den.clone()) };
    let twice = num.add(&num);
    let above = twice.cmp(&den.mul(&Big::from_i128(2 * i128::from(i64::MAX) + 1))).is_ge();
    let below = twice.cmp(&den.mul(&Big::from_i128(2 * i128::from(i64::MIN) - 1))).is_lt();
    match result {
        None => above || below,
        Some(r) => {
            let r = Big::from_i64(r);
            let error = num.sub(&r.mul(&den)).abs();
            match error.add(&error).cmp(&den) {
                Ordering::Less => true,
                Ordering::Equal => r.is_even(),
                Ordering::Greater => false,
            }
        }
    }
}
