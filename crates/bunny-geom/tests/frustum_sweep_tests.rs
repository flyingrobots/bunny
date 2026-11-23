//! Seeded sweeps for frustum classification and plane extraction.
//!
//! The oracles here share no helper with the kernel: signed distances are modelled in
//! `i128` with their own ties-to-even rounding, the sphere comparison uses a schoolbook
//! multiplication over 32-bit limbs, and the extraction is checked against exact clip
//! coordinates formed in `i128` from the matrix entries. See
//! `docs/topics/frustum/test-plan.md` for the seeds and the pinned counts.

use bunny_geom::{
    FixedAabb3, FixedFrustum, FixedPlane3, FixedSphere3, FrustumClass, FrustumSphereError,
    PlaneSide, PlaneSpan,
};
use bunny_linalg::{FixedMat4, FixedVec3};
use bunny_num::FixedQ32_32;
use std::cmp::Ordering;
use wasm_bindgen_test::wasm_bindgen_test;

const UNIT: i64 = 1_i64 << 32;
const HALF: i64 = 1_i64 << 31;
const BOX_SEED: u64 = 0x146F_0001_9E37_79B9;
const SPHERE_SEED: u64 = 0x146F_0002_7F4A_7C15;
const EXTRACT_SEED: u64 = 0x146F_0003_D1B5_4A32;
const CAMERA_SEED: u64 = 0x146F_0004_94D0_49BB;

fn raw(v: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(v)
}

fn pt(x: i64, y: i64, z: i64) -> FixedVec3 {
    FixedVec3::new(raw(x), raw(y), raw(z))
}

fn plane(nx: i64, ny: i64, nz: i64, offset: i64) -> FixedPlane3 {
    FixedPlane3::new(pt(nx, ny, nz), raw(offset))
}

// ---- Independent model of the signed distance ----

fn fits(v: i128) -> Option<i128> {
    (i128::from(i64::MIN)..=i128::from(i64::MAX)).contains(&v).then_some(v)
}

/// One Q32.32 product: exact in `i128`, rounded to nearest, ties to even, refused outside i64.
fn model_mul(a: i64, b: i64) -> Option<i128> {
    let exact = i128::from(a) * i128::from(b);
    let floor = exact.div_euclid(1_i128 << 32);
    let rem = exact.rem_euclid(1_i128 << 32);
    let rounded =
        if rem > (1 << 31) || (rem == (1 << 31) && floor % 2 != 0) { floor + 1 } else { floor };
    fits(rounded)
}

/// `((x + y) + z) - offset`, each step refused outside i64.
fn model_distance(on: FixedPlane3, point: FixedVec3) -> Option<i128> {
    let normal = on.normal;
    let along_x = model_mul(normal.x.raw(), point.x.raw())?;
    let along_y = model_mul(normal.y.raw(), point.y.raw())?;
    let along_z = model_mul(normal.z.raw(), point.z.raw())?;
    fits(fits(fits(along_x + along_y)? + along_z)? - i128::from(on.offset.raw()))
}

fn corners(b: FixedAabb3) -> Vec<FixedVec3> {
    let mut out = Vec::new();
    for x in [b.min.x, b.max.x] {
        for y in [b.min.y, b.max.y] {
            for z in [b.min.z, b.max.z] {
                out.push(FixedVec3::new(x, y, z));
            }
        }
    }
    out
}

fn model_box_span(p: FixedPlane3, b: FixedAabb3) -> Option<PlaneSpan> {
    let (mut front, mut behind) = (false, false);
    for c in corners(b) {
        let d = model_distance(p, c)?;
        front |= d > 0;
        behind |= d < 0;
    }
    Some(match (front, behind) {
        (false, false) => PlaneSpan::Coplanar,
        (true, false) => PlaneSpan::Front,
        (false, true) => PlaneSpan::Behind,
        (true, true) => PlaneSpan::Straddling,
    })
}

/// The stated rule, written out again: Behind anywhere is Outside; then overflow; then a
/// straddled plane is Intersecting; otherwise Inside.
fn model_combine(spans: &[Option<PlaneSpan>]) -> Option<FrustumClass> {
    if spans.contains(&Some(PlaneSpan::Behind)) {
        return Some(FrustumClass::Outside);
    }
    if spans.iter().any(Option::is_none) {
        return None;
    }
    if spans.contains(&Some(PlaneSpan::Straddling)) {
        return Some(FrustumClass::Intersecting);
    }
    Some(FrustumClass::Inside)
}

// ---- Schoolbook big integers over 32-bit limbs, little-endian ----

fn big(v: u128) -> Vec<u32> {
    (0..4).map(|i| u32::try_from((v >> (32 * i)) & 0xFFFF_FFFF).unwrap()).collect()
}

fn big_mul(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut out = vec![0_u64; a.len() + b.len() + 1];
    for (i, &x) in a.iter().enumerate() {
        let mut carry = 0_u64;
        for (j, &y) in b.iter().enumerate() {
            let t = out[i + j] + u64::from(x) * u64::from(y) + carry;
            out[i + j] = t & 0xFFFF_FFFF;
            carry = t >> 32;
        }
        let mut k = i + b.len();
        while carry != 0 {
            let t = out[k] + carry;
            out[k] = t & 0xFFFF_FFFF;
            carry = t >> 32;
            k += 1;
        }
    }
    out.into_iter().map(|limb| u32::try_from(limb).unwrap()).collect()
}

fn big_add(a: &[u32], b: &[u32]) -> Vec<u32> {
    let len = a.len().max(b.len()) + 1;
    let mut out = Vec::with_capacity(len);
    let mut carry = 0_u64;
    for i in 0..len {
        let t = u64::from(*a.get(i).unwrap_or(&0)) + u64::from(*b.get(i).unwrap_or(&0)) + carry;
        out.push(u32::try_from(t & 0xFFFF_FFFF).unwrap());
        carry = t >> 32;
    }
    out
}

fn big_cmp(a: &[u32], b: &[u32]) -> Ordering {
    let len = a.len().max(b.len());
    for i in (0..len).rev() {
        let (x, y) = (*a.get(i).unwrap_or(&0), *b.get(i).unwrap_or(&0));
        if x != y {
            return x.cmp(&y);
        }
    }
    Ordering::Equal
}

fn big_square(v: i128) -> Vec<u32> {
    let m = big(v.unsigned_abs());
    big_mul(&m, &m)
}

/// The sphere against one plane: `D^2 2^64` against `R^2 |N|^2` in big integers.
fn model_sphere_span(p: FixedPlane3, s: FixedSphere3) -> Option<PlaneSpan> {
    let d = model_distance(p, s.center)?;
    let n = p.normal;
    let norm = big_add(
        &big_add(&big_square(n.x.raw().into()), &big_square(n.y.raw().into())),
        &big_square(n.z.raw().into()),
    );
    let reach = big_mul(&big_square(s.radius.raw().into()), &norm);
    let clearance = big_mul(&big_square(d), &big(1_u128 << 64));
    Some(match (big_cmp(&clearance, &reach), d < 0) {
        (Ordering::Less, _) => PlaneSpan::Straddling,
        (_, true) => PlaneSpan::Behind,
        (_, false) => PlaneSpan::Front,
    })
}

// ---- Seeded generator ----

struct Xorshift(u64);

const EXTREMES: [i64; 9] = [i64::MIN, -(1 << 62), -(1 << 32), -1, 0, 1, 1 << 32, 1 << 62, i64::MAX];

impl Xorshift {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn range(&mut self, limit: i64) -> i64 {
        (self.next() % (2 * limit.cast_unsigned() + 1)).cast_signed() - limit
    }

    fn below(&mut self, limit: i64) -> i64 {
        (self.next() % limit.cast_unsigned()).cast_signed()
    }

    fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[usize::try_from(self.next() % u64::try_from(items.len()).unwrap()).unwrap()]
    }

    /// Mode 0: wide random. Mode 1: half-unit grid with raw-unit jitter. Mode 2: range ends.
    fn coord(&mut self, mode: u64) -> i64 {
        match mode {
            0 => self.range(1000 * UNIT),
            1 => self.range(8) * HALF + self.pick(&[0, 0, 0, 1, -1]),
            _ => self.pick(&EXTREMES).saturating_add(self.range(2)),
        }
    }

    fn point(&mut self, mode: u64) -> FixedVec3 {
        pt(self.coord(mode), self.coord(mode), self.coord(mode))
    }

    /// A shape point: for mode 3, a quarter-unit grid within 3/2 of `(0, 0, -2)` with
    /// raw-unit jitter, so boxes and spheres fall inside, across and on the faces.
    fn shape_point(&mut self, mode: u64) -> FixedVec3 {
        if mode != 3 {
            return self.point(mode);
        }
        let mut c = || self.range(6) * (UNIT / 4) + self.pick(&[0, 0, 0, 1, -1]);
        pt(c(), c(), c() - 2 * UNIT)
    }

    /// Modes 0 to 2: six random planes. Mode 3: the box or pyramid frustum of
    /// `frustum_tests.rs` with each offset moved by up to one raw unit.
    fn frustum(&mut self, mode: u64) -> FixedFrustum {
        if mode == 3 {
            let base = if self.next().is_multiple_of(2) { cube() } else { pyramid() };
            return FixedFrustum::new(
                base.planes()
                    .map(|p| FixedPlane3::new(p.normal, raw(p.offset.raw() + self.range(1)))),
            );
        }
        let mut planes = [plane(0, 0, 0, 0); 6];
        for p in &mut planes {
            *p = FixedPlane3::new(self.point(mode), raw(self.coord(mode)));
        }
        FixedFrustum::new(planes)
    }
}

fn cube() -> FixedFrustum {
    FixedFrustum::new([
        plane(UNIT, 0, 0, -UNIT),
        plane(-UNIT, 0, 0, -UNIT),
        plane(0, UNIT, 0, -UNIT),
        plane(0, -UNIT, 0, -UNIT),
        plane(0, 0, -UNIT, UNIT),
        plane(0, 0, UNIT, -3 * UNIT),
    ])
}

fn pyramid() -> FixedFrustum {
    FixedFrustum::new([
        plane(UNIT, 0, -UNIT, 0),
        plane(-UNIT, 0, -UNIT, 0),
        plane(0, UNIT, -UNIT, 0),
        plane(0, -UNIT, -UNIT, 0),
        plane(0, 0, -3 * HALF, 3 * HALF),
        plane(0, 0, HALF, -3 * HALF),
    ])
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Tally {
    inside: u32,
    intersecting: u32,
    outside: u32,
    overflow: u32,
}

impl Tally {
    fn add(&mut self, class: Option<FrustumClass>) {
        match class {
            Some(FrustumClass::Inside) => self.inside += 1,
            Some(FrustumClass::Intersecting) => self.intersecting += 1,
            Some(FrustumClass::Outside) => self.outside += 1,
            None => self.overflow += 1,
        }
    }
}

/// Outside is certain: some plane has every corner Behind or On, one Behind, by `classify`.
fn culling_plane_exists(f: FixedFrustum, b: FixedAabb3) -> bool {
    f.planes().iter().any(|p| {
        let sides: Option<Vec<PlaneSide>> = corners(b).iter().map(|c| p.classify(*c)).collect();
        sides.is_some_and(|s| !s.contains(&PlaneSide::Front) && s.contains(&PlaneSide::Behind))
    })
}

fn check_box(f: FixedFrustum, b: FixedAabb3, case: &str) -> Option<FrustumClass> {
    let got = f.classify_aabb(b);
    let per_plane: Vec<Option<PlaneSpan>> = f.planes().iter().map(|p| p.classify_aabb(b)).collect();
    assert_eq!(got, model_combine(&per_plane), "{case}: six classify_aabb by the rule");
    let modelled: Vec<Option<PlaneSpan>> =
        f.planes().iter().map(|p| model_box_span(*p, b)).collect();
    assert_eq!(got, model_combine(&modelled), "{case}: independent eight-corner model");
    match got {
        Some(FrustumClass::Outside) => {
            assert!(culling_plane_exists(f, b), "{case}: Outside not certain");
        }
        Some(FrustumClass::Inside) => {
            for c in corners(b) {
                assert_eq!(f.contains_point(c), Some(true), "{case}: Inside corner {c:?}");
            }
        }
        _ => assert!(!culling_plane_exists(f, b), "{case}: a culling plane was missed"),
    }
    got
}

#[wasm_bindgen_test(unsupported = test)]
fn frustum_tp_011_seeded_boxes_follow_the_rule_and_outside_is_certain() {
    let mut rng = Xorshift(BOX_SEED);
    let mut tally = Tally::default();
    for i in 0..20_000 {
        let mode = rng.next() % 4;
        let f = rng.frustum(mode);
        let b = FixedAabb3::new(rng.shape_point(mode), rng.shape_point(mode));
        tally.add(check_box(f, b, &format!("seed {BOX_SEED:#x} case {i} mode {mode}")));
    }
    println!("FRUSTUM-TP-011 seed {BOX_SEED:#x}: {tally:?}");
    assert_eq!(tally, Tally { inside: 942, intersecting: 8033, outside: 6975, overflow: 4050 });
}

fn check_sphere(f: FixedFrustum, s: FixedSphere3, case: &str) -> Option<FrustumClass> {
    let got = f.classify_sphere(s);
    let modelled: Vec<Option<PlaneSpan>> =
        f.planes().iter().map(|p| model_sphere_span(*p, s)).collect();
    let want = model_combine(&modelled).ok_or(FrustumSphereError::Overflow);
    assert_eq!(got, want, "{case}: sphere {s:?}");
    if s.radius.raw() == 0 {
        let as_point = f.contains_point(s.center);
        let class = got.ok().map(|c| c == FrustumClass::Inside);
        assert_eq!(class, as_point, "{case}: zero radius is the point rule");
    }
    got.ok()
}

#[wasm_bindgen_test(unsupported = test)]
fn frustum_tp_012_seeded_spheres_match_the_wide_integer_model() {
    let mut rng = Xorshift(SPHERE_SEED);
    let mut tally = Tally::default();
    for i in 0..20_000 {
        let mode = rng.next() % 4;
        let f = rng.frustum(mode);
        let shape = if mode == 3 { 1 } else { mode };
        let radius = match rng.next() % 4 {
            0 => 0,
            1 => rng.below(6) * (UNIT / 4) + rng.pick(&[0, 0, 1]),
            2 => rng.coord(shape).checked_abs().unwrap_or(i64::MAX),
            _ => rng.pick(&[i64::MAX, 1 << 62, 1 << 40, 1]),
        };
        let s = FixedSphere3::new(rng.shape_point(mode), raw(radius));
        tally.add(check_sphere(f, s, &format!("seed {SPHERE_SEED:#x} case {i} mode {mode}")));
        let negative = FixedSphere3::new(s.center, raw(-1 - rng.below(UNIT)));
        assert_eq!(f.classify_sphere(negative), Err(FrustumSphereError::NegativeRadius));
    }
    println!("FRUSTUM-TP-012 seed {SPHERE_SEED:#x}: {tally:?}");
    assert_eq!(tally, Tally { inside: 920, intersecting: 5508, outside: 10677, overflow: 2895 });
}

// ---- Extraction ----

type Rows = [[i64; 4]; 4];

fn rows_of(m: FixedMat4) -> Rows {
    let r = |row: (FixedQ32_32, FixedQ32_32, FixedQ32_32, FixedQ32_32)| {
        [row.0.raw(), row.1.raw(), row.2.raw(), row.3.raw()]
    };
    [r(m.row0()), r(m.row1()), r(m.row2()), r(m.row3())]
}

fn mat(rows: Rows) -> FixedMat4 {
    let row = |r: [i64; 4]| (raw(r[0]), raw(r[1]), raw(r[2]), raw(r[3]));
    FixedMat4::from_rows(row(rows[0]), row(rows[1]), row(rows[2]), row(rows[3]))
}

/// The six planes as `(a, b, c, d)` rows, in i128: `w + x`, `w - x`, `w + y`, `w - y`, `z`,
/// `w - z`, with `z >= 0` the near plane of the 0..=1 depth range.
fn model_rows(rows: &Rows) -> [[i128; 4]; 6] {
    let (x, y, z, w) = (rows[0], rows[1], rows[2], rows[3]);
    let comb = |a: [i64; 4], b: [i64; 4], sign: i128| -> [i128; 4] {
        std::array::from_fn(|k| i128::from(a[k]) + sign * i128::from(b[k]))
    };
    [comb(w, x, 1), comb(w, x, -1), comb(w, y, 1), comb(w, y, -1), comb(z, z, 0), comb(w, z, -1)]
}

fn model_extract(m: &Rows) -> Option<[FixedPlane3; 6]> {
    let mut out = [plane(0, 0, 0, 0); 6];
    for (slot, r) in out.iter_mut().zip(model_rows(m)) {
        let c = |v: i128| fits(v).map(|v| i64::try_from(v).unwrap());
        // The kernel forms d and then -d: both must fit.
        c(r[3])?;
        *slot = plane(c(r[0])?, c(r[1])?, c(r[2])?, c(-r[3])?);
    }
    Some(out)
}

/// Exact clip coordinates of `p` under the stored entries, times `2^64` (raw times `2^32`).
fn exact_clip(m: &Rows, p: [i64; 3]) -> [i128; 4] {
    m.map(|r| {
        let mut acc = i128::from(r[3]) << 32;
        for (e, q) in r.iter().zip(p) {
            acc += i128::from(*e) * i128::from(q);
        }
        acc
    })
}

fn random_rows(rng: &mut Xorshift, mode: u64) -> Rows {
    let mut m = [[0; 4]; 4];
    for row in &mut m {
        for e in row.iter_mut() {
            *e = match mode {
                0 => rng.range(8 * UNIT),
                1 => rng.range(4) * HALF + rng.pick(&[0, 0, 1, -1]),
                _ => rng.coord(2),
            };
        }
    }
    m
}

#[wasm_bindgen_test(unsupported = test)]
fn frustum_tp_013_seeded_extraction_is_exact_and_within_three_halves_raw() {
    let mut rng = Xorshift(EXTRACT_SEED);
    let (mut refused, mut decided, mut near_face) = (0_u32, 0_u32, 0_u32);
    for i in 0..10_000 {
        let mode = rng.next() % 3;
        let rows = random_rows(&mut rng, mode);
        let case = format!("seed {EXTRACT_SEED:#x} case {i} mode {mode}");
        let got = FixedFrustum::from_clip_matrix(mat(rows));
        assert_eq!(got.map(FixedFrustum::planes), model_extract(&rows), "{case}: planes");
        let Some(frustum) = got else {
            refused += 1;
            continue;
        };
        if mode == 2 {
            continue;
        }
        for _ in 0..4 {
            let mut coord = || match mode {
                0 => rng.range(64 * UNIT),
                _ => rng.range(16) * (UNIT / 4) + rng.pick(&[0, 0, 0, 1, -1]),
            };
            let coords = [coord(), coord(), coord()];
            let [cx, cy, cz, cw] = exact_clip(&rows, coords);
            let exact = [cw + cx, cw - cx, cw + cy, cw - cy, cz, cw - cz];
            let point = pt(coords[0], coords[1], coords[2]);
            for (face, want) in frustum.planes().iter().zip(exact) {
                let got = face.signed_distance(point).unwrap().raw();
                assert!(
                    2 * (i128::from(got) * i128::from(UNIT) - want).abs() <= 3 * i128::from(UNIT),
                    "{case}: bound"
                );
            }
            let margin = 3 * i128::from(HALF);
            if exact.iter().all(|e| *e > margin) {
                assert_eq!(frustum.contains_point(point), Some(true), "{case}: clearly inside");
                decided += 1;
            } else if exact.iter().any(|e| *e < -margin) {
                assert_eq!(frustum.contains_point(point), Some(false), "{case}: clearly outside");
                decided += 1;
            } else {
                near_face += 1;
            }
        }
    }
    println!("FRUSTUM-TP-013 seed {EXTRACT_SEED:#x}: refused {refused} decided {decided} near a face {near_face}");
    assert_eq!((refused, decided, near_face), (3236, 26491, 25));
}

// ---- Seeded cameras: the repository's own builders ----

/// `a * b` in raw units, floored: the test's own arithmetic, not the kernel's.
fn fmul(a: i128, b: i128) -> i128 {
    (a * b) >> 32
}

/// A world point from a view-space point, through the look-at rows (right, up, back).
fn to_world(view: &Rows, eye: [i64; 3], v: [i128; 3]) -> FixedVec3 {
    let c = |k: usize| {
        let along = (0..3).map(|r| fmul(v[r], i128::from(view[r][k]))).sum::<i128>();
        i64::try_from(i128::from(eye[k]) + along).unwrap()
    };
    pt(c(0), c(1), c(2))
}

struct Camera {
    clip_from_world: FixedMat4,
    view: Rows,
    eye: [i64; 3],
    /// Half extents at depth `d` (raw): `(width, height)`.
    perspective: Option<(i128, i128)>,
    ortho: (i128, i128),
    near: i128,
    far: i128,
}

impl Camera {
    fn half_extents(&self, d: i128) -> (i128, i128) {
        match self.perspective {
            Some((tan_w, tan_h)) => (fmul(d, tan_w), fmul(d, tan_h)),
            None => self.ortho,
        }
    }
}

fn camera(rng: &mut Xorshift) -> Option<Camera> {
    let near = if rng.next().is_multiple_of(2) { UNIT / 16 + rng.below(4 * UNIT) } else { 0 };
    let far = near + UNIT + rng.below(999 * UNIT);
    let (projection, perspective, ortho) = if near > 0 {
        let tan = UNIT / 4 + rng.below(15 * UNIT / 4);
        let aspect = UNIT / 2 + rng.below(3 * UNIT / 2);
        let m = FixedMat4::try_perspective(raw(tan), raw(aspect), raw(near), raw(far)).unwrap();
        (m, Some((fmul(tan.into(), aspect.into()), i128::from(tan))), (0, 0))
    } else {
        let (hw, hh) = (UNIT / 4 + rng.below(64 * UNIT), UNIT / 4 + rng.below(64 * UNIT));
        let m = FixedMat4::try_orthographic(raw(hw), raw(hh), raw(near), raw(far)).unwrap();
        (m, None, (i128::from(hw), i128::from(hh)))
    };
    let eye = [rng.range(100 * UNIT), rng.range(100 * UNIT), rng.range(100 * UNIT)];
    let dir = [rng.range(10 * UNIT), rng.range(10 * UNIT), rng.range(10 * UNIT)];
    let target = pt(eye[0] + dir[0], eye[1] + dir[1], eye[2] + dir[2]);
    let up = pt(rng.range(UNIT), rng.range(UNIT), rng.range(UNIT));
    let view = FixedMat4::try_look_at(pt(eye[0], eye[1], eye[2]), target, up).ok()?;
    let clip_from_world = projection.checked_mul_mat4(view).unwrap();
    Some(Camera {
        clip_from_world,
        view: rows_of(view),
        eye,
        perspective,
        ortho,
        near: near.into(),
        far: far.into(),
    })
}

/// A view-space point strictly inside by 1/64 of each face's extent.
fn inside_point(rng: &mut Xorshift, cam: &Camera) -> [i128; 3] {
    let margin = (cam.far - cam.near) / 64;
    let d = cam.near
        + margin
        + i128::from(rng.below(i64::try_from(cam.far - cam.near - 2 * margin).unwrap()));
    let (hw, hh) = cam.half_extents(d);
    let lateral =
        |rng: &mut Xorshift, h: i128| i128::from(rng.range(i64::try_from(h - h / 64).unwrap()));
    [lateral(rng, hw), lateral(rng, hh), -d]
}

/// A view-space point outside one face by at least 1/64 of that face's extent.
fn outside_point(rng: &mut Xorshift, cam: &Camera, face: u64) -> [i128; 3] {
    let margin = (cam.far - cam.near) / 64;
    let [x, y, z] = inside_point(rng, cam);
    let d = -z;
    let (hw, hh) = cam.half_extents(d);
    let beyond =
        |rng: &mut Xorshift, h: i128| h + h / 64 + i128::from(rng.below(i64::try_from(h).unwrap()));
    match face {
        0 => [-beyond(rng, hw), y, z],
        1 => [beyond(rng, hw), y, z],
        2 => [x, -beyond(rng, hh), z],
        3 => [x, beyond(rng, hh), z],
        4 => [x, y, -(cam.near - margin - i128::from(rng.below(UNIT)))],
        _ => [x, y, -(cam.far + margin + i128::from(rng.below(UNIT)))],
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn frustum_tp_014_seeded_cameras_classify_points_known_inside_and_outside() {
    let mut rng = Xorshift(CAMERA_SEED);
    let (mut cameras, mut skipped, mut points) = (0_u32, 0_u32, 0_u32);
    for i in 0..1_000 {
        let Some(cam) = camera(&mut rng) else {
            skipped += 1;
            continue;
        };
        cameras += 1;
        let f = FixedFrustum::from_clip_matrix(cam.clip_from_world).unwrap();
        for k in 0..8 {
            let v = inside_point(&mut rng, &cam);
            let p = to_world(&cam.view, cam.eye, v);
            assert_eq!(
                f.contains_point(p),
                Some(true),
                "seed {CAMERA_SEED:#x} camera {i} inside {k}: {v:?}"
            );
            points += 1;
        }
        for face in 0..6 {
            let v = outside_point(&mut rng, &cam, face);
            let p = to_world(&cam.view, cam.eye, v);
            assert_eq!(
                f.contains_point(p),
                Some(false),
                "seed {CAMERA_SEED:#x} camera {i} face {face}: {v:?}"
            );
            points += 1;
        }
    }
    println!(
        "FRUSTUM-TP-014 seed {CAMERA_SEED:#x}: cameras {cameras} skipped {skipped} points {points}"
    );
    assert_eq!((cameras, skipped, points), (1000, 0, 14000));
}
