//! The 0.7.1 code of `SurfaceHit::new` and `ShadingFrame::cosine_direction`, retained as the
//! reference for the cross-version differential case `OPTICS-TP-061`.
//!
//! The arithmetic is the released code's, step for step. It is rewritten over the public API
//! (the triangle's vertices instead of the private plane, the frame's getters instead of its
//! private fields), and it names which check refused an input, so the test can tell whether the
//! typed refusal of the new code is the right one. Do not edit it to follow a change in the
//! crate: a deliberate change to these outputs retires or re-scopes `OPTICS-TP-062` instead.
//!
//! One departure: the 0.7.1 plane test overflows `i128` for a hit point far outside the trace
//! domain (a panic in a debug build, a wrapped comparison in a release build, which then refuses
//! the point as outside the domain either way). The reference checks that arithmetic and reports
//! [`HitRefusal::Overflow`], where 0.7.1 release builds returned `None`.

use bunny_linalg::FixedVec3;
use bunny_num::{FixedAngle, FixedQ32_32};
use bunny_optics::{point_at, ShadingFrame, TriangleSurface};
use bunny_query::{TraceRay, TraceTriangle, TriangleHit, TRACE_COORD_LIMIT_RAW};

type Wide = [i128; 3];

const ONE: i128 = 1 << 32;
const ONE_RAW: i64 = 1 << 32;

/// Which check of the 0.7.1 code refused a surface hit, with the values it saw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HitRefusal {
    /// `point_at` returned `None` for this distance.
    Unrepresentable(FixedQ32_32),
    /// The exact plane test overflowed `i128` for this hit point.
    Overflow(FixedVec3),
    /// No step reached the arrival side from this point.
    NotOnArrivalSide(FixedVec3),
    /// The stepped start point (each coordinate clamped to the `i64` raw range) is outside.
    Outside(FixedVec3),
}

/// The 0.7.1 `SurfaceHit::new`: the start point and the facing frame, or the check that refused.
pub(crate) fn surface_hit(
    surface: &TriangleSurface,
    triangle: &TraceTriangle,
    ray: &TraceRay,
    hit: &TriangleHit,
) -> Result<(FixedVec3, ShadingFrame), HitRefusal> {
    let [v0, v1, v2] = triangle.vertices().map(wide);
    let normal = cross(sub(v1, v0), sub(v2, v0));
    let sign = if hit.front_face { 1 } else { -1 };
    let plane = normal.map(|c| sign * c);
    let offset = sign * dot(normal, v0);
    let start = point_at(ray, hit.distance).ok_or(HitRefusal::Unrepresentable(hit.distance))?;
    let step = dominant_step(plane);
    let mut stepped = None;
    for k in 0..=2 {
        let point = [0, 1, 2].map(|i| wide(start)[i] + k * step[i]);
        if checked_dot(plane, point).ok_or(HitRefusal::Overflow(start))? > offset {
            stepped = Some(point);
            break;
        }
    }
    let point = stepped.ok_or(HitRefusal::NotOnArrivalSide(start))?;
    let limit = u128::from(TRACE_COORD_LIMIT_RAW.unsigned_abs());
    if point.iter().all(|c| c.unsigned_abs() < limit) {
        Ok((narrow(point), surface.facing_frame(hit.front_face)))
    } else {
        Err(HitRefusal::Outside(narrow(point)))
    }
}

/// The 0.7.1 `ShadingFrame::cosine_direction`.
pub(crate) fn cosine_direction(
    frame: &ShadingFrame,
    sample: [FixedQ32_32; 2],
) -> Option<FixedVec3> {
    let [u, v] = sample;
    let inside = |c: FixedQ32_32| (0..ONE_RAW).contains(&c.raw());
    if !inside(u) || !inside(v) {
        return None;
    }
    let radius = i128::from(u.sqrt()?.raw());
    let height = i128::from(FixedQ32_32::from_raw(ONE_RAW - u.raw()).sqrt()?.raw());
    let (sin, cos) = FixedAngle::from_turns(v).sin_cos();
    let weights = [radius * i128::from(cos.raw()), radius * i128::from(sin.raw()), height * ONE];
    let columns = [frame.tangent(), frame.bitangent(), frame.normal()].map(wide);
    let axis = |i: usize| {
        let column = [columns[0][i], columns[1][i], columns[2][i]];
        round_shift(dot(weights, column), 64).clamp(-ONE, ONE)
    };
    Some(narrow([axis(0), axis(1), axis(2)]))
}

fn wide(v: FixedVec3) -> Wide {
    [v.x, v.y, v.z].map(|c| i128::from(c.raw()))
}

/// Each component clamped to `-i64::MAX..=i64::MAX`, as the 0.7.1 code narrows.
fn narrow(v: Wide) -> FixedVec3 {
    let max = i128::from(i64::MAX);
    let [x, y, z] = v.map(|c| {
        FixedQ32_32::from_raw(i64::try_from(c.clamp(-max, max)).expect("clamped into i64"))
    });
    FixedVec3::new(x, y, z)
}

fn sub(a: Wide, b: Wide) -> Wide {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: Wide, b: Wide) -> i128 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn checked_dot(a: Wide, b: Wide) -> Option<i128> {
    let term = |i: usize| a[i].checked_mul(b[i]);
    term(0)?.checked_add(term(1)?)?.checked_add(term(2)?)
}

fn cross(a: Wide, b: Wide) -> Wide {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn dominant_step(plane: Wide) -> Wide {
    let [x, y, z] = plane.map(i128::unsigned_abs);
    let axis = if x >= y && x >= z {
        0
    } else if y >= z {
        1
    } else {
        2
    };
    [0, 1, 2].map(|i| if i == axis { plane[i].signum() } else { 0 })
}

/// `value / 2^shift` rounded to nearest, ties to even.
fn round_shift(value: i128, shift: u32) -> i128 {
    let floor = value >> shift;
    let remainder = value - (floor << shift);
    let half = 1 << (shift - 1);
    if remainder > half || (remainder == half && floor & 1 == 1) {
        floor + 1
    } else {
        floor
    }
}
