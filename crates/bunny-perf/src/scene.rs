//! A whole scene: camera rays through a BVH over a triangle mesh.
//!
//! The kernel cases time one call each. These time what a renderer pays per ray: the tree
//! walk, the box tests and the triangle tests together. The mesh is a bumpy sphere (a
//! subdivided cube pushed out to a seeded radius) standing on two large ground triangles;
//! the camera looks at it through a square grid of rays, some of which hit only the ground
//! or nothing.

use std::hint::black_box;

use bunny_broadphase::{build_bvh, BvhNode};
use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;

use crate::cases::{case, Case};

/// Quads along each edge of each cube face: 6 * 48 * 48 * 2 = 27 648 triangles.
const GRID: i64 = 48;
/// Camera rays per side; `RAYS_PER_SIDE^2` must equal the batch size the cases report.
const RAYS_PER_SIDE: i64 = 32;

pub(crate) type Triangle = [FixedVec3; 3];

pub(crate) struct Scene {
    pub(crate) triangles: Vec<Triangle>,
    pub(crate) nodes: Vec<BvhNode>,
    pub(crate) prim_indices: Vec<u32>,
}

const fn fixed(raw: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(raw)
}

const fn point(x: i64, y: i64, z: i64) -> FixedVec3 {
    FixedVec3::new(fixed(x), fixed(y), fixed(z))
}

/// A point on the bumpy sphere above cube-face coordinates `(a, b)` in `0..=GRID`.
fn sphere_point(face: usize, a: i64, b: i64) -> FixedVec3 {
    let along = |i: i64| 2 * ONE_RAW * i / GRID - ONE_RAW;
    let (a, b) = (along(a), along(b));
    let cube = match face {
        0 => point(ONE_RAW, a, b),
        1 => point(-ONE_RAW, b, a),
        2 => point(b, ONE_RAW, a),
        3 => point(a, -ONE_RAW, b),
        4 => point(a, b, ONE_RAW),
        _ => point(b, a, -ONE_RAW),
    };
    let unit = cube.normalize().unwrap_or(cube);
    // Up to 4 % of extra radius, from a hash of the cube point so shared corners agree.
    let hash = (cube.x.raw() ^ cube.y.raw().rotate_left(21) ^ cube.z.raw().rotate_left(42))
        .wrapping_mul(0x9E37_79B9_7F4A_7C15_u64.cast_signed());
    let radius = fixed(ONE_RAW + (hash >> 40).rem_euclid(ONE_RAW / 25));
    unit * radius
}

fn mesh() -> Vec<Triangle> {
    let mut triangles = Vec::new();
    for face in 0..6 {
        for a in 0..GRID {
            for b in 0..GRID {
                let corner = |da, db| sphere_point(face, a + da, b + db);
                let (p00, p10, p11, p01) = (corner(0, 0), corner(1, 0), corner(1, 1), corner(0, 1));
                triangles.push([p00, p10, p11]);
                triangles.push([p00, p11, p01]);
            }
        }
    }
    let (ground, far) = (-ONE_RAW * 11 / 10, 20 * ONE_RAW);
    let corner = |x, z| point(x, ground, z);
    triangles.push([corner(-far, -far), corner(far, -far), corner(far, far)]);
    triangles.push([corner(-far, -far), corner(far, far), corner(-far, far)]);
    triangles
}

fn bounds(triangle: &Triangle) -> FixedAabb3 {
    let [a, b, c] = *triangle;
    FixedAabb3::new(
        FixedVec3::new(a.x.min(b.x).min(c.x), a.y.min(b.y).min(c.y), a.z.min(b.z).min(c.z)),
        FixedVec3::new(a.x.max(b.x).max(c.x), a.y.max(b.y).max(c.y), a.z.max(b.z).max(c.z)),
    )
}

pub(crate) fn scene() -> Scene {
    let triangles = mesh();
    let boxes: Vec<FixedAabb3> = triangles.iter().map(bounds).collect();
    let empty = BvhNode {
        bounds: FixedAabb3::new(point(0, 0, 0), point(0, 0, 0)),
        first_child_or_prim_idx: 0,
        prim_count: 0,
    };
    let mut nodes = vec![empty; 2 * boxes.len()];
    let mut prim_indices = vec![0_u32; boxes.len()];
    let used = build_bvh(&mut nodes, &mut prim_indices, &boxes).unwrap_or(0);
    nodes.truncate(used);
    Scene { triangles, nodes, prim_indices }
}

/// Camera position and one direction per ray, not normalized, every component within 1.0.
fn camera() -> (FixedVec3, Vec<FixedVec3>) {
    let origin = point(0, ONE_RAW * 4 / 5, 4 * ONE_RAW);
    let mut directions = Vec::new();
    for row in 0..RAYS_PER_SIDE {
        for column in 0..RAYS_PER_SIDE {
            // A 1.0-wide window one unit ahead, tilted down toward the sphere.
            let across = |i: i64| (2 * i + 1) * ONE_RAW / (2 * RAYS_PER_SIDE) - ONE_RAW / 2;
            directions.push(point(across(column), -across(row) - ONE_RAW / 5, -ONE_RAW));
        }
    }
    (origin, directions)
}

/// The camera rays for the visitor path, in the same order as [`trace_rays`].
#[cfg(feature = "scene")]
fn visitor_rays() -> Vec<bunny_geom::FixedRay3> {
    let (origin, directions) = camera();
    directions.iter().filter_map(|&d| bunny_geom::FixedRay3::try_new(origin, d).ok()).collect()
}

/// Closest hit with the APIs Bunny had before `closest_hit`: visit every primitive whose box
/// the ray's line touches, test it, and keep the nearest hit point (the first one visited
/// among equal distances).
#[cfg(feature = "scene")]
fn visitor_closest(scene: &Scene, ray: &bunny_geom::FixedRay3) -> Option<(FixedQ32_32, u32)> {
    let direction = ray.direction.into_inner();
    let mut best: Option<(FixedQ32_32, u32)> = None;
    let visited = bunny_broadphase::intersect_ray(
        &scene.nodes,
        &scene.prim_indices,
        black_box(ray),
        |primitive| {
            let Some(&[a, b, c]) = scene.triangles.get(primitive as usize) else {
                return;
            };
            if let Some(hit) = bunny_query::ray_intersects_triangle(ray, a, b, c) {
                let distance = (hit - ray.origin).dot(direction);
                if best.is_none_or(|(nearest, _)| distance < nearest) {
                    best = Some((distance, primitive));
                }
            }
        },
    );
    black_box((visited, best)).1
}

#[cfg(feature = "scene")]
pub(crate) fn visitor_cases() -> Vec<Case> {
    let scene = scene();
    let rays = visitor_rays();
    vec![case("scene/closest_visitor", move || {
        rays.iter()
            .map(|ray| visitor_closest(&scene, ray).map_or(0, |(distance, _)| distance.raw()))
            .fold(0, i64::wrapping_add)
    })]
}

#[cfg(feature = "trace")]
pub(crate) fn trace_rays() -> Vec<bunny_query::TraceRay> {
    let (origin, directions) = camera();
    directions.iter().filter_map(|&d| bunny_query::TraceRay::new(origin, d)).collect()
}

#[cfg(feature = "trace")]
pub(crate) fn trace_triangles(scene: &Scene) -> Vec<bunny_query::TraceTriangle> {
    scene
        .triangles
        .iter()
        .filter_map(|&[a, b, c]| bunny_query::TraceTriangle::new(a, b, c))
        .collect()
}

/// Closest hit with `closest_hit` and the trace triangle test.
#[cfg(feature = "trace")]
pub(crate) fn trace_closest(
    scene: &Scene,
    triangles: &[bunny_query::TraceTriangle],
    ray: &bunny_query::TraceRay,
) -> Option<bunny_broadphase::ClosestHit> {
    let far = FixedQ32_32::from_raw(i64::MAX);
    let ray = black_box(ray);
    let bvh = bunny_broadphase::BvhView::new(&scene.nodes, &scene.prim_indices);
    let hit = bunny_broadphase::closest_hit(bvh, ray, far, |primitive, limit| {
        let triangle = triangles.get(primitive as usize)?;
        ray.hit_triangle(triangle, FixedQ32_32::ZERO, limit).map(|hit| hit.distance)
    });
    black_box(hit).ok().flatten()
}

/// Closest hit and any hit with `closest_hit`, `any_hit` and the trace kernels.
#[cfg(feature = "trace")]
pub(crate) fn trace_cases() -> Vec<Case> {
    use std::rc::Rc;

    use bunny_broadphase::{any_hit, BvhView};

    let scene = Rc::new(scene());
    let triangles = Rc::new(trace_triangles(&scene));
    let rays = Rc::new(trace_rays());
    let (closest_scene, closest_triangles, closest_rays) =
        (Rc::clone(&scene), Rc::clone(&triangles), Rc::clone(&rays));
    let far = FixedQ32_32::from_raw(i64::MAX);
    vec![
        case("trace/closest_hit", move || {
            closest_rays
                .iter()
                .map(|ray| {
                    trace_closest(&closest_scene, &closest_triangles, ray)
                        .map_or(0, |hit| hit.distance.raw())
                })
                .fold(0, i64::wrapping_add)
        }),
        case("trace/any_hit", move || {
            rays.iter()
                .map(|ray| {
                    let ray = black_box(ray);
                    let bvh = BvhView::new(&scene.nodes, &scene.prim_indices);
                    let hit = any_hit(bvh, ray, far, |primitive| {
                        triangles.get(primitive as usize).is_some_and(|triangle| {
                            ray.hit_triangle(triangle, FixedQ32_32::ZERO, far).is_some()
                        })
                    });
                    i64::from(black_box(hit).unwrap_or(false))
                })
                .fold(0, i64::wrapping_add)
        }),
        case("trace/ray_prepare", {
            let (origin, directions) = camera();
            move || {
                directions
                    .iter()
                    .map(|&d| {
                        black_box(bunny_query::TraceRay::new(black_box(origin), d))
                            .map_or(0, |ray| ray.direction().x.raw())
                    })
                    .fold(0, i64::wrapping_add)
            }
        }),
    ]
}

/// Checks that the visitor path and `closest_hit` choose the same primitive on every camera
/// ray (or both miss), so `scene/closest_visitor` and `trace/closest_hit` time the same work.
/// Returns the indices of the rays on which they differ.
#[cfg(all(feature = "scene", feature = "trace"))]
pub(crate) fn disagreeing_rays() -> Vec<usize> {
    let scene = scene();
    let triangles = trace_triangles(&scene);
    let (old_rays, new_rays) = (visitor_rays(), trace_rays());
    let mut bad: Vec<usize> = Vec::new();
    if old_rays.len() != new_rays.len()
        || i64::try_from(old_rays.len()) != Ok(RAYS_PER_SIDE * RAYS_PER_SIDE)
    {
        bad.push(usize::MAX);
    }
    for (index, (old, new)) in old_rays.iter().zip(&new_rays).enumerate() {
        let old = visitor_closest(&scene, old).map(|(_, primitive)| primitive);
        let new = trace_closest(&scene, &triangles, new).map(|hit| hit.primitive);
        if old != new {
            bad.push(index);
        }
    }
    bad
}

// These tests need the default feature groups (`scene` and `trace`), as `cargo test` uses.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scene_has_the_stated_size() {
        assert_eq!(mesh().len(), 27_650);
    }

    #[test]
    fn both_paths_choose_the_same_primitive_on_every_ray() {
        assert_eq!(disagreeing_rays(), Vec::<usize>::new());
    }

    #[test]
    fn most_rays_hit_something() {
        // A guard that the benchmark rays do real work, not only misses.
        let scene = scene();
        let hits = visitor_rays().iter().filter(|r| visitor_closest(&scene, r).is_some()).count();
        assert!(hits > 600, "only {hits} of 1024 rays hit");
    }
}
