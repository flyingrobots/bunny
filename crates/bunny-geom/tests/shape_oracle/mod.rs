//! Independent exact oracles for the shape-library tests.
//!
//! `Wide` is a 256-bit two's-complement integer on eight 32-bit limbs, with products formed
//! limb by limb in `u64`. It shares no code with `bunny-geom`, whose degeneracy test compares
//! sign-and-magnitude `u128` products and whose axis check orders lengths before dot products.
//! Every quantity here is below `2^132` in magnitude, so arithmetic modulo `2^256` is exact.

#![allow(dead_code)]

use core::cmp::Ordering;

/// A 256-bit two's-complement integer, least significant limb first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Wide([u32; 8]);

const LOW: u64 = 0xFFFF_FFFF;

fn low_limb(value: u64) -> u32 {
    u32::try_from(value & LOW).expect("masked to 32 bits")
}

impl Wide {
    pub(crate) fn from_i64(value: i64) -> Self {
        let fill = if value < 0 { u32::MAX } else { 0 };
        let bits = value.cast_unsigned();
        let mut limbs = [fill; 8];
        limbs[0] = low_limb(bits);
        limbs[1] = low_limb(bits >> 32);
        Self(limbs)
    }

    pub(crate) fn add(self, other: Self) -> Self {
        let mut limbs = [0; 8];
        let mut carry = 0_u64;
        for (out, (a, b)) in limbs.iter_mut().zip(self.0.iter().zip(other.0)) {
            let sum = u64::from(*a) + u64::from(b) + carry;
            *out = low_limb(sum);
            carry = sum >> 32;
        }
        Self(limbs)
    }

    pub(crate) fn neg(self) -> Self {
        Self(self.0.map(|limb| !limb)).add(Self::from_i64(1))
    }

    pub(crate) fn sub(self, other: Self) -> Self {
        self.add(other.neg())
    }

    /// The product modulo `2^256`, which is the exact product when it is below `2^255`.
    pub(crate) fn mul(self, other: Self) -> Self {
        let mut limbs = [0_u32; 8];
        for (i, &a) in self.0.iter().enumerate() {
            let mut carry = 0_u64;
            for (j, &b) in other.0.iter().take(8 - i).enumerate() {
                let slot = &mut limbs[i + j];
                let t = u64::from(a) * u64::from(b) + u64::from(*slot) + carry;
                *slot = low_limb(t);
                carry = t >> 32;
            }
        }
        Self(limbs)
    }

    pub(crate) fn is_zero(self) -> bool {
        self.0.iter().all(|&limb| limb == 0)
    }

    pub(crate) fn is_negative(self) -> bool {
        self.0[7] >> 31 == 1
    }

    pub(crate) fn abs(self) -> Self {
        if self.is_negative() {
            self.neg()
        } else {
            self
        }
    }

    /// Signed comparison.
    pub(crate) fn compare(self, other: Self) -> Ordering {
        match (self.is_negative(), other.is_negative()) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => self.0.iter().rev().cmp(other.0.iter().rev()),
        }
    }
}

fn wide3(v: [i64; 3]) -> [Wide; 3] {
    v.map(Wide::from_i64)
}

/// The exact cross product `(b - a) x (c - a)` of raw vertices.
pub(crate) fn cross_of_edges(v0: [i64; 3], v1: [i64; 3], v2: [i64; 3]) -> [Wide; 3] {
    let (origin, first, second) = (wide3(v0), wide3(v1), wide3(v2));
    let u: [Wide; 3] = core::array::from_fn(|i| first[i].sub(origin[i]));
    let w: [Wide; 3] = core::array::from_fn(|i| second[i].sub(origin[i]));
    [
        u[1].mul(w[2]).sub(u[2].mul(w[1])),
        u[2].mul(w[0]).sub(u[0].mul(w[2])),
        u[0].mul(w[1]).sub(u[1].mul(w[0])),
    ]
}

/// Whether the triangle's raw vertices are collinear or coincident, exactly.
pub(crate) fn triangle_is_degenerate(a: [i64; 3], b: [i64; 3], c: [i64; 3]) -> bool {
    cross_of_edges(a, b, c).iter().all(|component| component.is_zero())
}

/// The exact raw dot product (a value with 64 fractional bits).
pub(crate) fn dot(a: [i64; 3], b: [i64; 3]) -> Wide {
    let (a, b) = (wide3(a), wide3(b));
    a[0].mul(b[0]).add(a[1].mul(b[1])).add(a[2].mul(b[2]))
}

/// What `FixedObb3::try_new` should report for these raw columns: `Ok(())`, `Err(Length(i))`
/// for the first column whose exact length is more than `length_tolerance` raw units from one,
/// or `Err(Dot(i, j))` for the first pair whose exact dot product is more than
/// `dot_tolerance` raw units from zero. Lengths are compared on exact squares.
pub(crate) fn obb_axes_verdict(
    columns: [[i64; 3]; 3],
    length_tolerance: i64,
    dot_tolerance: i64,
) -> Result<(), AxisFault> {
    let one = 1_i64 << 32;
    let low = Wide::from_i64(one - length_tolerance);
    let high = Wide::from_i64(one + length_tolerance);
    let (low, high) = (low.mul(low), high.mul(high));
    for (i, &column) in columns.iter().enumerate() {
        let square = dot(column, column);
        if square.compare(low) == Ordering::Less || square.compare(high) == Ordering::Greater {
            return Err(AxisFault::Length(i));
        }
    }
    let limit = Wide::from_i64(dot_tolerance).mul(Wide::from_i64(one));
    for (i, j) in [(0, 1), (0, 2), (1, 2)] {
        if dot(columns[i], columns[j]).abs().compare(limit) == Ordering::Greater {
            return Err(AxisFault::Dot(i, j));
        }
    }
    Ok(())
}

/// Which axis rule the oracle says failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AxisFault {
    Length(usize),
    Dot(usize, usize),
}

/// A 64-bit PCG-style generator; the seed is printed in every failure message.
pub(crate) struct Rng(pub(crate) u64);

impl Rng {
    pub(crate) fn next_u64(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        let x = self.0;
        (x ^ (x >> 29)).wrapping_mul(0xBF58_476D_1CE4_E5B9) ^ (x >> 32)
    }

    pub(crate) fn next_i64(&mut self) -> i64 {
        self.next_u64().cast_signed()
    }

    /// Uniform-ish in `[-m, m]` for `m < 2^62`.
    pub(crate) fn within(&mut self, m: i64) -> i64 {
        let span = u64::try_from(2 * m + 1).expect("m is non-negative and below 2^62");
        i64::try_from(self.next_u64() % span).expect("below 2^63") - m
    }
}
