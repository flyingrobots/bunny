//! Evidence for `bunny-smooth-shading/1`: which frame a smooth-shaded hit uses (shading normal,
//! flipped, or the geometric fallback), a grazing case where only the exact plane gives the right
//! side, and the light term with the shading cosine and the geometric shadow test.

use bunny_linalg::{FixedUnitVec3, FixedVec3};
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_optics::{
    smooth_normal, DirectionalLight, ShadingFrame, SmoothBounce, SmoothHit, SurfaceHit,
    TriangleSurface, SMOOTH_SHADING_ID,
};
use bunny_query::{TraceRay, TraceTriangle};
use wasm_bindgen_test::wasm_bindgen_test;

const ONE: i64 = ONE_RAW;
const FAR: FixedQ32_32 = FixedQ32_32::from_raw(i64::MAX);

type Raw3 = [i64; 3];

fn vec3(raw: Raw3) -> FixedVec3 {
    let [x, y, z] = raw.map(FixedQ32_32::from_raw);
    FixedVec3::new(x, y, z)
}

fn unit(raw: Raw3) -> FixedUnitVec3 {
    FixedUnitVec3::try_from_unit(vec3(raw)).expect("a unit vector")
}

const fn u(whole: i64) -> i64 {
    whole * ONE
}

/// The ground `y = 0`, front face up.
const GROUND: [Raw3; 3] = [[u(-8), 0, u(8)], [u(8), 0, u(8)], [0, 0, u(-8)]];
/// `(0.6, 0.8, 0)` rounded: tilted 37 degrees from `+Y` toward `+X`.
const TILTED: Raw3 = [2_576_980_378, 3_435_973_837, 0];
const X: Raw3 = [ONE, 0, 0];
const NEG_X: Raw3 = [-ONE, 0, 0];
const Y: Raw3 = [0, ONE, 0];

/// The surface hit and the smooth hit of a ray from `origin` along `direction` on `triangle`.
fn shade(
    triangle: [Raw3; 3],
    origin: Raw3,
    direction: Raw3,
    normals: [Option<Raw3>; 3],
) -> (SurfaceHit, SmoothHit) {
    let triangle = TraceTriangle::new(vec3(triangle[0]), vec3(triangle[1]), vec3(triangle[2]))
        .expect("in the trace domain");
    let surface = TriangleSurface::new(&triangle).expect("not degenerate");
    let ray = TraceRay::new(vec3(origin), vec3(direction)).expect("a trace ray");
    let hit = ray.hit_triangle(&triangle, FixedQ32_32::ZERO, FAR).expect("a hit");
    let surface_hit = SurfaceHit::new(&surface, &ray, &hit).expect("a surface hit");
    let smooth =
        SmoothHit::new(surface_hit, hit.barycentric_numerators(), normals.map(|n| n.map(unit)));
    (surface_hit, smooth)
}

fn ground(from_above: bool, normals: [Option<Raw3>; 3]) -> (SurfaceHit, SmoothHit) {
    let (y, dy) = if from_above { (u(2), -ONE) } else { (u(-2), ONE) };
    shade(GROUND, [0, y, 0], [0, dy, 0], normals)
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_079_the_shading_frame_faces_the_arrival_side_or_falls_back() {
    assert_eq!(SMOOTH_SHADING_ID, "bunny-smooth-shading/1");
    let tilted = Some(TILTED);
    // Front face: n . N > 0, the frame around n; the surface hit is unchanged.
    let (hit, smooth) = ground(true, [tilted; 3]);
    assert_eq!(smooth.frame(), ShadingFrame::new(unit(TILTED)));
    assert_eq!(smooth.surface_hit(), hit);
    // Back face: the facing normal is -N, n . -N < 0, so the frame is flipped.
    let (_, smooth) = ground(false, [tilted; 3]);
    assert_eq!(smooth.frame(), ShadingFrame::new(unit(TILTED)).flipped());
    assert_eq!(smooth.frame().normal(), vec3(TILTED.map(|c| -c)));
    // A normal along the surface (n . N = 0 exactly), a missing vertex normal, and normals that
    // cancel at the hit (X and -X at the midpoint of their edge) fall back to the geometric frame.
    for normals in [[Some(X); 3], [tilted, None, tilted]] {
        let (hit, smooth) = ground(true, normals);
        assert_eq!(smooth.frame(), hit.frame(), "{normals:?}");
    }
    let (hit, smooth) =
        shade(GROUND, [0, u(2), u(8)], [0, -ONE, 0], [Some(X), Some(NEG_X), Some(Y)]);
    assert_eq!(smooth.frame(), hit.frame());
    // Distinct normals: the frame around smooth_normal at the hit's own weights.
    let (_, smooth) = ground(true, [tilted, Some(Y), Some(X)]);
    let triangle =
        TraceTriangle::new(vec3(GROUND[0]), vec3(GROUND[1]), vec3(GROUND[2])).expect("tri");
    let ray = TraceRay::new(vec3([0, u(2), 0]), vec3([0, -ONE, 0])).expect("ray");
    let weights =
        ray.hit_triangle(&triangle, FixedQ32_32::ZERO, FAR).expect("hit").barycentric_numerators();
    let expected = smooth_normal([TILTED, Y, X].map(unit), weights).expect("a normal");
    assert_eq!(smooth.frame(), ShadingFrame::new(expected));
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_080_a_grazing_shading_normal_is_sided_by_the_exact_plane() {
    // A front-face hit on a triangle whose rounded unit normal g is off its exact normal N by up
    // to a raw unit; the shared vertex normal n is nearly along the surface, with n . N < 0
    // (exact: -142 276 829 487 961 620 915) but n . g > 0 (1 191 621 077), found by search. Only
    // the exact plane flips it.
    let triangle = [
        [14_884_853_697, -1_772_168_654, 15_083_532_738],
        [-5_184_124_845, 13_289_159_059, -7_286_836_095],
        [12_808_423_220, 17_028_505_016, -10_615_521_610],
    ];
    let n: Raw3 = [-3_075_239_551, -1_918_531_781, 2_304_101_036];
    // The ray runs against N, about (33.5, -469.3, -346.0) * 10^18 raw^2, through the centroid.
    let direction = [-(ONE / 14), ONE, ONE * 346 / 469];
    let centroid: Raw3 = [0, 1, 2].map(|i| triangle.iter().map(|v| v[i]).sum::<i64>() / 3);
    let origin = [0, 1, 2].map(|i| centroid[i] - 2 * direction[i]);
    let (hit, smooth) = shade(triangle, origin, direction, [Some(n); 3]);
    assert_eq!(
        hit.frame(),
        TriangleSurface::new(
            &TraceTriangle::new(vec3(triangle[0]), vec3(triangle[1]), vec3(triangle[2]))
                .expect("tri"),
        )
        .expect("surface")
        .frame(),
        "a front-face hit"
    );
    let g = hit.frame().normal();
    let rounded: i128 =
        [g.x, g.y, g.z].iter().zip(n).map(|(a, b)| i128::from(a.raw()) * i128::from(b)).sum();
    assert_eq!(rounded, 1_191_621_077);
    assert_eq!(smooth.frame(), ShadingFrame::new(unit(n)).flipped());
}

fn light(toward: Raw3) -> DirectionalLight {
    DirectionalLight::new(vec3(toward), [FixedQ32_32::from_raw(2 * ONE); 3]).expect("a light")
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_081_the_light_uses_the_shading_cosine_and_the_geometric_side() {
    let (hit, smooth) = ground(true, [Some(TILTED); 3]);
    let zero = [FixedQ32_32::ZERO; 3];
    // Straight above: the same shadow ray, the cosine against the tilted normal (0.8).
    let above = light([0, ONE, 0]);
    let (geometric, shaded) = (hit.light_sample(&above), smooth.light_sample(&above));
    assert_eq!(shaded.shadow_ray(), geometric.shadow_ray());
    assert!(shaded.shadow_ray().is_some());
    let expected = above.reflected_radiance(above.cosine(vec3(TILTED)));
    assert_eq!(shaded.radiance(true), expected);
    // By hand: the cosine is 3 435 973 837 raw (the normal's y); times FRAC_1_PI it is
    // 1 093 704 440 + 3 709 399 947 / 2^32, rounded up to 1 093 704 441; times 2.0 exactly.
    assert_eq!(shaded.radiance(true), [FixedQ32_32::from_raw(2_187_408_882); 3]);
    assert_ne!(shaded.radiance(true), geometric.radiance(true));
    // Above the plane but behind the shading normal (n . l = -0.2): a shadow ray, no light.
    let behind = light([-ONE, ONE / 2, 0]);
    let shaded = smooth.light_sample(&behind);
    assert_eq!(shaded.shadow_ray(), hit.light_sample(&behind).shadow_ray());
    assert!(shaded.shadow_ray().is_some());
    assert_eq!(shaded.radiance(true), zero);
    // Along the plane (and below it): no shadow ray and no light, although n . l = 0.6.
    for toward in [[ONE, 0, 0], [0, -ONE, 0]] {
        let shaded = smooth.light_sample(&light(toward));
        assert!(shaded.shadow_ray().is_none(), "{toward:?}");
        assert_eq!(shaded.radiance(true), zero, "{toward:?}");
    }
}

/// A unit-sized triangle whose exact normal is `(-2^32, 0, 2^64)` raw^2, hit from `+Z` (front)
/// or `-Z` (back).
fn sloped(from_above: bool) -> SmoothHit {
    let triangle = [[0, 0, 0], [ONE, 0, 1], [0, ONE, 0]];
    let (z, dz) = if from_above { (ONE, -ONE) } else { (-ONE, ONE) };
    shade(triangle, [ONE / 4, ONE / 4, z], [0, 0, dz], [None; 3]).1
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_083_bounces_are_above_the_exact_plane_or_rejected() {
    // The ground from above: N = (0, c, 0); one raw unit of y either side of the surface.
    let (_, ground) = ground(true, [None; 3]);
    let above = |hit: &SmoothHit, d: Raw3| hit.is_above_surface(vec3(d));
    assert!(above(&ground, [ONE, 1, 0]));
    assert!(!above(&ground, [ONE, 0, 0]) && !above(&ground, [ONE, -1, 0]));
    // A sloped plane: (2^32, 0, 1) . N = 0 exactly; one raw unit of z either side.
    let (front, back) = (sloped(true), sloped(false));
    assert!(
        !above(&front, [ONE, 0, 1]) && above(&front, [ONE, 0, 2]) && !above(&front, [ONE, 0, 0])
    );
    // From the back the facing normal is -N: the zero stays rejected, the sides swap.
    assert!(!above(&back, [ONE, 0, 1]) && !above(&back, [ONE, 0, 2]) && above(&back, [ONE, 0, 0]));
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_084_a_grazing_bounce_is_sided_by_the_exact_plane() {
    // The triangle of OPTICS-TP-080, front face: n . N < 0 although n . g > 0, and the reverse
    // for -n; the exact test decides.
    let triangle = [
        [14_884_853_697, -1_772_168_654, 15_083_532_738],
        [-5_184_124_845, 13_289_159_059, -7_286_836_095],
        [12_808_423_220, 17_028_505_016, -10_615_521_610],
    ];
    let n: Raw3 = [-3_075_239_551, -1_918_531_781, 2_304_101_036];
    let direction = [-(ONE / 14), ONE, ONE * 346 / 469];
    let centroid: Raw3 = [0, 1, 2].map(|i| triangle.iter().map(|v| v[i]).sum::<i64>() / 3);
    let origin = [0, 1, 2].map(|i| centroid[i] - 2 * direction[i]);
    let (hit, smooth) = shade(triangle, origin, direction, [None; 3]);
    let g = hit.frame().normal();
    let rounded = |d: Raw3| -> i128 {
        [g.x, g.y, g.z].iter().zip(d).map(|(a, b)| i128::from(a.raw()) * i128::from(b)).sum()
    };
    assert!(rounded(n) > 0 && !smooth.is_above_surface(vec3(n)));
    let reversed = n.map(|c| -c);
    assert!(rounded(reversed) < 0 && smooth.is_above_surface(vec3(reversed)));
}

#[wasm_bindgen_test(unsupported = test)]
fn optics_tp_085_bounces_below_the_surface_are_rejected_without_a_ray() {
    // (0.8, 0.6, 0): 53 degrees from the ground's normal, so part of its hemisphere is below.
    let steep: Raw3 = [3_435_973_837, 2_576_980_378, 0];
    for (normal, expect_below) in [(steep, true), (Y, false)] {
        let (hit, smooth) = ground(true, [Some(normal); 3]);
        let (mut rays, mut below) = (0, 0);
        for k in 0..256 {
            let sample = [k % 16, k / 16].map(|c| FixedQ32_32::from_raw(c << 28));
            let direction = smooth.frame().cosine_direction(sample).expect("a direction");
            match smooth.bounce(sample) {
                Some(SmoothBounce::Ray(ray)) => {
                    assert_eq!((ray.origin(), ray.direction()), (hit.point(), direction), "{k}");
                    assert!(smooth.is_above_surface(direction), "{k}");
                    rays += 1;
                }
                Some(SmoothBounce::BelowSurface) => {
                    assert!(!smooth.is_above_surface(direction), "{k}");
                    below += 1;
                }
                None => panic!("sample {k} is in the unit square"),
            }
        }
        // Counted once and locked: 49 of the 256 grid points fall below with the steep normal.
        assert_eq!((rays, below), if expect_below { (207, 49) } else { (256, 0) });
    }
    let (_, smooth) = ground(true, [Some(steep); 3]);
    assert_eq!(smooth.bounce([FixedQ32_32::from_raw(ONE), FixedQ32_32::ZERO]), None);
}
