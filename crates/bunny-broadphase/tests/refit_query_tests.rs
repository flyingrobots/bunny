//! Criterion 4 of flyingrobots/bunny#120: on fixed, seeded corpora, every query returns the
//! same answer from a tree refitted to moved primitives as from a tree rebuilt over them by
//! each builder, and both equal a test of every primitive. The motions include ones that
//! make the refitted tree bad (primitives swapping places, collapsing, spreading out):
//! correctness never depends on tree quality, and quality is not measured here.

mod common;
mod refit_support;

use bunny_broadphase::{
    any_hit, build_bvh, build_bvh_sah, closest_hit, compact_any_hit, compact_bvh,
    compact_closest_hit, intersect_aabb, sweep_and_prune, BvhView, ClosestHit, CompactNode,
    CompactView,
};
use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use bunny_query::{TraceRay, TraceTriangle};
use common::{aabb, build_with, vec3, Builder, Seeds, Tree, ONE};
use refit_support::{corpora, motions, raw_box, refitted};
use wasm_bindgen_test::wasm_bindgen_test;

const QUERY_SEED: u64 = 0x2EF1_7000_0000_0053;
const FAR: FixedQ32_32 = FixedQ32_32::from_raw(i64::MAX);

const BUILDERS: [(&str, Builder); 2] = [("build_bvh", build_bvh), ("build_bvh_sah", build_bvh_sah)];

/// The four trees compared for one corpus and motion: each builder's tree over the original
/// boxes refitted to the moved ones, and each builder's tree rebuilt over the moved ones.
fn trees(original: &[FixedAabb3], moved: &[FixedAabb3]) -> Vec<(String, Tree)> {
    let mut out = Vec::new();
    for (name, builder) in BUILDERS {
        let refit = refitted(&build_with(builder, original), moved).expect("a built tree refits");
        out.push((format!("{name} refitted"), refit));
        out.push((format!("{name} rebuilt"), build_with(builder, moved)));
    }
    out
}

// ---- Rays: closest_hit, any_hit and the compact searches ----

fn moved_triangle(triangle: &TraceTriangle, map: impl Fn(FixedVec3) -> FixedVec3) -> TraceTriangle {
    let [a, b, c] = triangle.vertices().map(map);
    TraceTriangle::new(a, b, c).expect("the motion stays in the trace domain")
}

fn seeded_triangles(seeds: &mut Seeds, count: usize) -> Vec<TraceTriangle> {
    let mut triangles: Vec<TraceTriangle> = (0..count)
        .map(|index| {
            let reach = if index % 40 == 0 { 6 * ONE } else { ONE };
            let v0 = seeds.point(6 * ONE);
            TraceTriangle::new(v0, v0 + seeds.point(reach), v0 + seeds.point(reach))
                .expect("in domain")
        })
        .collect();
    let ground = |x: i64, z: i64| vec3([x * ONE, -7 * ONE, z * ONE]);
    triangles.extend(TraceTriangle::new(ground(-40, -40), ground(40, -40), ground(40, 40)));
    triangles.extend(TraceTriangle::new(ground(-40, -40), ground(40, 40), ground(-40, 40)));
    let copies: Vec<TraceTriangle> = triangles.iter().step_by(10).copied().collect();
    triangles.extend(copies);
    triangles
}

fn triangle_motions(
    seeds: &mut Seeds,
    triangles: &[TraceTriangle],
) -> Vec<(&'static str, Vec<TraceTriangle>)> {
    let jitter = triangles
        .iter()
        .map(|triangle| {
            let step = seeds.point(ONE);
            moved_triangle(triangle, |vertex| vertex + step)
        })
        .collect();
    let swapped = triangles.iter().rev().copied().collect();
    let one = TraceTriangle::new(
        vec3([-6 * ONE, -6 * ONE, 0]),
        vec3([6 * ONE, -6 * ONE, 0]),
        vec3([0, 6 * ONE, 0]),
    )
    .expect("in domain");
    let collapsed = vec![one; triangles.len()];
    let spread = triangles
        .iter()
        .map(|triangle| {
            moved_triangle(triangle, |v| vec3([v.x.raw() * 4, v.y.raw() * 4, v.z.raw() * 4]))
        })
        .collect();
    vec![
        ("jitter", jitter),
        ("swap places", swapped),
        ("collapse to one triangle", collapsed),
        ("spread out", spread),
    ]
}

fn seeded_ray(seeds: &mut Seeds) -> Option<TraceRay> {
    let origin = seeds.point(8 * ONE);
    let toward = seeds.point(6 * ONE) - origin;
    let mut direction = [toward.x.raw() / 16, toward.y.raw() / 16, toward.z.raw() / 16];
    for component in &mut direction {
        match seeds.next() % 10 {
            0 => *component = 0,
            1 => *component = seeds.signed(2),
            _ => {}
        }
    }
    TraceRay::new(origin, vec3(direction))
}

struct RayScene<'a> {
    triangles: &'a [TraceTriangle],
}

impl RayScene<'_> {
    fn distance(&self, ray: &TraceRay, primitive: u32, limit: FixedQ32_32) -> Option<FixedQ32_32> {
        let triangle = self.triangles.get(usize::try_from(primitive).ok()?)?;
        ray.hit_triangle(triangle, FixedQ32_32::ZERO, limit).map(|hit| hit.distance)
    }

    fn brute_force(&self, ray: &TraceRay) -> Option<ClosestHit> {
        let mut best: Option<ClosestHit> = None;
        for (index, _) in self.triangles.iter().enumerate() {
            let primitive = u32::try_from(index).expect("small");
            let Some(distance) = self.distance(ray, primitive, FAR) else { continue };
            if best.is_none_or(|b| (distance, primitive) < (b.distance, b.primitive)) {
                best = Some(ClosestHit { primitive, distance });
            }
        }
        best
    }

    /// `closest_hit`, `any_hit` and, on `compact_bvh` of the tree, their compact forms.
    fn answers(&self, tree: &Tree, ray: &TraceRay) -> [(Option<ClosestHit>, bool); 2] {
        let bvh = BvhView::new(&tree.nodes, &tree.prim_indices);
        let closest = |p, limit| self.distance(ray, p, limit);
        let any = |p| self.distance(ray, p, FAR).is_some();
        let plain = (
            closest_hit(bvh, ray, FAR, closest).expect("well formed"),
            any_hit(bvh, ray, FAR, any).expect("well formed"),
        );
        let empty = CompactNode { bounds: [tree.nodes[0].bounds; 2], children: Default::default() };
        let mut out = vec![empty; tree.nodes.len() / 2];
        let (root, used) =
            compact_bvh(&tree.nodes, &mut out).expect("converts").expect("not empty");
        let view =
            CompactView { root: Some(root), nodes: &out[..used], prim_indices: &tree.prim_indices };
        let compact = (
            compact_closest_hit(view, ray, FAR, closest).expect("well formed"),
            compact_any_hit(view, ray, FAR, any).expect("well formed"),
        );
        [plain, compact]
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_053_ray_searches_on_refitted_and_rebuilt_trees_equal_every_primitive() {
    let mut seeds = Seeds(QUERY_SEED);
    let mut total = (0_usize, 0_usize);
    for count in [40, 200] {
        let original = seeded_triangles(&mut seeds, count);
        let boxes: Vec<FixedAabb3> = original.iter().map(TraceTriangle::bounds).collect();
        for (motion, moved) in triangle_motions(&mut seeds, &original) {
            let moved_boxes: Vec<FixedAabb3> = moved.iter().map(TraceTriangle::bounds).collect();
            let scene = RayScene { triangles: &moved };
            let compared = trees(&boxes, &moved_boxes);
            let (mut hits, mut rays) = (0_usize, 0_usize);
            for case in 0..300 {
                let Some(ray) = seeded_ray(&mut seeds) else { continue };
                let expected = scene.brute_force(&ray);
                for (name, tree) in &compared {
                    for (form, answer) in ["plain", "compact"].iter().zip(scene.answers(tree, &ray))
                    {
                        let context = format!(
                            "seed={QUERY_SEED:#x} n={count} {motion} {name} {form} case={case}"
                        );
                        assert_eq!(answer, (expected, expected.is_some()), "{context}");
                    }
                }
                if motion.starts_with("collapse") {
                    // Every primitive is the same triangle: the lowest index wins every tie.
                    assert!(expected.is_none_or(|hit| hit.primitive == 0), "collapse case={case}");
                }
                hits += usize::from(expected.is_some());
                rays += 1;
            }
            assert!(
                hits >= 20 && rays - hits >= 20,
                "n={count} {motion}: {hits} hits of {rays} rays"
            );
            total = (total.0 + hits, total.1 + rays);
        }
    }
    println!("bvh_tp_053: {} hits of {} rays over 8 scene motions", total.0, total.1);
}

// ---- Boxes: intersect_aabb and pairs ----

/// Closed-interval overlap of two raw boxes on every axis, written here.
fn overlaps(a: [i64; 6], b: [i64; 6]) -> bool {
    (0..3).all(|axis| a[axis] <= b[axis + 3] && b[axis] <= a[axis + 3])
}

/// The primitives whose own box overlaps `query`, from the tree: `intersect_aabb` reports
/// whole leaves, so each reported primitive is tested; sorted for comparison.
fn tree_overlaps(tree: &Tree, boxes: &[FixedAabb3], query: &FixedAabb3) -> Vec<u32> {
    let mut found = Vec::new();
    intersect_aabb(&tree.nodes, &tree.prim_indices, query, |primitive| {
        if overlaps(raw_box(&boxes[primitive as usize]), raw_box(query)) {
            found.push(primitive);
        }
    })
    .expect("well formed");
    found.sort_unstable();
    found
}

fn brute_overlaps(boxes: &[FixedAabb3], query: &FixedAabb3) -> Vec<u32> {
    let query = raw_box(query);
    (0..boxes.len())
        .filter(|&index| overlaps(raw_box(&boxes[index]), query))
        .map(|index| u32::try_from(index).expect("small"))
        .collect()
}

fn seeded_query(seeds: &mut Seeds) -> FixedAabb3 {
    let corner = seeds.point(8 * ONE);
    let size = [seeds.next(), seeds.next(), seeds.next()]
        .map(|bits| i64::try_from(bits % 4_000).expect("small") * ONE / 1_000);
    let min = [corner.x.raw(), corner.y.raw(), corner.z.raw()];
    aabb(min, [min[0] + size[0], min[1] + size[1], min[2] + size[2]])
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_054_box_queries_on_refitted_and_rebuilt_trees_equal_every_primitive() {
    let mut seeds = Seeds(QUERY_SEED ^ 0x54);
    let (mut queries, mut found) = (0_usize, 0_usize);
    for (corpus, boxes) in corpora(QUERY_SEED ^ 0x54) {
        for (motion, moved) in motions(&mut seeds, &boxes) {
            let compared = trees(&boxes, &moved);
            for case in 0..40 {
                let query = seeded_query(&mut seeds);
                let expected = brute_overlaps(&moved, &query);
                for (name, tree) in &compared {
                    let got = tree_overlaps(tree, &moved, &query);
                    assert_eq!(
                        got, expected,
                        "seed={QUERY_SEED:#x} {corpus} {motion} {name} case={case}"
                    );
                }
                queries += 1;
                found += expected.len();
            }
        }
    }
    assert_eq!(queries, 7 * 5 * 40);
    assert!(found > queries, "the queries find primitives: {found} over {queries} queries");
}

#[wasm_bindgen_test(unsupported = test)]
fn bvh_tp_055_overlapping_pairs_from_refitted_and_rebuilt_trees_equal_sweep_and_prune() {
    let mut seeds = Seeds(QUERY_SEED ^ 0x55);
    let mut total_pairs = 0_usize;
    for (corpus, boxes) in corpora(QUERY_SEED ^ 0x55) {
        for (motion, moved) in motions(&mut seeds, &boxes) {
            let n = moved.len();
            let mut brute = Vec::new();
            for (a, first) in moved.iter().enumerate() {
                for (b, second) in moved.iter().enumerate().skip(a + 1) {
                    if overlaps(raw_box(first), raw_box(second)) {
                        brute.push((
                            u32::try_from(a).expect("small"),
                            u32::try_from(b).expect("small"),
                        ));
                    }
                }
            }
            let mut sap = vec![(0_u32, 0_u32); n * n.saturating_sub(1) / 2];
            let mut scratch = vec![0_u32; n];
            let used =
                sweep_and_prune(&mut sap, &mut scratch, &moved).expect("room for every pair");
            assert_eq!(&sap[..used], &brute[..], "{corpus} {motion}: sweep_and_prune");
            for (name, tree) in trees(&boxes, &moved) {
                let mut pairs = Vec::new();
                for (a, query) in moved.iter().enumerate() {
                    let a = u32::try_from(a).expect("small");
                    pairs.extend(
                        tree_overlaps(&tree, &moved, query)
                            .into_iter()
                            .filter(|&b| b > a)
                            .map(|b| (a, b)),
                    );
                }
                assert_eq!(pairs, brute, "seed={QUERY_SEED:#x} {corpus} {motion} {name}");
            }
            total_pairs += brute.len();
        }
    }
    println!("bvh_tp_055: {total_pairs} overlapping pairs over 35 corpus motions");
}
