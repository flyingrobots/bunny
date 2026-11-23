//! An exact rational oracle for the closest point of a triangle, written apart from the kernel.
//!
//! The kernel decides the face region from the signs of triple products and keeps every value
//! over one denominator in fixed-width limbs. This oracle uses none of that: arbitrary-size
//! sign-magnitude integers, rationals reduced by nothing, the face weights solved from the
//! 2x2 Gram system by Cramer's rule, and every squared distance summed from the rational
//! point. It shares no helper with `bunny-query`.

#![allow(dead_code)]
// The oracle follows the textbook names of points and parameters.
#![allow(clippy::many_single_char_names)]

use std::cmp::Ordering;

/// An integer of any size: sign and little-endian base-`2^32` magnitude, no trailing zero
/// limb; zero is never negative.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Int {
    negative: bool,
    magnitude: Vec<u32>,
}

impl Int {
    pub(crate) fn new(value: i128) -> Self {
        let mut rest = value.unsigned_abs();
        let mut magnitude = Vec::new();
        while rest != 0 {
            magnitude.push(u32::try_from(rest & 0xFFFF_FFFF).expect("one limb"));
            rest >>= 32;
        }
        Self { negative: value < 0, magnitude }
    }

    fn from_parts(negative: bool, mut magnitude: Vec<u32>) -> Self {
        while magnitude.last() == Some(&0) {
            magnitude.pop();
        }
        let negative = negative && !magnitude.is_empty();
        Self { negative, magnitude }
    }

    pub(crate) fn is_negative(&self) -> bool {
        self.negative
    }

    pub(crate) fn is_zero(&self) -> bool {
        self.magnitude.is_empty()
    }

    pub(crate) fn neg(&self) -> Self {
        Self::from_parts(!self.negative, self.magnitude.clone())
    }

    pub(crate) fn add(&self, other: &Self) -> Self {
        if self.negative == other.negative {
            return Self::from_parts(
                self.negative,
                add_magnitudes(&self.magnitude, &other.magnitude),
            );
        }
        match cmp_magnitudes(&self.magnitude, &other.magnitude) {
            Ordering::Less => {
                Self::from_parts(other.negative, sub_magnitudes(&other.magnitude, &self.magnitude))
            }
            _ => Self::from_parts(self.negative, sub_magnitudes(&self.magnitude, &other.magnitude)),
        }
    }

    pub(crate) fn sub(&self, other: &Self) -> Self {
        self.add(&other.neg())
    }

    pub(crate) fn mul(&self, other: &Self) -> Self {
        let mut out = vec![0_u64; self.magnitude.len() + other.magnitude.len() + 1];
        for (i, &a) in self.magnitude.iter().enumerate() {
            let mut carry = 0_u64;
            for (j, &b) in other.magnitude.iter().enumerate() {
                let t = out[i + j] + u64::from(a) * u64::from(b) + carry;
                out[i + j] = t & 0xFFFF_FFFF;
                carry = t >> 32;
            }
            out[i + other.magnitude.len()] += carry;
        }
        let limbs = out.into_iter().map(|limb| u32::try_from(limb).expect("one limb")).collect();
        Self::from_parts(self.negative != other.negative, limbs)
    }

    pub(crate) fn cmp(&self, other: &Self) -> Ordering {
        match (self.negative, other.negative) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (false, false) => cmp_magnitudes(&self.magnitude, &other.magnitude),
            (true, true) => cmp_magnitudes(&other.magnitude, &self.magnitude),
        }
    }
}

fn add_magnitudes(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut out = Vec::new();
    let mut carry = 0_u64;
    for k in 0..a.len().max(b.len()) {
        let t = u64::from(*a.get(k).unwrap_or(&0)) + u64::from(*b.get(k).unwrap_or(&0)) + carry;
        out.push(u32::try_from(t & 0xFFFF_FFFF).expect("one limb"));
        carry = t >> 32;
    }
    out.push(u32::try_from(carry).expect("one limb"));
    out
}

/// `a - b` for `b <= a`.
fn sub_magnitudes(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut out = Vec::new();
    let mut borrow = 0_i64;
    for (k, &limb) in a.iter().enumerate() {
        let mut t = i64::from(limb) - i64::from(*b.get(k).unwrap_or(&0)) - borrow;
        borrow = i64::from(t < 0);
        t += borrow << 32;
        out.push(u32::try_from(t).expect("one limb"));
    }
    assert_eq!(borrow, 0, "subtrahend larger than minuend");
    out
}

fn cmp_magnitudes(a: &[u32], b: &[u32]) -> Ordering {
    let trimmed = |m: &[u32]| m.len() - m.iter().rev().take_while(|&&l| l == 0).count();
    let (la, lb) = (trimmed(a), trimmed(b));
    la.cmp(&lb).then_with(|| a[..la].iter().rev().cmp(b[..lb].iter().rev()))
}

/// A rational `num / den` with `den > 0`, not reduced.
#[derive(Clone, Debug)]
pub(crate) struct Ratio {
    pub(crate) num: Int,
    pub(crate) den: Int,
}

impl Ratio {
    pub(crate) fn int(value: i128) -> Self {
        Self { num: Int::new(value), den: Int::new(1) }
    }

    pub(crate) fn new(num: Int, den: Int) -> Self {
        assert!(!den.is_zero(), "zero denominator");
        if den.is_negative() {
            Self { num: num.neg(), den: den.neg() }
        } else {
            Self { num, den }
        }
    }

    pub(crate) fn add(&self, o: &Self) -> Self {
        Self::new(self.num.mul(&o.den).add(&o.num.mul(&self.den)), self.den.mul(&o.den))
    }

    pub(crate) fn sub(&self, o: &Self) -> Self {
        Self::new(self.num.mul(&o.den).sub(&o.num.mul(&self.den)), self.den.mul(&o.den))
    }

    pub(crate) fn mul(&self, o: &Self) -> Self {
        Self::new(self.num.mul(&o.num), self.den.mul(&o.den))
    }

    pub(crate) fn cmp(&self, o: &Self) -> Ordering {
        self.num.mul(&o.den).cmp(&o.num.mul(&self.den))
    }

    pub(crate) fn is_negative(&self) -> bool {
        self.num.is_negative()
    }
}

pub(crate) type Point = [Ratio; 3];

fn ratio_point(raw: [i64; 3]) -> Point {
    raw.map(|c| Ratio::int(i128::from(c)))
}

fn minus(a: &Point, b: &Point) -> Point {
    [a[0].sub(&b[0]), a[1].sub(&b[1]), a[2].sub(&b[2])]
}

fn dot(a: &Point, b: &Point) -> Ratio {
    a[0].mul(&b[0]).add(&a[1].mul(&b[1])).add(&a[2].mul(&b[2]))
}

fn along(start: &Point, direction: &Point, t: &Ratio) -> Point {
    [0, 1, 2].map(|k| start[k].add(&direction[k].mul(t)))
}

/// The exact closest point of the closed triangle to `p` and its squared distance in raw
/// units squared.
pub(crate) fn closest(p: [i64; 3], triangle: [[i64; 3]; 3]) -> (Point, Ratio) {
    let p = ratio_point(p);
    let [a, b, c] = triangle.map(ratio_point);
    let mut candidates = vec![a.clone(), b.clone(), c.clone()];
    for (u, w) in [(&a, &b), (&b, &c), (&c, &a)] {
        candidates.extend(on_segment(&p, u, w));
    }
    candidates.extend(on_face(&p, &a, &b, &c));
    let distance = |q: &Point| {
        let d = minus(&p, q);
        dot(&d, &d)
    };
    let mut best = candidates.swap_remove(0);
    let mut best_distance = distance(&best);
    for q in candidates {
        let d = distance(&q);
        if d.cmp(&best_distance) == Ordering::Less {
            best = q;
            best_distance = d;
        }
    }
    (best, best_distance)
}

/// The foot of the perpendicular from `p` to the segment, when it falls inside it.
fn on_segment(p: &Point, u: &Point, w: &Point) -> Option<Point> {
    let e = minus(w, u);
    let length = dot(&e, &e);
    if length.num.is_zero() {
        return None;
    }
    let t = dot(&e, &minus(p, u));
    let t = Ratio::new(t.num.mul(&length.den), t.den.mul(&length.num));
    let inside = !t.is_negative() && t.cmp(&Ratio::int(1)) != Ordering::Greater;
    inside.then(|| along(u, &e, &t))
}

/// The projection onto the plane, by the Gram system
/// `[e1.e1 e1.e2; e1.e2 e2.e2] [v; w] = [e1.s; e2.s]`, when it lies on the triangle.
fn on_face(p: &Point, a: &Point, b: &Point, c: &Point) -> Option<Point> {
    let (e1, e2, s) = (minus(b, a), minus(c, a), minus(p, a));
    let (g11, g12, g22) = (dot(&e1, &e1), dot(&e1, &e2), dot(&e2, &e2));
    let (r1, r2) = (dot(&e1, &s), dot(&e2, &s));
    let det = g11.mul(&g22).sub(&g12.mul(&g12));
    if det.num.is_zero() {
        return None;
    }
    let solve = |n: Ratio| Ratio::new(n.num.mul(&det.den), n.den.mul(&det.num));
    let v = solve(r1.mul(&g22).sub(&r2.mul(&g12)));
    let w = solve(g11.mul(&r2).sub(&g12.mul(&r1)));
    let sum = v.add(&w);
    let inside =
        !v.is_negative() && !w.is_negative() && sum.cmp(&Ratio::int(1)) != Ordering::Greater;
    let point = along(a, &e1, &v);
    inside.then(|| along(&point, &e2, &w))
}

/// Whether `reported` is `exact` rounded to the nearest integer, ties to even.
pub(crate) fn rounds_to(exact: &Ratio, reported: i64) -> bool {
    let error = exact.sub(&Ratio::int(i128::from(reported)));
    let twice = error.mul(&Ratio::int(2));
    let magnitude = if twice.is_negative() { twice.mul(&Ratio::int(-1)) } else { twice };
    match magnitude.cmp(&Ratio::int(1)) {
        Ordering::Less => true,
        Ordering::Equal => reported % 2 == 0,
        Ordering::Greater => false,
    }
}

/// `value / 2^32`.
pub(crate) fn over_two_to_32(value: &Ratio) -> Ratio {
    Ratio::new(value.num.clone(), value.den.mul(&Int::new(1 << 32)))
}
