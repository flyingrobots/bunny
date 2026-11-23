//! The exact oracle for `bunny-dielectric/1`: a signed integer of any size, and Snell's law,
//! the condition for total internal reflection and the unpolarized Fresnel reflectance written
//! with it from the chapter's formulas (`docs/topics/optics/README.md#smooth-dielectric`).
//!
//! Nothing here calls the code under test, and nothing is floating point. A square root is
//! the exact floor root of a value scaled up by `2^128`, so its error is below `2^-64` of a
//! unit of the scaled value; each use states the slack that covers it.

// Limbs are below 2^32 by construction, so the `as u32` conversions keep every bit.
#![allow(clippy::cast_possible_truncation)]

use std::cmp::Ordering;
use std::ops::{Add, Mul, Sub};

/// A signed integer of any size: a sign and little-endian base-`2^32` limbs with no trailing
/// zero limb. Zero is not negative.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Int {
    negative: bool,
    limbs: Vec<u32>,
}

impl Int {
    pub(crate) fn new(value: i128) -> Self {
        let magnitude = value.unsigned_abs();
        let limbs = (0..4).map(|k| (magnitude >> (32 * k)) as u32).collect();
        Self::from_parts(value < 0, limbs)
    }

    fn from_parts(negative: bool, mut limbs: Vec<u32>) -> Self {
        while limbs.last() == Some(&0) {
            limbs.pop();
        }
        Self { negative: negative && !limbs.is_empty(), limbs }
    }

    /// `2^shift`.
    pub(crate) fn power_of_two(shift: u32) -> Self {
        let mut limbs = vec![0; (shift / 32) as usize];
        limbs.push(1 << (shift % 32));
        Self::from_parts(false, limbs)
    }

    pub(crate) fn is_negative(&self) -> bool {
        self.negative
    }

    pub(crate) fn is_zero(&self) -> bool {
        self.limbs.is_empty()
    }

    pub(crate) fn abs(&self) -> Self {
        Self::from_parts(false, self.limbs.clone())
    }

    pub(crate) fn negated(&self) -> Self {
        Self::from_parts(!self.negative, self.limbs.clone())
    }

    /// The floor of the square root of a non-negative value: bits are set from the top
    /// whenever the square stays at most `self`.
    pub(crate) fn isqrt(&self) -> Self {
        assert!(!self.negative, "the square root of a negative value");
        let bits = u32::try_from(self.limbs.len()).expect("a small value") * 32;
        (0..=bits / 2).rev().fold(Self::new(0), |root, bit| {
            let candidate = &root + &Self::power_of_two(bit);
            if &candidate * &candidate <= *self {
                candidate
            } else {
                root
            }
        })
    }

    fn compare_magnitude(&self, other: &Self) -> Ordering {
        let by_length = self.limbs.len().cmp(&other.limbs.len());
        by_length.then_with(|| self.limbs.iter().rev().cmp(other.limbs.iter().rev()))
    }

    /// `|self| + |other|`.
    fn magnitude_sum(&self, other: &Self) -> Vec<u32> {
        let mut out = Vec::new();
        let mut carry = 0_u64;
        for k in 0..=self.limbs.len().max(other.limbs.len()) {
            let limb = |v: &Self| u64::from(v.limbs.get(k).copied().unwrap_or(0));
            let total = limb(self) + limb(other) + carry;
            out.push(total as u32);
            carry = total >> 32;
        }
        out
    }

    /// `|self| - |other|`, for `|other| <= |self|`.
    fn magnitude_difference(&self, other: &Self) -> Vec<u32> {
        let mut out = Vec::new();
        let mut borrow = 0_i64;
        for (k, &limb) in self.limbs.iter().enumerate() {
            let total = i64::from(limb) - i64::from(other.limbs.get(k).copied().unwrap_or(0));
            let total = total - borrow;
            borrow = i64::from(total < 0);
            out.push(u32::try_from(total + (borrow << 32)).expect("a limb"));
        }
        out
    }
}

impl Ord for Int {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.negative, other.negative) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (false, false) => self.compare_magnitude(other),
            (true, true) => other.compare_magnitude(self),
        }
    }
}

impl PartialOrd for Int {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Add for &Int {
    type Output = Int;

    fn add(self, other: &Int) -> Int {
        if self.negative == other.negative {
            return Int::from_parts(self.negative, self.magnitude_sum(other));
        }
        let (large, small) = match self.compare_magnitude(other) {
            Ordering::Less => (other, self),
            _ => (self, other),
        };
        Int::from_parts(large.negative, large.magnitude_difference(small))
    }
}

impl Sub for &Int {
    type Output = Int;

    #[allow(clippy::suspicious_arithmetic_impl)] // a - b is a + (-b)
    fn sub(self, other: &Int) -> Int {
        self + &other.negated()
    }
}

impl Mul for &Int {
    type Output = Int;

    fn mul(self, other: &Int) -> Int {
        let mut out = vec![0_u64; self.limbs.len() + other.limbs.len() + 1];
        for (i, &a) in self.limbs.iter().enumerate() {
            let mut carry = 0_u64;
            for (j, &b) in other.limbs.iter().enumerate() {
                let total = out[i + j] + u64::from(a) * u64::from(b) + carry;
                out[i + j] = total & 0xFFFF_FFFF;
                carry = total >> 32;
            }
            out[i + other.limbs.len()] += carry;
        }
        let limbs = out.into_iter().map(|limb| limb as u32).collect();
        Int::from_parts(self.negative != other.negative, limbs)
    }
}

pub(crate) type Wide3 = [i128; 3];
pub(crate) type Int3 = [Int; 3];

pub(crate) fn int3(v: Wide3) -> Int3 {
    v.map(Int::new)
}

pub(crate) fn dot(a: &Int3, b: &Int3) -> Int {
    &(&(&a[0] * &b[0]) + &(&a[1] * &b[1])) + &(&a[2] * &b[2])
}

pub(crate) fn cross(a: &Int3, b: &Int3) -> Int3 {
    let term = |i: usize, j: usize| &(&a[i] * &b[j]) - &(&a[j] * &b[i]);
    [term(1, 2), term(2, 0), term(0, 1)]
}

/// The extra scale of the oracle's direction, so that a floored root is exact to `2^-64`.
const SCALE_BITS: u32 = 64;

/// A ray direction `d` meeting a surface of normal `n`, from index `a` into index `b`: every
/// quantity of the chapter's formulas, with `d` scaled by `2^64`.
pub(crate) struct Refraction {
    a: Int,
    b: Int,
    d: Int3,
    n: Int3,
    /// `(d . d)(n . n)`.
    lengths: Int,
    /// `-(d . n)`.
    m: Int,
    /// `b^2 (d.d)(n.n) - a^2 |d x n|^2`: not positive exactly under total internal reflection.
    k: Int,
}

impl Refraction {
    pub(crate) fn new(direction: Wide3, normal: Wide3, indices: [i64; 2]) -> Self {
        let scale = Int::power_of_two(SCALE_BITS);
        let (d, n) = (int3(direction).map(|c| &c * &scale), int3(normal));
        let [a, b] = indices.map(|index| Int::new(i128::from(index)));
        let lengths = &dot(&d, &d) * &dot(&n, &n);
        let sine = cross(&d, &n);
        let refracts = &(&(&b * &b) * &lengths) - &(&(&a * &a) * &dot(&sine, &sine));
        Self { m: dot(&d, &n).negated(), a, b, d, n, lengths, k: refracts }
    }

    /// Whether Snell's law has no refracted ray: `(a / b)^2 sin^2 >= 1`, with
    /// `sin^2 = |d x n|^2 / ((d . d)(n . n))`.
    pub(crate) fn is_total(&self) -> bool {
        self.k <= Int::new(0)
    }

    /// Whether the direction arrives against the normal: `d . n < 0`.
    pub(crate) fn arrives(&self) -> bool {
        !self.m.is_negative() && !self.m.is_zero()
    }

    /// `(numerator, denominator)` of the unpolarized Fresnel reflectance
    /// `(r_s^2 + r_p^2) / 2`, `r_s = (a m - q) / (a m + q)`,
    /// `r_p = (b^2 m - a q) / (b^2 m + a q)`, with `q = floor(sqrt(K))`. For a direction that
    /// arrives and refracts, `a m >= 2^96`, so flooring `q` moves the value by less than
    /// `2^-90`.
    pub(crate) fn reflectance(&self) -> (Int, Int) {
        let q = self.k.isqrt();
        let (am, bm, aq) = (&self.a * &self.m, &(&self.b * &self.b) * &self.m, &self.a * &q);
        let square = |v: Int| &v * &v;
        let (s_top, s_bottom) = (square(&am - &q), square(&am + &q));
        let (p_top, p_bottom) = (square(&bm - &aq), square(&bm + &aq));
        let numerator = &(&s_top * &p_bottom) + &(&p_top * &s_bottom);
        (numerator, &(&s_bottom * &p_bottom) * &Int::new(2))
    }

    /// `T = a (d (n.n) - (d.n) n) - q n`, a vector along the refracted direction, with
    /// `q = floor(sqrt(K))`. `|T| = b (n.n) |d|` is at least `2^160`, and flooring `q` moves
    /// each component by less than `2^33`.
    pub(crate) fn refracted(&self) -> Int3 {
        let q = self.k.isqrt();
        let nn = dot(&self.n, &self.n);
        [0, 1, 2].map(|i| {
            let tangent = &(&self.d[i] * &nn) + &(&self.m * &self.n[i]);
            &(&self.a * &tangent) - &(&q * &self.n[i])
        })
    }

    /// The smallest integer `k >= 1` with `cos >= 1 / k` for the cosine of the angle in the
    /// rarer medium: the incidence angle when `a <= b`, else the refracted angle.
    pub(crate) fn inverse_rarer_cosine(&self) -> i128 {
        let (top, bottom) = if self.a <= self.b {
            (&self.m * &self.m, self.lengths.clone())
        } else {
            (self.k.clone(), &(&self.b * &self.b) * &self.lengths)
        };
        let (mut low, mut high) = (0_i128, 1_i128 << 70);
        while high - low > 1 {
            let mid = i128::midpoint(low, high);
            if &(&Int::new(mid) * &Int::new(mid)) * &top >= bottom {
                high = mid;
            } else {
                low = mid;
            }
        }
        high
    }
}

/// Whether `a * sqrt(q) <= t`, exactly.
fn at_most(a: &Int, t: &Int, q: &Int) -> bool {
    let (lhs, rhs) = (&(a * a) * q, t * t);
    if t.is_negative() {
        a.is_negative() && lhs >= rhs
    } else {
        a.is_negative() || lhs <= rhs
    }
}

/// Whether every raw component `r` of `raw` satisfies `|r |S| - s 2^32| <= bound |S| + slack`
/// for the nonzero exact vector `S = exact`: within `bound + slack / |S|` raw units of the
/// exact unit vector along it.
pub(crate) fn within(raw: [i64; 3], exact: &Int3, bound: i128, slack: &Int) -> bool {
    let length_squared = dot(exact, exact);
    let one = Int::power_of_two(32);
    raw.iter().zip(exact).all(|(component, s)| {
        let (r, scaled) = (i128::from(*component), s * &one);
        at_most(&Int::new(r - bound), &(&scaled + slack), &length_squared)
            && at_most(&Int::new(-(r + bound)), &(slack - &scaled), &length_squared)
    })
}
