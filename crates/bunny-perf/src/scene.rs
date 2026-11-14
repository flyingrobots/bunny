//! A whole scene: camera rays through a BVH over a triangle mesh.
//!
//! The kernel cases time one call each. These time what a renderer pays per ray: the tree
//! walk, the box tests and the triangle tests together. The mesh is a bumpy sphere (a
//! subdivided cube pushed out to a seeded radius) standing on two large ground triangles;
//! the camera looks at it through a square grid of rays, some of which hit only the ground
//! or nothing.

// A partial feature build (as `cargo xtask perf` makes against an older revision) leaves
// some of the shared scene helpers unused.
#![cfg_attr(
    not(all(feature = "scene", feature = "trace", feature = "build", feature = "sah")),
    allow(dead_code)
)]

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
pub(crate) const RAYS_PER_SIDE: i64 = 32;

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

/// A BVH builder with `build_bvh`'s signature.
type Builder = fn(&mut [BvhNode], &mut [u32], &[FixedAabb3]) -> Option<usize>;

const EMPTY_NODE: BvhNode = BvhNode {
    bounds: FixedAabb3::new(point(0, 0, 0), point(0, 0, 0)),
    first_child_or_prim_idx: 0,
    prim_count: 0,
};

fn boxes(triangles: &[Triangle]) -> Vec<FixedAabb3> {
    triangles.iter().map(bounds).collect()
}

fn scene_with(build: Builder) -> Scene {
    let triangles = mesh();
    let boxes = boxes(&triangles);
    let mut nodes = vec![EMPTY_NODE; 2 * boxes.len()];
    let mut prim_indices = vec![0_u32; boxes.len()];
    let used = build(&mut nodes, &mut prim_indices, &boxes).unwrap_or(0);
    nodes.truncate(used);
    Scene { triangles, nodes, prim_indices }
}

/// The scene in a tree from `build_bvh` (quartile splits).
pub(crate) fn scene() -> Scene {
    scene_with(build_bvh)
}

/// The scene in a tree from `build_bvh_sah`.
#[cfg(feature = "sah")]
pub(crate) fn sah_scene() -> Scene {
    scene_with(bunny_broadphase::build_bvh_sah)
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

/// The checksum of `trace_closest` over every ray. One function for every tree, so the
/// quartile-tree and SAH-tree cases run the same machine code.
#[cfg(feature = "trace")]
#[inline(never)]
fn closest_sum(
    scene: &Scene,
    triangles: &[bunny_query::TraceTriangle],
    rays: &[bunny_query::TraceRay],
) -> i64 {
    rays.iter()
        .map(|ray| trace_closest(scene, triangles, ray).map_or(0, |hit| hit.distance.raw()))
        .fold(0, i64::wrapping_add)
}

/// The number of rays `any_hit` reports a hit for. One function for every tree.
#[cfg(feature = "trace")]
#[inline(never)]
fn any_sum(
    scene: &Scene,
    triangles: &[bunny_query::TraceTriangle],
    rays: &[bunny_query::TraceRay],
) -> i64 {
    use bunny_broadphase::{any_hit, BvhView};

    let far = FixedQ32_32::from_raw(i64::MAX);
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
}

/// `trace/closest_hit{suffix}` and `trace/any_hit{suffix}` on `scene`'s tree.
#[cfg(feature = "trace")]
fn traversal_cases(scene: Scene, names: [&'static str; 2]) -> Vec<Case> {
    use std::rc::Rc;

    let scene = Rc::new(scene);
    let triangles = Rc::new(trace_triangles(&scene));
    let rays = Rc::new(trace_rays());
    let (closest_scene, closest_triangles, closest_rays) =
        (Rc::clone(&scene), Rc::clone(&triangles), Rc::clone(&rays));
    let [closest_name, any_name] = names;
    vec![
        case(closest_name, move || closest_sum(&closest_scene, &closest_triangles, &closest_rays)),
        case(any_name, move || any_sum(&scene, &triangles, &rays)),
    ]
}

/// Closest hit and any hit with `closest_hit`, `any_hit` and the trace kernels.
#[cfg(feature = "trace")]
pub(crate) fn trace_cases() -> Vec<Case> {
    let mut cases = traversal_cases(scene(), ["trace/closest_hit", "trace/any_hit"]);
    cases.push(case("trace/ray_prepare", {
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
    }));
    cases
}

/// The same rays through the tree `build_bvh_sah` builds over the same triangles.
#[cfg(all(feature = "trace", feature = "sah"))]
pub(crate) fn sah_trace_cases() -> Vec<Case> {
    traversal_cases(sah_scene(), ["trace/closest_hit_sah", "trace/any_hit_sah"])
}

/// One build of the whole scene's tree per call, into buffers allocated once.
#[cfg(any(feature = "build", feature = "sah"))]
fn build_case(name: &'static str, build: Builder) -> Case {
    use std::cell::RefCell;

    let boxes = boxes(&mesh());
    let buffers = RefCell::new((vec![EMPTY_NODE; 2 * boxes.len()], vec![0_u32; boxes.len()]));
    Case {
        name,
        calls: 1,
        run: Box::new(move || {
            let (nodes, prim_indices) = &mut *buffers.borrow_mut();
            let used = build(nodes, prim_indices, black_box(&boxes));
            let root = nodes.first().map_or(0, |node| node.bounds.max.x.raw());
            black_box((used, root)).0.and_then(|used| i64::try_from(used).ok()).unwrap_or(-1)
        }),
    }
}

/// Building the scene's tree with `build_bvh`.
#[cfg(feature = "build")]
pub(crate) fn build_cases() -> Vec<Case> {
    vec![build_case("build/bvh", build_bvh)]
}

/// Building the scene's tree with `build_bvh_sah`.
#[cfg(feature = "sah")]
pub(crate) fn sah_build_cases() -> Vec<Case> {
    vec![build_case("build/bvh_sah", bunny_broadphase::build_bvh_sah)]
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

/// Checks that `closest_hit` and `any_hit` give the same answer on every camera ray in the
/// quartile tree and the SAH tree, so the `_sah` cases time the same results. Returns the
/// indices of the rays on which they differ.
#[cfg(all(feature = "trace", feature = "sah"))]
pub(crate) fn sah_disagreeing_rays() -> Vec<usize> {
    use bunny_broadphase::{any_hit, BvhView};

    let (old, new) = (scene(), sah_scene());
    let triangles = trace_triangles(&old);
    let far = FixedQ32_32::from_raw(i64::MAX);
    let any = |scene: &Scene, ray: &bunny_query::TraceRay| {
        let bvh = BvhView::new(&scene.nodes, &scene.prim_indices);
        any_hit(bvh, ray, far, |primitive| {
            triangles
                .get(primitive as usize)
                .is_some_and(|t| ray.hit_triangle(t, FixedQ32_32::ZERO, far).is_some())
        })
    };
    let mut bad = Vec::new();
    for (index, ray) in trace_rays().iter().enumerate() {
        let closest = |scene: &Scene| trace_closest(scene, &triangles, ray);
        if closest(&old) != closest(&new) || any(&old, ray) != any(&new, ray) {
            bad.push(index);
        }
    }
    bad
}

// These tests need the default feature groups (`scene`, `trace`, `sah`), as `cargo test` uses.
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

    #[test]
    fn the_sah_tree_and_the_quartile_tree_agree_on_every_ray() {
        assert_eq!(sah_disagreeing_rays(), Vec::<usize>::new());
    }

    /// Half surface area in raw units squared; the scene's extents (40 units) keep it exact.
    fn half_area(bounds: &FixedAabb3) -> u128 {
        let extent = |min: FixedQ32_32, max: FixedQ32_32| u128::from(max.raw().abs_diff(min.raw()));
        let (x, y, z) = (
            extent(bounds.min.x, bounds.max.x),
            extent(bounds.min.y, bounds.max.y),
            extent(bounds.min.z, bounds.max.z),
        );
        x * y + y * z + z * x
    }

    /// SAH cost per unit root area, in thousandths, with the weights `build_bvh_sah` uses.
    fn sah_cost_milli(nodes: &[BvhNode]) -> u128 {
        use bunny_broadphase::bvh::{SAH_INTERSECTION_COST, SAH_TRAVERSAL_COST};
        let total: u128 = nodes
            .iter()
            .map(|node| match node.prim_count {
                0 => SAH_TRAVERSAL_COST * half_area(&node.bounds),
                count => SAH_INTERSECTION_COST * half_area(&node.bounds) * u128::from(count),
            })
            .sum();
        total * 1000 / half_area(&nodes[0].bounds)
    }

    fn depth(nodes: &[BvhNode], index: usize) -> usize {
        let node = nodes[index];
        if node.prim_count > 0 {
            return 0;
        }
        let child = node.first_child_or_prim_idx as usize;
        1 + depth(nodes, child).max(depth(nodes, child + 1))
    }

    /// The depth of the leaf that lists `primitive`.
    fn leaf_depth(scene: &Scene, index: usize, primitive: u32) -> Option<usize> {
        let node = scene.nodes[index];
        if node.prim_count > 0 {
            let first = node.first_child_or_prim_idx as usize;
            let slots = &scene.prim_indices[first..first + node.prim_count as usize];
            return slots.contains(&primitive).then_some(0);
        }
        let child = node.first_child_or_prim_idx as usize;
        let below =
            leaf_depth(scene, child, primitive).or_else(|| leaf_depth(scene, child + 1, primitive));
        below.map(|depth| depth + 1)
    }

    /// Triangle tests `closest_hit` and `any_hit` make over all 1 024 rays.
    fn triangle_tests(scene: &Scene) -> (usize, usize) {
        use bunny_broadphase::{any_hit, closest_hit, BvhView};
        use std::cell::Cell;

        let triangles = trace_triangles(scene);
        let far = FixedQ32_32::from_raw(i64::MAX);
        let (closest, any) = (Cell::new(0), Cell::new(0));
        for ray in &trace_rays() {
            let bvh = BvhView::new(&scene.nodes, &scene.prim_indices);
            let _ = closest_hit(bvh, ray, far, |primitive, limit| {
                closest.set(closest.get() + 1);
                ray.hit_triangle(&triangles[primitive as usize], FixedQ32_32::ZERO, limit)
                    .map(|hit| hit.distance)
            });
            let _ = any_hit(bvh, ray, far, |primitive| {
                any.set(any.get() + 1);
                ray.hit_triangle(&triangles[primitive as usize], FixedQ32_32::ZERO, far).is_some()
            });
        }
        (closest.get(), any.get())
    }

    /// A timing-free quality measure: the SAH tree's cost is below the quartile tree's on the
    /// benchmark scene. Run with `--nocapture` to see the shapes, the depth of the two ground
    /// triangles' leaves and the triangle tests per batch.
    #[test]
    fn the_sah_tree_costs_less_than_the_quartile_tree() {
        let (old, new) = (scene(), sah_scene());
        let leaves = |nodes: &[BvhNode]| nodes.iter().filter(|n| n.prim_count > 0).count();
        for (name, scene) in [("build_bvh", &old), ("build_bvh_sah", &new)] {
            let nodes = &scene.nodes;
            println!(
                "{name}: {} nodes, {} leaves, depth {}, SAH cost {} / 1000, ground leaves at depth {:?} and {:?}, triangle tests (closest, any) {:?}",
                nodes.len(),
                leaves(nodes),
                depth(nodes, 0),
                sah_cost_milli(nodes),
                leaf_depth(scene, 0, 27_648),
                leaf_depth(scene, 0, 27_649),
                triangle_tests(scene),
            );
        }
        assert!(sah_cost_milli(&new.nodes) < sah_cost_milli(&old.nodes));
    }
}
