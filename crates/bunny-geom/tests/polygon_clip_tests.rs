//! Convex polygon clipping tests.

use bunny_geom::{FixedPlane3, FixedSegment3, PolygonClip, PolygonClipError, SegmentClip};
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

/// The plane `x = 1`, front is `x > 1`.
fn plane_x1() -> FixedPlane3 {
    FixedPlane3::new(pt(UNIT, 0, 0), raw(UNIT))
}

/// A point in the `z = 0` plane on the grid of whole units.
fn xy(x: i64, y: i64) -> FixedVec3 {
    pt(x * UNIT, y * UNIT, 0)
}

fn clip(plane: FixedPlane3, polygon: &[FixedVec3]) -> Vec<FixedVec3> {
    let mut out = vec![pt(7, 7, 7); polygon.len() + 1];
    match plane.clip_polygon(polygon, &mut out) {
        Ok(PolygonClip::Kept(len)) => out[..len].to_vec(),
        Ok(PolygonClip::Outside) => Vec::new(),
        Err(e) => panic!("unexpected error {e:?}"),
    }
}

fn square() -> [FixedVec3; 4] {
    [xy(0, 0), xy(4, 0), xy(4, 4), xy(0, 4)]
}

/// Twice the signed area of the `xy` projection (positive is counter-clockwise).
fn area2(poly: &[FixedVec3]) -> i128 {
    let next = poly.iter().cycle().skip(1);
    poly.iter()
        .zip(next)
        .map(|(a, b)| {
            i128::from(a.x.raw()) * i128::from(b.y.raw())
                - i128::from(b.x.raw()) * i128::from(a.y.raw())
        })
        .sum()
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_010_square_crossing_starts_at_first_kept_vertex() {
    // Vertex 0 and 3 are behind x = 1; the first kept vertex is 1.
    let out = clip(plane_x1(), &square());
    assert_eq!(out, vec![xy(4, 0), xy(4, 4), xy(1, 4), xy(1, 0)]);
    assert!(area2(&out) > 0, "counter-clockwise winding is preserved");
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_011_one_vertex_on_the_plane() {
    let plane = plane_x1();
    // Apex on the plane, base behind: only a point touches.
    assert!(clip(plane, &[xy(1, 0), xy(0, 1), xy(0, -1)]).is_empty());
    // Apex on the plane, base in front: unchanged.
    let front = [xy(1, 0), xy(2, -1), xy(2, 1)];
    assert_eq!(clip(plane, &front), front.to_vec());
    // Vertex on the plane between a behind and a front neighbor: no duplicate cut.
    let tri = [xy(1, 0), xy(3, 0), xy(-1, 2)];
    assert_eq!(clip(plane, &tri), vec![xy(1, 0), xy(3, 0), xy(1, 1)]);
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_012_all_in_and_all_out() {
    let plane = plane_x1();
    let front = [xy(2, 0), xy(5, 1), xy(3, 4), xy(2, 3)];
    assert_eq!(clip(plane, &front), front.to_vec());
    let behind = [xy(-2, 0), xy(0, 1), xy(0, 4)];
    assert!(clip(plane, &behind).is_empty());
    let almost = [xy(-2, 0), pt(UNIT - 1, 0, 0), xy(0, 4)];
    assert!(clip(plane, &almost).is_empty());
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_013_edge_in_the_plane() {
    let plane = plane_x1();
    // One edge on the plane, rest behind: a segment, not a polygon.
    assert!(clip(plane, &[xy(1, 0), xy(1, 4), xy(0, 4), xy(0, 0)]).is_empty());
    // One edge on the plane, rest in front: unchanged, starting at vertex 0.
    let front = [xy(1, 0), xy(1, 4), xy(3, 4), xy(3, 0)];
    assert_eq!(clip(plane, &front), front.to_vec());
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_014_coplanar_polygon_is_unchanged() {
    let plane = FixedPlane3::new(pt(0, 0, UNIT), raw(0));
    let poly = square();
    assert_eq!(clip(plane, &poly), poly.to_vec());
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_015_degenerate_inputs_and_repeated_vertices() {
    let plane = plane_x1();
    let mut out = [pt(0, 0, 0); 8];
    assert_eq!(plane.clip_polygon(&[], &mut out), Err(PolygonClipError::TooFewVertices));
    assert_eq!(
        plane.clip_polygon(&[xy(2, 0), xy(3, 0)], &mut out),
        Err(PolygonClipError::TooFewVertices)
    );
    // Two separate kept vertices, joined by two cuts, make a quadrilateral.
    assert_eq!(
        plane.clip_polygon(&[xy(2, 0), xy(0, 1), xy(0, -1), xy(2, 0)], &mut out),
        Ok(PolygonClip::Kept(4))
    );
    // Two on-plane vertices leave an edge, not a polygon.
    assert_eq!(
        plane.clip_polygon(&[xy(1, 0), xy(1, 0), xy(0, 3)], &mut out),
        Ok(PolygonClip::Outside)
    );
    // Repeated input vertices are not merged.
    let repeated = [xy(2, 0), xy(2, 0), xy(3, 1), xy(2, 2)];
    assert_eq!(clip(plane, &repeated), repeated.to_vec());
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_016_output_buffer_bounds() {
    let plane = plane_x1();
    // Cutting one corner of a square gives a pentagon: n + 1 vertices.
    let poly = [xy(0, 0), xy(4, 0), xy(4, 4), xy(0, 4)];
    let cut = FixedPlane3::new(pt(UNIT, UNIT, 0), raw(UNIT));
    let mut five = [pt(9, 9, 9); 6];
    assert_eq!(cut.clip_polygon(&poly, &mut five), Ok(PolygonClip::Kept(5)));
    assert_eq!(five[5], pt(9, 9, 9), "slots after the result are untouched");
    let mut tight = [pt(0, 0, 0); 5];
    assert_eq!(cut.clip_polygon(&poly, &mut tight), Ok(PolygonClip::Kept(5)));
    let mut small = [pt(0, 0, 0); 4];
    assert_eq!(cut.clip_polygon(&poly, &mut small), Err(PolygonClipError::OutputTooSmall));
    assert_eq!(plane.clip_polygon(&poly, &mut []), Err(PolygonClipError::OutputTooSmall));
    // Outside needs no output room.
    let behind = [xy(-3, 0), xy(-1, 0), xy(-2, 1)];
    assert_eq!(plane.clip_polygon(&behind, &mut []), Ok(PolygonClip::Outside));
}

fn cut_of(plane: FixedPlane3, a: FixedVec3, b: FixedVec3) -> FixedVec3 {
    match plane.clip_segment(FixedSegment3::new(a, b)) {
        Some(SegmentClip::Kept(s)) if s.start == a => s.end,
        Some(SegmentClip::Kept(s)) => s.start,
        other => panic!("expected a crossing, got {other:?}"),
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_017_shared_edge_cut_ignores_traversal_direction() {
    let plane = plane_x1();
    // Triangles over the edge a-b, traversed in opposite directions.
    let (c, d) = (pt(3 * UNIT, UNIT, 9), pt(5 * UNIT, 2 * UNIT, 11));
    let mut found_asymmetry = false;
    for k in 0..400_i64 {
        let (a, b) = (pt(k, 3 * k, 5 * k), pt(3 * UNIT + 7 * k + 5, 11 * k, 3 * UNIT - k));
        let canonical = cut_of(plane, a, b);
        assert!(clip(plane, &[a, b, c]).contains(&canonical), "crack at k = {k}");
        assert!(clip(plane, &[b, a, d]).contains(&canonical), "crack at k = {k}");
        found_asymmetry |= cut_of(plane, b, a) != canonical;
    }
    // clip_segment itself is not symmetric, which is why polygons canonicalize.
    assert!(found_asymmetry);
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_018_cut_equals_segment_clip_in_canonical_direction() {
    let plane = plane_x1();
    let (a, b) = (pt(0, 0, 0), pt(3 * UNIT, 3 * UNIT, 3 * UNIT));
    let canonical = cut_of(plane, a, b);
    let c = pt(3 * UNIT, 0, 0);
    for poly in [[a, b, c], [b, a, c], [c, b, a]] {
        assert!(clip(plane, &poly).contains(&canonical));
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_019_golden_inexact_cut() {
    // Edge a-b runs x = 0 to 3 across x = 1: t = 1/3 rounds to raw 1431655765.
    let (a, b, c) = (pt(0, 0, 0), pt(3 * UNIT, 3 * UNIT, 3 * UNIT), pt(3 * UNIT, 0, 0));
    let third = 4_294_967_295;
    assert_eq!(
        clip(plane_x1(), &[a, b, c]),
        vec![b, c, pt(third, 0, 0), pt(third, third, third)]
    );
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_020_winding_survives_both_orientations() {
    let plane = plane_x1();
    let mut cw = square();
    cw.reverse();
    let out = clip(plane, &cw);
    assert_eq!(out.len(), 4);
    assert!(area2(&out) < 0, "clockwise stays clockwise");
    assert_eq!(out, vec![xy(4, 4), xy(4, 0), xy(1, 0), xy(1, 4)]);
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_021_overflow_is_an_error() {
    let huge = FixedPlane3::new(pt(i64::MAX, 0, 0), raw(0));
    let mut out = [pt(0, 0, 0); 4];
    let poly = [pt(i64::MAX, 0, 0), xy(0, 0), xy(0, 1)];
    assert_eq!(huge.clip_polygon(&poly, &mut out), Err(PolygonClipError::Overflow));
    // The cut's interpolation overflows although both distances fit.
    let plane = FixedPlane3::new(pt(0, 0, UNIT), raw(UNIT));
    let wide = [pt(i64::MAX, 0, 2 * UNIT), pt(i64::MIN, 0, 0), pt(0, 0, 3 * UNIT)];
    assert_eq!(plane.clip_polygon(&wide, &mut out), Err(PolygonClipError::Overflow));
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
}

/// A convex polygon: points on a parabola in ascending order, closed by a chord,
/// with axes permuted into 3D.
fn convex_polygon(rng: &mut Xorshift) -> Vec<FixedVec3> {
    let count = 3 + (rng.next() % 6) as usize;
    let mut xs: Vec<i64> = (0..count as i64 * 3).collect();
    for i in (1..xs.len()).rev() {
        xs.swap(i, (rng.next() % (i as u64 + 1)) as usize);
    }
    xs.truncate(count);
    xs.sort_unstable();
    let spin = rng.next() % 3;
    xs.iter()
        .map(|&x| {
            let (u, v) = ((x - 6) * UNIT / 4, (x - 6) * (x - 6) * UNIT / 16);
            match spin {
                0 => pt(u, v, 0),
                1 => pt(0, u, v),
                _ => pt(v, 0, u),
            }
        })
        .collect()
}

fn plane_through(rng: &mut Xorshift, poly: &[FixedVec3]) -> FixedPlane3 {
    let normal = pt(rng.range(2 * UNIT), rng.range(2 * UNIT), rng.range(2 * UNIT));
    let anchor = poly[(rng.next() % poly.len() as u64) as usize];
    let base = FixedPlane3::new(normal, raw(0)).signed_distance(anchor).unwrap();
    FixedPlane3::new(normal, raw(base.raw() + rng.range(UNIT)))
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

fn assert_cut_close(plane: FixedPlane3, a: FixedVec3, b: FixedVec3, got: FixedVec3) {
    let key = |p: FixedVec3| (p.x.raw(), p.y.raw(), p.z.raw());
    let (lo, hi) = if key(a) <= key(b) { (a, b) } else { (b, a) };
    let d = (dist(plane, lo), dist(plane, hi));
    assert_axis_close((lo.x.raw(), hi.x.raw()), d, got.x.raw());
    assert_axis_close((lo.y.raw(), hi.y.raw()), d, got.y.raw());
    assert_axis_close((lo.z.raw(), hi.z.raw()), d, got.z.raw());
}

/// Independent Sutherland-Hodgman: expected exact vertices and cut edges.
fn expected(plane: FixedPlane3, poly: &[FixedVec3]) -> Vec<(FixedVec3, Option<FixedVec3>)> {
    let d: Vec<i64> = poly.iter().map(|p| dist(plane, *p)).collect();
    let Some(first) = d.iter().position(|&v| v >= 0) else { return Vec::new() };
    let n = poly.len();
    let mut list = Vec::new();
    for j in 0..n {
        let (i, k) = ((first + j) % n, (first + j + 1) % n);
        if d[i] >= 0 {
            list.push((poly[i], None));
        }
        if (d[i] >= 0) != (d[k] >= 0) && d[i] != 0 && d[k] != 0 {
            list.push((poly[i], Some(poly[k])));
        }
    }
    list
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_022_seeded_sweep_matches_reference() {
    let mut rng = Xorshift(0x2545_F491_4F6C_DD1D);
    let mut kept_polygons = 0;
    for _ in 0..2000 {
        let poly = convex_polygon(&mut rng);
        let plane = plane_through(&mut rng, &poly);
        let want = expected(plane, &poly);
        let got = clip(plane, &poly);
        assert!(got.len() <= poly.len() + 1);
        if want.len() < 3 {
            assert!(got.is_empty());
            continue;
        }
        kept_polygons += 1;
        assert_eq!(got.len(), want.len());
        for ((vertex, edge_end), out) in want.iter().zip(&got) {
            match edge_end {
                None => assert_eq!(out, vertex),
                Some(end) => assert_cut_close(plane, *vertex, *end, *out),
            }
        }
        // Every output vertex is kept, up to the documented rounding of cuts.
        assert!(got.iter().all(|p| dist(plane, *p) >= -(1 << 12)));
    }
    assert!(kept_polygons > 500, "the sweep must exercise real clips");
}

#[wasm_bindgen_test(unsupported = test)]
fn pc_tp_023_sweep_reversed_polygon_gives_the_same_points() {
    let mut rng = Xorshift(0x1234_5678_9ABC_DEF1);
    for _ in 0..1000 {
        let poly = convex_polygon(&mut rng);
        let plane = plane_through(&mut rng, &poly);
        let mut rev = poly.clone();
        rev.reverse();
        let key = |p: &FixedVec3| (p.x.raw(), p.y.raw(), p.z.raw());
        let (mut a, mut b) = (clip(plane, &poly), clip(plane, &rev));
        a.sort_by_key(key);
        b.sort_by_key(key);
        assert_eq!(a, b);
    }
}
