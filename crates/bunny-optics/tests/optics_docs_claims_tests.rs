//! Pinning tests for claims the optics chapter makes about facing, `SurfaceHit::new` and the
//! sine table behind the cosine-weighted bounce (flyingrobots/bunny#299). Each passes on the
//! code as released: they document behaviour, they do not change it.

use bunny_linalg::FixedVec3;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_optics::{SurfaceHit, TriangleSurface};
use bunny_query::{TraceRay, TraceTriangle};
use wasm_bindgen_test::wasm_bindgen_test;

const ONE: i64 = ONE_RAW;
const MAX: FixedQ32_32 = FixedQ32_32::from_raw(i64::MAX);

type Raw3 = [i64; 3];

fn fixed(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

fn vec3(raw: Raw3) -> FixedVec3 {
    FixedVec3::new(fixed(raw[0]), fixed(raw[1]), fixed(raw[2]))
}

fn triangle(v: [Raw3; 3]) -> TraceTriangle {
    TraceTriangle::new(vec3(v[0]), vec3(v[1]), vec3(v[2])).expect("in the trace domain")
}

fn ray(origin: Raw3, direction: Raw3) -> TraceRay {
    TraceRay::new(vec3(origin), vec3(direction)).expect("a valid trace ray")
}

fn sub(a: Raw3, b: Raw3) -> [i128; 3] {
    [0, 1, 2].map(|i| i128::from(a[i]) - i128::from(b[i]))
}

/// The exact plane normal `(v1 - v0) x (v2 - v0)`, in `i128`.
fn plane_normal(v: [Raw3; 3]) -> [i128; 3] {
    let (a, b) = (sub(v[1], v[0]), sub(v[2], v[0]));
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn dot(a: [i128; 3], b: [i128; 3]) -> i128 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// The claim of the chapter: the facing guarantee is about the exact plane normal. For the
/// rounded unit normal of the frame it can fail at a grazing angle. The witness is the one in
/// flyingrobots/bunny#299, found by search.
#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_270_the_facing_sign_is_exact_for_the_plane_normal_not_the_rounded_unit_normal() {
    let vertices = [
        [-410_258_444_763, 272_959_634_540, -195_780_224_615],
        [-379_872_021_676, 121_057_362_350, -20_894_480_519],
        [-268_912_502_372, 224_709_765_362, 55_166_427_270],
    ];
    let origin = [-321_740_529_943, 279_686_771_352, -70_193_295_445];
    let direction = [474_787_860, -2_373_473_004, 2_732_589_751];
    let triangle = triangle(vertices);
    let ray = ray(origin, direction);
    let hit = ray.hit_triangle(&triangle, fixed(0), MAX).expect("the witness ray hits");
    assert!(hit.front_face);
    let surface = TriangleSurface::new(&triangle).expect("not degenerate");
    let start = SurfaceHit::new(&surface, &ray, &hit).expect("a hit the kernel reported");

    // The exact test: d . N < 0 on the front face (about -5.8e21 raw^2 for this witness).
    let exact = dot(direction.map(i128::from), plane_normal(vertices));
    assert!(exact < 0, "exact d . N = {exact}");
    assert!((-6_000_000_000_000_000_000_000..-5_500_000_000_000_000_000_000).contains(&exact));

    // The frame normal is the rounded unit normal; its dot with the ray direction is positive.
    let n = start.frame().normal();
    let rounded = dot(direction.map(i128::from), [n.x.raw(), n.y.raw(), n.z.raw()].map(i128::from));
    assert_eq!(rounded, 65_823_548, "d . n in raw^2 with the rounded unit normal");
}

/// `SurfaceHit::new` refuses a hit whose point cannot reach the arrival side of the exact
/// plane, not every hit the kernel did not report: a changed distance that still lands on the
/// arrival side is accepted, and the start point follows the changed distance.
#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_271_a_changed_distance_is_refused_only_off_the_arrival_side() {
    let ground = [[-8 * ONE, 0, 8 * ONE], [8 * ONE, 0, 8 * ONE], [0, 0, -8 * ONE]];
    let triangle = triangle(ground);
    let surface = TriangleSurface::new(&triangle).expect("not degenerate");
    let ray = ray([0, 2 * ONE, 0], [0, -ONE, 0]);
    let mut hit = ray.hit_triangle(&triangle, fixed(0), MAX).expect("hits the ground");
    assert_eq!(hit.distance, fixed(2 * ONE));
    let reported = SurfaceHit::new(&surface, &ray, &hit).expect("the reported hit");
    assert_eq!(reported.point(), vec3([0, 1, 0]), "one raw unit above the plane");

    // Distance 1.0: the point is 1.0 above the ground, on the arrival side. Accepted.
    hit.distance = fixed(ONE);
    let moved = SurfaceHit::new(&surface, &ray, &hit).expect("on the arrival side");
    assert_eq!(moved.point(), vec3([0, ONE, 0]));
    assert_eq!(SurfaceHit::try_new(&surface, &ray, &hit), Ok(moved));

    // Distance 3.0: the point is 1.0 below the ground, two steps cannot reach. Refused.
    hit.distance = fixed(3 * ONE);
    assert_eq!(SurfaceHit::new(&surface, &ray, &hit), None);
}

/// A regression pin on the bits the bounce direction takes from `FixedAngle::sin_cos`: the
/// direction about the +Y frame normal for every one of 4 097 turn fractions `k / 4096` (each
/// knot of the sine table, with the segment between knots in `sin_cos`'s interpolation) at
/// four `u` values, folded by FNV-1a. This is not an independent oracle (the error bound of
/// `sin_cos` is `AT-TP-004`); it fails when a table entry changes any output bit here.
#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_272_the_bounce_direction_pins_every_knot_of_the_sine_table() {
    let ground = [[-8 * ONE, 0, 8 * ONE], [8 * ONE, 0, 8 * ONE], [0, 0, -8 * ONE]];
    let surface = TriangleSurface::new(&triangle(ground)).expect("not degenerate");
    let frame = surface.frame();
    let mut digest: u64 = 0xCBF2_9CE4_8422_2325;
    let mut cases = 0_u32;
    for u in [0, ONE / 4, ONE / 2, ONE - 1] {
        for k in 0..=4096_i64 {
            let v = k * (ONE / 4096);
            let d = frame
                .cosine_direction([fixed(u), fixed(v.min(ONE - 1))])
                .expect("a sample inside [0, 1)");
            for raw in [d.x.raw(), d.y.raw(), d.z.raw()] {
                for byte in raw.to_le_bytes() {
                    digest = (digest ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01B3);
                }
            }
            cases += 1;
        }
    }
    assert_eq!(cases, 16_388);
    assert_eq!(digest, 0x67F149E2DD7352C3, "pinned digest");
}
