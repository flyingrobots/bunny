//! Plane classification and segment clipping tests.

use bunny_geom::{FixedPlane3, FixedSegment3, PlaneSide, SegmentClip};
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use wasm_bindgen_test::wasm_bindgen_test;

const UNIT: i64 = 1_i64 << 32;

fn raw(v: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(v)
}

fn pt(x: i64, y: i64, z: i64) -> FixedVec3 {
    FixedVec3::new(raw(x), raw(y), raw(z))
}

fn seg(a: FixedVec3, b: FixedVec3) -> FixedSegment3 {
    FixedSegment3::new(a, b)
}

/// The plane `z = 1`.
fn plane_z1() -> FixedPlane3 {
    FixedPlane3::new(pt(0, 0, UNIT), raw(UNIT))
}

/// The plane `z = 0`, with raw-unit control for tie cases.
fn plane_z0() -> FixedPlane3 {
    FixedPlane3::new(pt(0, 0, UNIT), raw(0))
}

fn kept(clip: Option<SegmentClip>) -> FixedSegment3 {
    match clip {
        Some(SegmentClip::Kept(s)) => s,
        other => panic!("expected a kept segment, got {other:?}"),
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_001_classification_has_three_outcomes() {
    let plane = plane_z1();
    assert_eq!(plane.classify(pt(5, 7, 2 * UNIT)), Some(PlaneSide::Front));
    assert_eq!(plane.classify(pt(5, 7, 0)), Some(PlaneSide::Behind));
    assert_eq!(plane.classify(pt(5, 7, UNIT)), Some(PlaneSide::On));
    assert_eq!(plane.classify(pt(5, 7, UNIT + 1)), Some(PlaneSide::Front));
    assert_eq!(plane.classify(pt(5, 7, UNIT - 1)), Some(PlaneSide::Behind));

    // Non-unit normal (2, 0, 0) and offset 2: the plane x = 1.
    let scaled = FixedPlane3::new(pt(2 * UNIT, 0, 0), raw(2 * UNIT));
    assert_eq!(scaled.classify(pt(UNIT, 9, 9)), Some(PlaneSide::On));
    assert_eq!(scaled.classify(pt(UNIT + UNIT / 2, 0, 0)), Some(PlaneSide::Front));
    assert_eq!(scaled.signed_distance(pt(UNIT + UNIT / 2, 0, 0)), Some(raw(UNIT)));
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_002_crossing_segment_keeps_front_part_exactly() {
    let plane = plane_z1();
    let behind_to_front = seg(pt(0, 0, 0), pt(8 * UNIT, 4 * UNIT, 4 * UNIT));
    let clipped = kept(plane.clip_segment(behind_to_front));
    assert_eq!(clipped.start, pt(2 * UNIT, UNIT, UNIT));
    assert_eq!(clipped.end, pt(8 * UNIT, 4 * UNIT, 4 * UNIT));

    let front_to_behind = seg(pt(8 * UNIT, 4 * UNIT, 4 * UNIT), pt(0, 0, 0));
    let clipped = kept(plane.clip_segment(front_to_behind));
    assert_eq!(clipped.start, pt(8 * UNIT, 4 * UNIT, 4 * UNIT));
    assert_eq!(clipped.end, pt(2 * UNIT, UNIT, UNIT));
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_003_fully_front_is_kept_and_fully_behind_is_outside() {
    let plane = plane_z1();
    let front = seg(pt(0, 0, 2 * UNIT), pt(UNIT, 3, 5 * UNIT));
    assert_eq!(plane.clip_segment(front), Some(SegmentClip::Kept(front)));
    let behind = seg(pt(0, 0, 0), pt(UNIT, 3, UNIT - 1));
    assert_eq!(plane.clip_segment(behind), Some(SegmentClip::Outside));
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_004_touching_end_belongs_to_front() {
    let plane = plane_z1();
    let on_to_behind = seg(pt(3, 4, UNIT), pt(3, 4, 0));
    let clipped = kept(plane.clip_segment(on_to_behind));
    assert_eq!((clipped.start, clipped.end), (on_to_behind.start, on_to_behind.start));

    let behind_to_on = seg(pt(3, 4, 0), pt(3, 4, UNIT));
    let clipped = kept(plane.clip_segment(behind_to_on));
    assert_eq!((clipped.start, clipped.end), (behind_to_on.end, behind_to_on.end));

    let on_to_front = seg(pt(3, 4, UNIT), pt(3, 4, 2 * UNIT));
    assert_eq!(plane.clip_segment(on_to_front), Some(SegmentClip::Kept(on_to_front)));
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_005_coplanar_segment_is_kept() {
    let plane = plane_z1();
    let coplanar = seg(pt(-UNIT, 2, UNIT), pt(9 * UNIT, -7, UNIT));
    assert_eq!(plane.clip_segment(coplanar), Some(SegmentClip::Kept(coplanar)));
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_006_golden_third_locks_raw_outputs() {
    // z runs 0 -> 3 across the plane z = 1, so t = 1/3 and is not representable.
    // t rounds to raw 1431655765; each coordinate offset 3 * t = 4294967295.
    let plane = plane_z1();
    let clipped = kept(plane.clip_segment(seg(pt(0, 0, 0), pt(3 * UNIT, 3 * UNIT, 3 * UNIT))));
    assert_eq!(clipped.start, pt(4_294_967_295, 4_294_967_295, 4_294_967_295));
    assert_eq!(clipped.end, pt(3 * UNIT, 3 * UNIT, 3 * UNIT));
    // The endpoint is one raw unit behind the plane: the documented bound, not repaired.
    assert_eq!(plane.classify(clipped.start), Some(PlaneSide::Behind));
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_007_parameter_ties_round_to_even() {
    // da = 3 raw, da - db = 2^33 raw: t = 1.5 raw, ties-to-even gives 2.
    // Offset (-2^33) * 2 >> 32 = -4, so z = 3 - 4 = -1.
    let plane = plane_z0();
    let tie_up = seg(pt(0, 0, 3), pt(0, 0, 3 - (1_i64 << 33)));
    assert_eq!(kept(plane.clip_segment(tie_up)).end.z, raw(-1));

    // da = 1 raw: t = 0.5 raw, ties-to-even gives 0, so the endpoint is the start.
    let tie_down = seg(pt(0, 0, 1), pt(0, 0, 1 - (1_i64 << 33)));
    assert_eq!(kept(plane.clip_segment(tie_down)).end.z, raw(1));
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_008_overflow_returns_none() {
    let max = i64::MAX;
    let huge = FixedPlane3::new(pt(max, 0, 0), raw(0));
    assert_eq!(huge.signed_distance(pt(max, 0, 0)), None);
    assert_eq!(huge.classify(pt(max, 0, 0)), None);
    assert_eq!(huge.clip_segment(seg(pt(max, 0, 0), pt(0, 0, 0))), None);

    // The interpolation offset (b - a) overflows although both distances fit.
    let plane = plane_z1();
    let wide = seg(pt(max, 0, 2 * UNIT), pt(i64::MIN, 0, 0));
    assert_eq!(plane.clip_segment(wide), None);
}

struct Xorshift(u64);

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

    fn point(&mut self) -> FixedVec3 {
        let limit = 1000 * UNIT;
        pt(self.range(limit), self.range(limit), self.range(limit))
    }

    fn plane(&mut self) -> FixedPlane3 {
        let n = pt(self.range(2 * UNIT), self.range(2 * UNIT), self.range(2 * UNIT));
        FixedPlane3::new(n, raw(self.range(1000 * UNIT)))
    }
}

fn dist(plane: FixedPlane3, p: FixedVec3) -> i64 {
    plane.signed_distance(p).unwrap().raw()
}

fn nearest_div(num: i128, den: i128) -> i128 {
    let half = den / 2;
    if num >= 0 {
        (num + half) / den
    } else {
        -((-num + half) / den)
    }
}

fn assert_axis_close((a, b): (i64, i64), (da, db): (i64, i64), got: i64) {
    let delta = i128::from(b) - i128::from(a);
    let den = i128::from(da) - i128::from(db);
    let exact = i128::from(a) + nearest_div(delta * i128::from(da), den);
    let bound = (delta.abs() >> 33) + 2;
    assert!((i128::from(got) - exact).abs() <= bound, "got {got} exact {exact} bound {bound}");
}

fn check_crossing(plane: FixedPlane3, s: FixedSegment3) {
    let (da, db) = (dist(plane, s.start), dist(plane, s.end));
    let clipped = kept(plane.clip_segment(s));
    let (front, cut) =
        if da >= 0 { (clipped.start, clipped.end) } else { (clipped.end, clipped.start) };
    assert_eq!(front, if da >= 0 { s.start } else { s.end });
    assert_axis_close((s.start.x.raw(), s.end.x.raw()), (da, db), cut.x.raw());
    assert_axis_close((s.start.y.raw(), s.end.y.raw()), (da, db), cut.y.raw());
    assert_axis_close((s.start.z.raw(), s.end.z.raw()), (da, db), cut.z.raw());
}

fn check_segment(plane: FixedPlane3, s: FixedSegment3) {
    let (da, db) = (dist(plane, s.start), dist(plane, s.end));
    assert_eq!(plane.classify(s.start).map(side_is_front), Some(da >= 0));
    match (da >= 0, db >= 0) {
        (true, true) => assert_eq!(plane.clip_segment(s), Some(SegmentClip::Kept(s))),
        (false, false) => assert_eq!(plane.clip_segment(s), Some(SegmentClip::Outside)),
        _ => check_crossing(plane, s),
    }
}

const fn side_is_front(side: PlaneSide) -> bool {
    !matches!(side, PlaneSide::Behind)
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_009_seeded_sweep_matches_exact_reference() {
    let mut rng = Xorshift(0x9E37_79B9_7F4A_7C15);
    for _ in 0..2000 {
        let plane = rng.plane();
        check_segment(plane, seg(rng.point(), rng.point()));
    }
}
