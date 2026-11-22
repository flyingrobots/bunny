//! An independent oracle for `TraceRay::hit_triangle`, written from the contract in
//! `docs/topics/ray-tracing/README.md#exact-triangle-test`, not from the kernel.
//!
//! Why it is independent: the kernel (and its frozen copy in `trace_reference.rs`) follows
//! Moller-Trumbore in `i128`: `p = d x e2`, `det = e1 . p`, `u = s . p`, `q = s x e1`,
//! `v = d . q`, `t = e2 . q`. This oracle computes none of those quantities and calls no
//! kernel code. It decides the crossing from three signed volumes, one per edge `(a, b)`:
//! the volume of the tetrahedron `(origin, origin + d, a, b)`, `det[a - o, b - o, d]`. The
//! ray's line crosses the triangle, edges and corners included, exactly when the three
//! volumes are all `>= 0` or all `<= 0` and their sum `S` is not zero (`S = 0` means the
//! line is parallel to the plane or the triangle has no area). The distance comes from the
//! plane equation `n . (o + t d - v0) = 0` with `n = (v1 - v0) x (v2 - v0)`, floored by a
//! bit-by-bit long division. Everything is evaluated in a 256-bit integer written here.
//!
//! Expanding the triple products gives the kernel's outputs from these: with
//! `sigma = sign(S)`, `u = sigma * vol(v2, v0)`, `v = sigma * vol(v0, v1)`, `det = |S|`, and
//! `front_face` is `S < 0` (because `S = d . n`). The tests compare those exactly.

use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use bunny_query::{TraceRay, TraceTriangle};

/// A 256-bit two's-complement integer in little-endian 64-bit limbs. The widest value here,
/// `n . (v0 - o)` times `2^32`, is below `2^160`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Wide([u64; 4]);

impl Wide {
    const ZERO: Self = Self([0; 4]);

    fn from_i64(value: i64) -> Self {
        let fill = if value < 0 { u64::MAX } else { 0 };
        Self([value.cast_unsigned(), fill, fill, fill])
    }

    fn add(self, rhs: Self) -> Self {
        let mut limbs = [0; 4];
        let mut carry = 0;
        for (i, limb) in limbs.iter_mut().enumerate() {
            let sum = u128::from(self.0[i]) + u128::from(rhs.0[i]) + carry;
            *limb = low_limb(sum);
            carry = sum >> 64;
        }
        Self(limbs)
    }

    fn neg(self) -> Self {
        Self(self.0.map(|limb| !limb)).add(Self::from_i64(1))
    }

    fn sub(self, rhs: Self) -> Self {
        self.add(rhs.neg())
    }

    /// The product modulo `2^256`, which is exact for every product formed here.
    fn mul(self, rhs: Self) -> Self {
        let mut limbs = [0; 4];
        for i in 0..4 {
            let mut carry = 0;
            for j in 0..4 - i {
                let sum =
                    u128::from(limbs[i + j]) + u128::from(self.0[i]) * u128::from(rhs.0[j]) + carry;
                limbs[i + j] = low_limb(sum);
                carry = sum >> 64;
            }
        }
        Self(limbs)
    }

    fn is_negative(self) -> bool {
        self.0[3] >> 63 == 1
    }

    /// `-1`, `0` or `1`.
    fn signum(self) -> i128 {
        if self.is_negative() {
            -1
        } else {
            i128::from(self != Self::ZERO)
        }
    }

    fn bit(self, index: usize) -> u64 {
        (self.0[index / 64] >> (index % 64)) & 1
    }

    fn shifted_left_one(self, low_bit: u64) -> Self {
        let mut limbs = [0; 4];
        let mut carry = low_bit;
        for (i, limb) in limbs.iter_mut().enumerate() {
            *limb = (self.0[i] << 1) | carry;
            carry = self.0[i] >> 63;
        }
        Self(limbs)
    }

    fn to_i128(self) -> i128 {
        let low = (u128::from(self.0[1]) << 64) | u128::from(self.0[0]);
        let value = i128::from_ne_bytes(low.to_ne_bytes());
        assert_eq!(Self::from_i128(value), self, "the value fits i128");
        value
    }

    fn from_i128(value: i128) -> Self {
        let high = Self::from_i64(i64::try_from(value >> 64).expect("an i128 shifted by 64"));
        let low = value.cast_unsigned() & u128::from(u64::MAX);
        Self([low_limb(low), high.0[0], high.0[1], high.0[2]])
    }
}

/// The low 64 bits.
fn low_limb(value: u128) -> u64 {
    u64::try_from(value & u128::from(u64::MAX)).expect("masked to 64 bits")
}

/// `floor(numerator / denominator)` for `numerator >= 0` and `denominator > 0`, one bit at a
/// time (restoring long division).
fn floor_div(numerator: Wide, denominator: Wide) -> Wide {
    let (mut quotient, mut remainder) = (Wide::ZERO, Wide::ZERO);
    for index in (0..256).rev() {
        remainder = remainder.shifted_left_one(numerator.bit(index));
        let reduced = remainder.sub(denominator);
        let fits = !reduced.is_negative();
        quotient = quotient.shifted_left_one(u64::from(fits));
        if fits {
            remainder = reduced;
        }
    }
    quotient
}

type Point = [Wide; 3];

fn point(raw: [i64; 3]) -> Point {
    raw.map(Wide::from_i64)
}

fn minus(a: Point, b: Point) -> Point {
    [a[0].sub(b[0]), a[1].sub(b[1]), a[2].sub(b[2])]
}

fn cross(a: Point, b: Point) -> Point {
    let term = |i: usize, j: usize| a[i].mul(b[j]).sub(a[j].mul(b[i]));
    [term(1, 2), term(2, 0), term(0, 1)]
}

fn dot(a: Point, b: Point) -> Wide {
    a[0].mul(b[0]).add(a[1].mul(b[1])).add(a[2].mul(b[2]))
}

/// What `hit_triangle` must report for a hit with limits `i64::MIN..=i64::MAX`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Expected {
    pub(crate) distance: i64,
    pub(crate) front_face: bool,
    pub(crate) u: i128,
    pub(crate) v: i128,
    pub(crate) det: i128,
}

/// The oracle's verdict on one ray and triangle.
pub(crate) struct Verdict {
    /// `u`, `v` and `det - u - v` as the kernel orients them (all `>= 0` inside the
    /// triangle), or `None` when `S = 0`.
    pub(crate) weights: Option<[i128; 3]>,
    /// Whether the plane is at a distance `>= 0` along the ray (meaningless when `S = 0`).
    pub(crate) ahead: bool,
    pub(crate) hit: Option<Expected>,
}

/// The oracle for a ray `origin + t * direction` and a triangle, all raw Q32.32.
pub(crate) fn verdict(origin: [i64; 3], direction: [i64; 3], vertices: [[i64; 3]; 3]) -> Verdict {
    let (start, along) = (point(origin), point(direction));
    let [to0, to1, to2] = vertices.map(|vertex| minus(point(vertex), start));
    let volume = |p: Point, q: Point| dot(cross(p, q), along);
    let (s01, s12, s20) = (volume(to0, to1), volume(to1, to2), volume(to2, to0));
    let sum = s01.add(s12).add(s20);
    let sigma = sum.signum();
    if sigma == 0 {
        return Verdict { weights: None, ahead: false, hit: None };
    }
    let weights = [s20, s01, s12].map(|s| s.to_i128() * sigma);
    let [v0, v1, v2] = vertices.map(point);
    let normal = cross(minus(v1, v0), minus(v2, v0));
    let (mut numerator, mut denominator) = (dot(normal, minus(v0, start)), dot(normal, along));
    assert_eq!(denominator, sum, "the volumes sum to d . n");
    if denominator.is_negative() {
        (numerator, denominator) = (numerator.neg(), denominator.neg());
    }
    let ahead = !numerator.is_negative();
    let inside = weights.iter().all(|&w| w >= 0);
    let distance = (inside && ahead)
        .then(|| floor_div(numerator.mul(Wide::from_i64(1 << 32)), denominator).to_i128())
        .and_then(|raw| i64::try_from(raw).ok());
    let hit = distance.map(|distance| Expected {
        distance,
        front_face: sigma < 0,
        u: weights[0],
        v: weights[1],
        det: sum.to_i128() * sigma,
    });
    Verdict { weights: Some(weights), ahead, hit }
}

fn vec3(raw: [i64; 3]) -> FixedVec3 {
    let [x, y, z] = raw.map(FixedQ32_32::from_raw);
    FixedVec3::new(x, y, z)
}

/// Runs `hit_triangle` with limits `i64::MIN..=i64::MAX` and asserts that it agrees with the
/// oracle on hit or miss, distance, facing and the exact numerators. Returns the verdict.
pub(crate) fn assert_kernel_agrees(
    origin: [i64; 3],
    direction: [i64; 3],
    vertices: [[i64; 3]; 3],
    context: &str,
) -> Verdict {
    let ray = TraceRay::new(vec3(origin), vec3(direction)).expect("ray in the trace domain");
    let triangle = TraceTriangle::new(vec3(vertices[0]), vec3(vertices[1]), vec3(vertices[2]))
        .expect("triangle in the trace domain");
    let (low, high) = (FixedQ32_32::from_raw(i64::MIN), FixedQ32_32::from_raw(i64::MAX));
    let got = ray.hit_triangle(&triangle, low, high).map(|hit| {
        let numerators = hit.barycentric_numerators();
        Expected {
            distance: hit.distance.raw(),
            front_face: hit.front_face,
            u: numerators.u(),
            v: numerators.v(),
            det: numerators.det(),
        }
    });
    let verdict = verdict(origin, direction, vertices);
    assert_eq!(
        got, verdict.hit,
        "{context}: origin={origin:?} direction={direction:?} vertices={vertices:?}"
    );
    verdict
}
