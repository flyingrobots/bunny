//! Evidence for `bunny-vertex-normals/1`: hand-derived meshes, vertices without a normal,
//! refusals, an exact big-integer oracle over seeded meshes, independence from face order and
//! vertex labels, and golden vectors.
//!
//! The oracle shares no code with the crate: it computes each face normal in the shoelace form
//! `a x b + b x c + c x a`, collects faces per vertex by search, and checks the bound with exact
//! products of arbitrary size (`Big`), never a square root.

// Indices, limbs and seeded draws are converted between integer widths with `as`; every value
// is in range by construction (a vertex index below 13, a limb below 2^32, a draw reduced first).
#![allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap, clippy::cast_sign_loss)]

use std::cmp::Ordering;

use bunny_linalg::FixedVec3;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_optics::{VertexNormalError, VertexNormals, MAX_NORMAL_TRIANGLES, VERTEX_NORMALS_ID};
use bunny_query::TRACE_COORD_LIMIT_RAW;
use wasm_bindgen_test::wasm_bindgen_test;

const ONE: i64 = ONE_RAW;
/// `round(0.6 * 2^32)`: the exact value is 2 576 980 377.6.
const SIX_TENTHS: i64 = 2_576_980_378;
/// `round(0.8 * 2^32)`: the exact value is 3 435 973 836.8.
const EIGHT_TENTHS: i64 = 3_435_973_837;
const SEED: u64 = 0xB002_0289_0000_0001;
/// The stated bound: each component within one raw unit of the exact unit vector.
const BOUND_RAW: i128 = 1;

type Raw3 = [i64; 3];
type Mesh = (Vec<Raw3>, Vec<[u32; 3]>);

fn vec3(raw: Raw3) -> FixedVec3 {
    let [x, y, z] = raw.map(FixedQ32_32::from_raw);
    FixedVec3::new(x, y, z)
}

fn units(whole: [i64; 3]) -> Raw3 {
    whole.map(|c| c * ONE)
}

fn build(positions: &[Raw3], triangles: &[[u32; 3]]) -> Result<VertexNormals, VertexNormalError> {
    let positions: Vec<FixedVec3> = positions.iter().copied().map(vec3).collect();
    VertexNormals::new(&positions, triangles)
}

fn normals(positions: &[Raw3], triangles: &[[u32; 3]]) -> Vec<Option<Raw3>> {
    let normals = build(positions, triangles).expect("an accepted mesh");
    let raw = |n: bunny_linalg::FixedUnitVec3| {
        let v = n.into_inner();
        [v.x.raw(), v.y.raw(), v.z.raw()]
    };
    normals.normals().iter().map(|n| n.map(raw)).collect()
}

/// A 3 by 3 grid of points `at(i, j)` in whole units, two counter-clockwise triangles per cell.
fn grid(at: impl Fn(i64, i64) -> [i64; 3]) -> Mesh {
    let positions = (0..3).flat_map(|j| (0..3).map(move |i| (i, j))).map(|(i, j)| units(at(i, j)));
    let mut triangles = Vec::new();
    for (i, j) in (0..2).flat_map(|j| (0..2).map(move |i| (i, j))) {
        let p = |di: u32, dj: u32| 3 * (j + dj) + i + di;
        triangles.push([p(0, 0), p(1, 0), p(1, 1)]);
        triangles.push([p(0, 0), p(1, 1), p(0, 1)]);
    }
    (positions.collect(), triangles)
}

/// The apex `(0, 0, 3)` above the square of corners `(+-4, 0, 0)`, `(0, +-4, 0)`: four faces.
fn fan() -> Mesh {
    let corners = [[0, 0, 3], [4, 0, 0], [0, 4, 0], [-4, 0, 0], [0, -4, 0]];
    (corners.map(units).to_vec(), vec![[0, 1, 2], [0, 2, 3], [0, 3, 4], [0, 4, 1]])
}

/// Two faces meeting at the origin: one in `z = 0` with cross product `(0, 0, 8)`, one in
/// `x = 0` with cross product `(6, 0, 0)`; both corners at the origin are right angles.
fn fold() -> Mesh {
    let points = [[0, 0, 0], [2, 0, 0], [0, 4, 0], [0, 2, 0], [0, 0, 3]];
    (points.map(units).to_vec(), vec![[0, 1, 2], [0, 3, 4]])
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_066_hand_derived_meshes_give_exact_vertex_normals() {
    assert_eq!(VERTEX_NORMALS_ID, "bunny-vertex-normals/1");
    // Flat in z = 0: every face normal is +Z, so is every vertex normal.
    let (points, faces) = grid(|i, j| [i, j, 0]);
    assert!(normals(&points, &faces).iter().all(|n| *n == Some([0, 0, ONE])));
    // Flat in 3y + 4z = 0: every face's cross product is (0, 3, 4) square units, a vertex sums
    // m of them (m = 1, 2, 3 or 6), the prescale is exact and |(0, 3m, 4m)| = 5m.
    let (points, faces) = grid(|i, j| [i, 4 * j, -3 * j]);
    let tilted = Some([0, SIX_TENTHS, EIGHT_TENTHS]);
    assert!(normals(&points, &faces).iter().all(|n| *n == tilted));
    // The fan: faces (12, 12, 16), (-12, 12, 16), (-12, -12, 16), (12, -12, 16); the apex sums
    // to (0, 0, 64), the axis; each base corner sums two, e.g. (24, 0, 32) = 8 (3, 0, 4).
    let (points, faces) = fan();
    let expected = [
        Some([0, 0, ONE]),
        Some([SIX_TENTHS, 0, EIGHT_TENTHS]),
        Some([0, SIX_TENTHS, EIGHT_TENTHS]),
        Some([-SIX_TENTHS, 0, EIGHT_TENTHS]),
        Some([0, -SIX_TENTHS, EIGHT_TENTHS]),
    ];
    assert_eq!(normals(&points, &faces), expected);
    // The fold: area weighting gives (6, 0, 8) / 10 at the origin; equal (unit or angle)
    // weights would give (1, 0, 1) / sqrt(2).
    let (points, faces) = fold();
    let (z, x) = (Some([0, 0, ONE]), Some([ONE, 0, 0]));
    assert_eq!(normals(&points, &faces), [Some([SIX_TENTHS, 0, EIGHT_TENTHS]), z, z, x, x]);
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_067_vertices_without_faces_or_with_a_zero_sum_have_no_normal() {
    assert!(normals(&[], &[]).is_empty());
    // An unreferenced sixth vertex.
    let (mut points, faces) = fan();
    points.push(units([1, 1, 1]));
    assert_eq!(normals(&points, &faces)[5], None);
    // Only zero-area faces: collinear corners, and a repeated index.
    let line = [units([0, 0, 0]), units([1, 0, 0]), units([2, 0, 0])];
    assert_eq!(normals(&line, &[[0, 1, 2], [0, 0, 1]]), [None, None, None]);
    // The same triangle wound both ways cancels exactly.
    let (points, _) = fold();
    assert_eq!(normals(&points[..3], &[[0, 1, 2], [0, 2, 1]]), [None, None, None]);
    // No welding: the fold with its shared corner duplicated gives each copy its own face.
    let (points, _) = fold();
    let split = [points[0], points[1], points[2], points[0], points[3], points[4]];
    let (z, x) = (Some([0, 0, ONE]), Some([ONE, 0, 0]));
    assert_eq!(normals(&split, &[[0, 1, 2], [3, 4, 5]]), [z, z, z, x, x, x]);
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_068_meshes_outside_the_domain_are_refused_in_order() {
    let (points, faces) = fan();
    let edge = TRACE_COORD_LIMIT_RAW;
    for coordinate in [edge, -edge, i64::MAX, i64::MIN] {
        let mut outside = points.clone();
        outside[3] = [0, coordinate, 0];
        outside[4] = [coordinate, 0, 0];
        let error = build(&outside, &faces).err();
        let position = vec3([0, coordinate, 0]);
        assert_eq!(
            error,
            Some(VertexNormalError::PositionOutsideTraceDomain { vertex: 3, position })
        );
    }
    let mut inside = points.clone();
    inside[3] = [0, edge - 1, -(edge - 1)];
    assert!(build(&inside, &faces).is_ok());
    // The first triangle with a bad index, and its first bad corner.
    let bad = [[0, 1, 2], [0, 7, 5], [9, 0, 1]];
    let error = build(&points, &bad).err();
    let expected = VertexNormalError::IndexOutOfRange { triangle: 1, index: 7, vertex_count: 5 };
    assert_eq!(error, Some(expected));
    assert_eq!(expected.to_string(), "triangle 1 lists vertex 7, but the mesh has 5 vertices");
    // Positions are checked before indices.
    let mut both = points;
    both[0] = [edge, 0, 0];
    let error = build(&both, &bad).err().map(|e| e.to_string());
    let text = format!("vertex 0 at raw ({edge}, 0, 0) is outside the trace domain");
    assert_eq!(error, Some(text));
    assert_eq!(MAX_NORMAL_TRIANGLES, 1 << 32);
}

/// An unsigned integer of any size: little-endian base-`2^32` limbs, no trailing zero limb.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Big(Vec<u32>);

impl Big {
    fn new(value: u128) -> Self {
        let limbs = (0..4).map(|k| (value >> (32 * k)) as u32).collect();
        Self(limbs).trim()
    }

    fn trim(mut self) -> Self {
        while self.0.last() == Some(&0) {
            self.0.pop();
        }
        self
    }

    fn mul(&self, other: &Self) -> Self {
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

    fn add(&self, other: &Self) -> Self {
        let mut out = Vec::new();
        let mut carry = 0_u64;
        for k in 0..self.0.len().max(other.0.len()) + 1 {
            let limb = |b: &Self| u64::from(b.0.get(k).copied().unwrap_or(0));
            let t = limb(self) + limb(other) + carry;
            out.push((t & 0xFFFF_FFFF) as u32);
            carry = t >> 32;
        }
        Self(out).trim()
    }

    fn compare(&self, other: &Self) -> Ordering {
        self.0.len().cmp(&other.0.len()).then_with(|| self.0.iter().rev().cmp(other.0.iter().rev()))
    }
}

/// Whether `a * sqrt(q) <= s * 2^32`, exactly.
fn at_most(a: i128, s: i128, q: &Big) -> bool {
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

fn shoelace(a: [i128; 3], b: [i128; 3], c: [i128; 3]) -> [i128; 3] {
    let cross = |p: [i128; 3], q: [i128; 3]| {
        [p[1] * q[2] - p[2] * q[1], p[2] * q[0] - p[0] * q[2], p[0] * q[1] - p[1] * q[0]]
    };
    let [ab, bc, ca] = [cross(a, b), cross(b, c), cross(c, a)];
    [0, 1, 2].map(|i| ab[i] + bc[i] + ca[i])
}

/// The exact area-weighted sum at `vertex`: every face that lists it, each once.
fn exact_sum(points: &[Raw3], faces: &[[u32; 3]], vertex: u32) -> [i128; 3] {
    let at = |i: u32| points[i as usize].map(i128::from);
    let mut sum = [0_i128; 3];
    for face in faces.iter().filter(|f| f.contains(&vertex)) {
        let n = shoelace(at(face[0]), at(face[1]), at(face[2]));
        sum = [0, 1, 2].map(|i| sum[i] + n[i]);
    }
    sum
}

/// Checks one vertex normal against the exact unit vector along `sum`; the reason on failure.
fn check_vertex(normal: Option<Raw3>, sum: [i128; 3]) -> Result<(), String> {
    let Some(raw) = normal else {
        return if sum == [0; 3] { Ok(()) } else { Err(format!("None for sum {sum:?}")) };
    };
    let square = |s: i128| Big::new(s.unsigned_abs()).mul(&Big::new(s.unsigned_abs()));
    let q = sum.iter().fold(Big::new(0), |q, &s| q.add(&square(s)));
    for (component, s) in raw.iter().zip(sum) {
        let r = i128::from(*component);
        if !(at_most(r - BOUND_RAW, s, &q) && at_most(-(r + BOUND_RAW), -s, &q)) {
            return Err(format!("{raw:?} is not within {BOUND_RAW} raw of the unit {sum:?}"));
        }
    }
    Ok(())
}

fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    *state >> 11
}

/// A seeded mesh: 3 to 12 vertices within `2^bits` raw of the origin (clamped to the trace
/// domain), 1 to 24 triangles of random indices (repeats, and so zero-area faces, included).
fn seeded_mesh(state: &mut u64) -> Mesh {
    let bits = [6, 20, 33, 38, 41][(next(state) % 5) as usize];
    let limit = (1_i64 << bits).min(TRACE_COORD_LIMIT_RAW - 1);
    let vertex_count = 3 + next(state) % 10;
    let face_count = 1 + next(state) % 24;
    let mut coordinate = || (next(state) % (2 * limit as u64 + 1)) as i64 - limit;
    let points = (0..vertex_count).map(|_| [coordinate(), coordinate(), coordinate()]).collect();
    let mut index = || (next(state) % vertex_count) as u32;
    let faces = (0..face_count).map(|_| [index(), index(), index()]).collect();
    (points, faces)
}

fn check_mesh(points: &[Raw3], faces: &[[u32; 3]]) -> Result<(), String> {
    for (vertex, normal) in normals(points, faces).into_iter().enumerate() {
        let sum = exact_sum(points, faces, vertex as u32);
        check_vertex(normal, sum).map_err(|why| format!("vertex {vertex}: {why}"))?;
    }
    Ok(())
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_070_vertex_normals_are_within_one_raw_unit_of_the_exact_oracle() {
    let mut state = SEED;
    let (mut nones, mut somes) = (0, 0);
    for case in 0..400 {
        let (points, faces) = seeded_mesh(&mut state);
        let result = check_mesh(&points, &faces);
        assert!(result.is_ok(), "seed {SEED:#x}, case {case}: {}", result.unwrap_err());
        let found = normals(&points, &faces);
        nones += found.iter().filter(|n| n.is_none()).count();
        somes += found.iter().filter(|n| n.is_some()).count();
    }
    // Many faces at the corners of the domain around one vertex: the largest sums.
    let max = TRACE_COORD_LIMIT_RAW - 1;
    let corners = [[max, -max, max], [-max, max, max], [max, max, -max], [-max, -max, -max]];
    let points: Vec<Raw3> = [[0, 0, 0]].into_iter().chain(corners).collect();
    let faces: Vec<[u32; 3]> = (0..1000).map(|k| [0, 1 + k % 4, 1 + (k + 1 + k / 4) % 4]).collect();
    assert_eq!(check_mesh(&points, &faces), Ok(()));
    // Witnesses: both outcomes occur.
    assert!(nones > 100 && somes > 1000, "{nones} None, {somes} Some");
}

/// The mesh with its faces shuffled, each face's corners rotated, and its vertices relabelled
/// by a seeded permutation; and that permutation (old label to new).
fn shuffled(points: &[Raw3], faces: &[[u32; 3]], state: &mut u64) -> (Mesh, Vec<u32>) {
    let mut label: Vec<u32> = (0..points.len() as u32).collect();
    for i in (1..label.len()).rev() {
        label.swap(i, (next(state) % (i as u64 + 1)) as usize);
    }
    let mut moved = vec![[0; 3]; points.len()];
    for (old, &new) in label.iter().enumerate() {
        moved[new as usize] = points[old];
    }
    let mut renamed: Vec<[u32; 3]> = faces.iter().map(|f| f.map(|v| label[v as usize])).collect();
    for i in (1..renamed.len()).rev() {
        renamed.swap(i, (next(state) % (i as u64 + 1)) as usize);
    }
    for face in &mut renamed {
        face.rotate_left((next(state) % 3) as usize);
    }
    ((moved, renamed), label)
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_071_vertex_normals_do_not_depend_on_face_order_or_vertex_labels() {
    let mut state = SEED ^ 0x0EDE_0001;
    for case in 0..300 {
        let (points, faces) = seeded_mesh(&mut state);
        let before = normals(&points, &faces);
        let ((points, faces), label) = shuffled(&points, &faces, &mut state);
        let after = normals(&points, &faces);
        for (old, normal) in before.iter().enumerate() {
            let new = label[old] as usize;
            assert_eq!(after[new], *normal, "seed {SEED:#x}, case {case}, vertex {old}");
        }
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_072_golden_vertex_normals_lock_the_rule() {
    // A tetrahedron with irregular corners and a fifth vertex on one extra face: inexact and
    // negative components, and prescales that shift down.
    let points = [
        [ONE / 3, -5 * ONE / 4, 2 * ONE],
        [-7 * ONE / 3, ONE / 5, -ONE / 7],
        [3 * ONE, 2 * ONE / 3, ONE / 9],
        [ONE / 2, 3 * ONE, -ONE],
        [-ONE, -ONE, -3 * ONE],
    ];
    let faces = [[0, 1, 2], [0, 3, 1], [1, 3, 2], [2, 3, 0], [4, 1, 0]];
    // Produced by the crate and checked against the oracle of OPTICS-TP-070 below; changing
    // them is a contract change (`bunny-vertex-normals/2`).
    let golden = [
        Some([1_157_333_538, 3_256_708_161, 2_549_740_205]),
        Some([1_427_067_459, 897_767_543, -3_950_219_738]),
        Some([1_357_741_649, -2_385_221_816, -3_303_634_147]),
        Some([-417_565_197, 3_281_100_079, 2_739_847_743]),
        Some([2_305_634_351, 3_597_443_668, -434_963_643]),
    ];
    assert_eq!(normals(&points, &faces), golden);
    assert_eq!(check_mesh(&points, &faces), Ok(()));
}
