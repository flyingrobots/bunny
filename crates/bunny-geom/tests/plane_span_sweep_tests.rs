//! Seeded equivalence sweeps for segment, triangle and box classification.
//!
//! Every result is compared bit for bit with two definitions: the eight corners (or the
//! vertices) through the existing `FixedPlane3::classify`, and an independent model of the
//! documented signed distance written here in `i128`, sharing no helper with the kernel.

use bunny_geom::{
    FixedAabb3, FixedPlane3, FixedSegment3, PlaneSide, PlaneSpan, PolygonClip, PolygonClipError,
    SegmentClip,
};
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const UNIT: i64 = 1_i64 << 32;
const BOX_SEED: u64 = 0xA076_1D64_78BD_642F;
const EXTREME_SEED: u64 = 0xE703_7ED1_A0B4_28DB;
const SHAPE_SEED: u64 = 0x8EBC_6AF0_9C88_C6E3;

fn raw(v: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(v)
}

fn pt(x: i64, y: i64, z: i64) -> FixedVec3 {
    FixedVec3::new(raw(x), raw(y), raw(z))
}

// ---- Independent model of the documented signed distance ----

/// One Q32.32 product: exact in `i128`, rounded to nearest, ties to even, refused outside i64.
fn model_mul(a: i64, b: i64) -> Option<i128> {
    let exact = i128::from(a) * i128::from(b);
    let floor = exact.div_euclid(1_i128 << 32);
    let rem = exact.rem_euclid(1_i128 << 32);
    let half = 1_i128 << 31;
    let rounded = if rem > half || (rem == half && floor % 2 != 0) { floor + 1 } else { floor };
    fits(rounded)
}

fn fits(v: i128) -> Option<i128> {
    (i128::from(i64::MIN)..=i128::from(i64::MAX)).contains(&v).then_some(v)
}

/// `((x + y) + z) - offset`, each step refused outside i64, as the chapter states.
fn model_distance(plane: FixedPlane3, p: FixedVec3) -> Option<i128> {
    let (n, o) = (plane.normal, plane.offset.raw());
    let x = model_mul(n.x.raw(), p.x.raw())?;
    let y = model_mul(n.y.raw(), p.y.raw())?;
    let z = model_mul(n.z.raw(), p.z.raw())?;
    fits(fits(fits(x + y)? + z)? - i128::from(o))
}

fn model_span(distances: &[Option<i128>]) -> Option<PlaneSpan> {
    let mut front = false;
    let mut behind = false;
    for d in distances {
        let d = (*d)?;
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

fn corners(b: FixedAabb3) -> [FixedVec3; 8] {
    let (lo, hi) = (b.min, b.max);
    [
        FixedVec3::new(lo.x, lo.y, lo.z),
        FixedVec3::new(lo.x, lo.y, hi.z),
        FixedVec3::new(lo.x, hi.y, lo.z),
        FixedVec3::new(lo.x, hi.y, hi.z),
        FixedVec3::new(hi.x, lo.y, lo.z),
        FixedVec3::new(hi.x, lo.y, hi.z),
        FixedVec3::new(hi.x, hi.y, lo.z),
        FixedVec3::new(hi.x, hi.y, hi.z),
    ]
}

fn model_points(plane: FixedPlane3, points: &[FixedVec3]) -> Option<PlaneSpan> {
    let distances: Vec<Option<i128>> = points.iter().map(|p| model_distance(plane, *p)).collect();
    model_span(&distances)
}

/// The eight-corner (or vertex) definition through the existing point rule.
fn classify_points(plane: FixedPlane3, points: &[FixedVec3]) -> Option<PlaneSpan> {
    let mut sides = Vec::new();
    for p in points {
        sides.push(plane.classify(*p)?);
    }
    let front = sides.contains(&PlaneSide::Front);
    let behind = sides.contains(&PlaneSide::Behind);
    Some(match (front, behind) {
        (false, false) => PlaneSpan::Coplanar,
        (true, false) => PlaneSpan::Front,
        (false, true) => PlaneSpan::Behind,
        (true, true) => PlaneSpan::Straddling,
    })
}

// ---- Seeded generator ----

struct Xorshift(u64);

/// Coordinates at and near the ends of the Q32.32 range.
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

    fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        let len = u64::try_from(items.len()).unwrap();
        let index = usize::try_from(self.next() % len).unwrap();
        items[index]
    }

    /// Mode 0: wide random. Mode 1: a unit grid with raw-unit jitter, so touching and
    /// coplanar cases are common. Mode 2: the ends of the range.
    fn coord(&mut self, mode: u64) -> i64 {
        match mode {
            0 => self.range(1000 * UNIT),
            1 => self.range(3) * UNIT + self.pick(&[0, 0, 0, 1, -1]),
            _ => self.pick(&EXTREMES).saturating_add(self.range(2)),
        }
    }

    fn point(&mut self, mode: u64) -> FixedVec3 {
        pt(self.coord(mode), self.coord(mode), self.coord(mode))
    }

    fn plane(&mut self, mode: u64) -> FixedPlane3 {
        let comp = |r: &mut Self| match mode {
            0 => r.range(2 * UNIT),
            1 => r.pick(&[-UNIT, 0, 0, UNIT, 2 * UNIT]),
            _ => r.pick(&EXTREMES).saturating_add(r.range(2)),
        };
        let normal = pt(comp(self), comp(self), comp(self));
        let offset = match mode {
            0 => self.range(1000 * UNIT),
            1 => self.range(6) * UNIT,
            _ => self.pick(&EXTREMES),
        };
        FixedPlane3::new(normal, raw(offset))
    }

    /// A representable point inside the box's corner set, per axis between the two ends.
    fn inside(&mut self, b: FixedAabb3) -> FixedVec3 {
        let mut axis = |a: FixedQ32_32, c: FixedQ32_32| {
            let (lo, hi) = (a.raw().min(c.raw()), a.raw().max(c.raw()));
            let span = (i128::from(hi) - i128::from(lo)).cast_unsigned();
            let step = u128::from(self.next()) * u128::from(self.next()) % (span + 1);
            raw(i64::try_from(i128::from(lo) + step.cast_signed()).unwrap())
        };
        FixedVec3::new(axis(b.min.x, b.max.x), axis(b.min.y, b.max.y), axis(b.min.z, b.max.z))
    }
}

#[derive(Default, Debug)]
struct Tally {
    front: u32,
    behind: u32,
    straddling: u32,
    coplanar: u32,
    overflow: u32,
}

impl Tally {
    fn add(&mut self, span: Option<PlaneSpan>) {
        match span {
            Some(PlaneSpan::Front) => self.front += 1,
            Some(PlaneSpan::Behind) => self.behind += 1,
            Some(PlaneSpan::Straddling) => self.straddling += 1,
            Some(PlaneSpan::Coplanar) => self.coplanar += 1,
            None => self.overflow += 1,
        }
    }
}

/// Every representable point in the box agrees with the box's class.
fn check_inside(plane: FixedPlane3, span: Option<PlaneSpan>, p: FixedVec3) {
    let Some(span) = span else { return };
    let side = plane.classify(p);
    let ok = match span {
        PlaneSpan::Front => matches!(side, Some(PlaneSide::Front | PlaneSide::On)),
        PlaneSpan::Behind => matches!(side, Some(PlaneSide::Behind | PlaneSide::On)),
        PlaneSpan::Coplanar => side == Some(PlaneSide::On),
        PlaneSpan::Straddling => side.is_some(),
    };
    assert!(ok, "interior point {p:?} is {side:?} in a {span:?} box");
}

fn sweep_boxes(rng: &mut Xorshift, modes: &[u64], count: u32, seed: u64) -> Tally {
    let mut tally = Tally::default();
    for case in 0..count {
        let mode = modes[usize::try_from(case).unwrap() % modes.len()];
        let plane = rng.plane(mode);
        let b = FixedAabb3::new(rng.point(mode), rng.point(mode));
        let got = plane.classify_aabb(b);
        let context = format!("seed {seed:#x} case {case}: {plane:?} {b:?}");
        assert_eq!(got, classify_points(plane, &corners(b)), "eight corners, {context}");
        assert_eq!(got, model_points(plane, &corners(b)), "model, {context}");
        for _ in 0..4 {
            check_inside(plane, got, rng.inside(b));
        }
        tally.add(got);
    }
    tally
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_034_seeded_boxes_match_the_eight_corner_definition() {
    let mut rng = Xorshift(BOX_SEED);
    let tally = sweep_boxes(&mut rng, &[0, 1], 20_000, BOX_SEED);
    let enough = tally.front > 1000
        && tally.behind > 1000
        && tally.straddling > 1000
        && tally.coplanar > 100;
    assert!(enough, "seed {BOX_SEED:#x}: thin coverage {tally:?}");
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_035_boxes_at_the_ends_of_the_range_match_the_eight_corner_definition() {
    let mut rng = Xorshift(EXTREME_SEED);
    let tally = sweep_boxes(&mut rng, &[2], 50_000, EXTREME_SEED);
    let defined = tally.front + tally.behind + tally.straddling + tally.coplanar;
    assert!(tally.overflow > 5000 && defined > 5000, "seed {EXTREME_SEED:#x}: {tally:?}");
}

fn check_triangle(plane: FixedPlane3, t: [FixedVec3; 3]) -> Option<PlaneSpan> {
    let got = plane.classify_triangle(t);
    assert_eq!(got, classify_points(plane, &t), "vertices {plane:?} {t:?}");
    assert_eq!(got, model_points(plane, &t), "model {plane:?} {t:?}");
    let mut out = [pt(0, 0, 0); 4];
    let clip = plane.clip_polygon(&t, &mut out);
    let agrees = match got {
        Some(PlaneSpan::Front | PlaneSpan::Coplanar) => {
            clip == Ok(PolygonClip::Kept(3)) && out[..3] == t[..]
        }
        Some(PlaneSpan::Behind) => clip == Ok(PolygonClip::Outside),
        Some(PlaneSpan::Straddling) => {
            matches!(clip, Ok(PolygonClip::Kept(3 | 4)) | Err(PolygonClipError::Overflow))
        }
        None => clip == Err(PolygonClipError::Overflow),
    };
    assert!(agrees, "{got:?} against clip_polygon {clip:?}: {plane:?} {t:?}");
    got
}

/// Touching from behind keeps the touching point as a degenerate segment.
fn behind_clip(plane: FixedPlane3, s: FixedSegment3) -> SegmentClip {
    match (plane.classify(s.start), plane.classify(s.end)) {
        (Some(PlaneSide::On), _) => SegmentClip::Kept(FixedSegment3::new(s.start, s.start)),
        (_, Some(PlaneSide::On)) => SegmentClip::Kept(FixedSegment3::new(s.end, s.end)),
        _ => SegmentClip::Outside,
    }
}

fn check_segment(plane: FixedPlane3, s: FixedSegment3) {
    let got = plane.classify_segment(s);
    assert_eq!(got, classify_points(plane, &[s.start, s.end]), "endpoints {plane:?} {s:?}");
    assert_eq!(got, model_points(plane, &[s.start, s.end]), "model {plane:?} {s:?}");
    let clip = plane.clip_segment(s);
    let agrees = match got {
        Some(PlaneSpan::Front | PlaneSpan::Coplanar) => clip == Some(SegmentClip::Kept(s)),
        Some(PlaneSpan::Behind) => clip == Some(behind_clip(plane, s)),
        Some(PlaneSpan::Straddling) => matches!(clip, Some(SegmentClip::Kept(_)) | None),
        None => clip.is_none(),
    };
    assert!(agrees, "{got:?} against clip_segment {clip:?}: {plane:?} {s:?}");
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_036_seeded_triangles_and_segments_match_their_vertices() {
    let mut rng = Xorshift(SHAPE_SEED);
    let mut tally = Tally::default();
    for case in 0..30_000_u64 {
        let mode = case % 3;
        let plane = rng.plane(mode);
        let t = [rng.point(mode), rng.point(mode), rng.point(mode)];
        tally.add(check_triangle(plane, t));
        check_segment(plane, FixedSegment3::new(t[0], t[1]));
    }
    let enough = tally.front > 1000
        && tally.behind > 1000
        && tally.straddling > 1000
        && tally.coplanar > 100
        && tally.overflow > 1000;
    assert!(enough, "seed {SHAPE_SEED:#x}: thin coverage {tally:?}");
}
