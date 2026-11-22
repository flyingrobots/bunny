//! The counted compact searches (`compact_closest_hit_counted`, `compact_any_hit_counted`,
//! flyingrobots/bunny#335): hand-counted trees, a recursive reference search with its own
//! counters, and agreement with the uncounted functions on valid and malformed trees. Run
//! these under a wall-clock limit: a mutated search can loop on a malformed tree.

use bunny_broadphase::{
    build_bvh, build_bvh_sah, compact_any_hit, compact_any_hit_counted, compact_bvh,
    compact_closest_hit, compact_closest_hit_counted, BvhNode, ClosestHit, CompactChild,
    CompactNode, CompactRoot, CompactView, TraversalCounts,
};
use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::fixed_q32_32::ONE_RAW;
use bunny_num::FixedQ32_32;
use bunny_query::{TraceRay, TraceTriangle};
use wasm_bindgen_test::wasm_bindgen_test;

const COUNT_SEED: u64 = 0xB002_0335_0000_0001;
const FAR: FixedQ32_32 = FixedQ32_32::from_raw(i64::MAX);

fn units(n: i64) -> FixedQ32_32 {
    FixedQ32_32::from_raw(n * ONE_RAW)
}

fn vec3(raw: [i64; 3]) -> FixedVec3 {
    let [x, y, z] = raw.map(FixedQ32_32::from_raw);
    FixedVec3::new(x, y, z)
}

/// `[box_tests, box_entries, popped_skipped, interior_visits, leaf_visits, primitive_tests,
/// peak_stack]`, in that order.
fn counts(values: [u64; 7]) -> TraversalCounts {
    let mut counts = TraversalCounts::default();
    let fields = [
        &mut counts.box_tests,
        &mut counts.box_entries,
        &mut counts.popped_skipped,
        &mut counts.interior_visits,
        &mut counts.leaf_visits,
        &mut counts.primitive_tests,
        &mut counts.peak_stack,
    ];
    for (field, value) in fields.into_iter().zip(values) {
        *field = value;
    }
    counts
}

// ---- Hand-counted trees -------------------------------------------------------------------

/// The box `-1..=1` in x and y and `low..=high` in z: the ray below enters it at `5 - high`.
fn slab(low: i64, high: i64) -> FixedAabb3 {
    FixedAabb3::new(
        vec3([-ONE_RAW, -ONE_RAW, low * ONE_RAW]),
        vec3([ONE_RAW, ONE_RAW, high * ONE_RAW]),
    )
}

fn away() -> FixedAabb3 {
    FixedAabb3::new(vec3([7 * ONE_RAW; 3]), vec3([8 * ONE_RAW; 3]))
}

const fn leaf(primitive: u32) -> CompactChild {
    CompactChild { first: primitive, prim_count: 1 }
}

const fn node(index: u32) -> CompactChild {
    CompactChild { first: index, prim_count: 0 }
}

/// Both searches on a hand-built tree whose primitive `p` reports `distances[p]`.
fn both(
    root: Option<CompactRoot>,
    nodes: &[CompactNode],
    prims: &[u32],
    distances: &[Option<i64>],
) -> [(Option<u32>, TraversalCounts); 2] {
    let ray = TraceRay::new(vec3([0, 0, 5 * ONE_RAW]), vec3([0, 0, -ONE_RAW])).expect("in domain");
    let view = CompactView { root, nodes, prim_indices: prims };
    let distance = |p: u32| distances[p as usize].map(units);
    let closest =
        compact_closest_hit_counted(view, &ray, FAR, |p, _| distance(p)).expect("well formed");
    let any =
        compact_any_hit_counted(view, &ray, FAR, |p| distance(p).is_some()).expect("well formed");
    [(closest.0.map(|hit| hit.primitive), closest.1), (any.0.then_some(u32::MAX), any.1)]
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_415_counts_on_constructed_trees_are_the_hand_derived_values() {
    let none = TraversalCounts::default();
    assert_eq!(both(None, &[], &[], &[]), [(None, none), (None, none)], "empty tree");
    // One leaf of three: root box tested and entered, one leaf, three tests (any: one).
    let one =
        Some(CompactRoot { bounds: slab(-3, 0), child: CompactChild { first: 0, prim_count: 3 } });
    let found = both(one, &[], &[0, 1, 2], &[Some(8), Some(6), None]);
    assert_eq!(found[0], (Some(1), counts([1, 1, 0, 0, 1, 3, 1])), "one leaf");
    assert_eq!(found[1], (Some(u32::MAX), counts([1, 1, 0, 0, 1, 1, 1])), "one leaf, any");
    let missed = Some(CompactRoot { bounds: away(), child: leaf(0) });
    assert_eq!(both(missed, &[], &[0], &[Some(5)])[0], (None, counts([1, 0, 0, 0, 0, 0, 0])));
    // Root over leaf 0 (entered at 8.0) and leaf 1 (entered at 5.0, searched first).
    let root = Some(CompactRoot { bounds: slab(-3, 0), child: node(0) });
    let pair = [CompactNode { bounds: [slab(-3, -3), slab(0, 0)], children: [leaf(0), leaf(1)] }];
    // The hit at 5.0 leaves leaf 0's bound 8.0 beyond the limit: skipped at its pop.
    let found = both(root, &pair, &[0, 1], &[Some(8), Some(5)]);
    assert_eq!(found[0], (Some(1), counts([3, 3, 1, 1, 1, 1, 2])), "nearer prunes farther");
    assert_eq!(found[1].1, counts([3, 3, 0, 1, 1, 1, 2]), "any stops, nothing popped");
    let found = both(root, &pair, &[0, 1], &[Some(8), None]);
    assert_eq!(found[0], (Some(0), counts([3, 3, 0, 1, 2, 2, 2])), "nearer entered, missed");
    // Both leaves start at 5.0, where both primitives are hit; the left one, searched first
    // on the tie, holds the higher index, so the right one is still searched (bound = limit).
    let tie = [CompactNode { bounds: [slab(0, 0), slab(0, 0)], children: [leaf(0), leaf(1)] }];
    let found = both(root, &tie, &[1, 0], &[Some(5), Some(5)]);
    assert_eq!(found[0], (Some(0), counts([3, 3, 0, 1, 2, 2, 2])), "a bound equal to the limit");
    let neither = [CompactNode { bounds: [away(), away()], children: [leaf(0), leaf(1)] }];
    let found = both(root, &neither, &[0, 1], &[Some(8), Some(5)]);
    assert_eq!(found[0], (None, counts([3, 1, 0, 1, 0, 0, 1])), "root entered, no child");
    // Root over A (node 1, leaves at 5.0 and 6.0) and B (node 2, from 8.0): the hit at 5.0 in
    // A skips A's second leaf and B, whose boxes are never tested.
    let root = Some(CompactRoot { bounds: slab(-4, 0), child: node(0) });
    let three = [
        CompactNode { bounds: [slab(-1, 0), slab(-4, -3)], children: [node(1), node(2)] },
        CompactNode { bounds: [slab(0, 0), slab(-1, -1)], children: [leaf(0), leaf(1)] },
        CompactNode { bounds: [slab(-3, -3), slab(-4, -4)], children: [leaf(2), leaf(3)] },
    ];
    let found = both(root, &three, &[0, 1, 2, 3], &[Some(5), Some(6), Some(8), Some(9)]);
    assert_eq!(found[0], (Some(0), counts([5, 5, 2, 2, 1, 1, 3])), "a subtree pruned at its pop");
    assert_eq!(found[1], (Some(u32::MAX), counts([5, 5, 0, 2, 1, 1, 3])), "any, first hit");
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_419_adding_counts_sums_saturates_and_keeps_the_peak() {
    let a = counts([1, 2, 3, 4, 5, 6, 7]);
    let b = counts([10, 20, 30, 40, 50, 60, 3]);
    assert_eq!(a + b, counts([11, 22, 33, 44, 55, 66, 7]));
    assert_eq!(b + a, counts([11, 22, 33, 44, 55, 66, 7]));
    let full = counts([u64::MAX; 7]);
    assert_eq!(full + a, full, "saturates");
    let mut sum = a;
    sum += b;
    assert_eq!(sum, a + b);
}

// ---- Generated scenes ---------------------------------------------------------------------

struct Seeds(u64);

impl Seeds {
    fn next(&mut self) -> u64 {
        self.0 =
            self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    /// Reduced in `u64` first: a 53-bit draw does not fit the 32-bit `usize` of `wasm32`.
    fn below(&mut self, bound: usize) -> usize {
        let bound = u64::try_from(bound).expect("fits");
        usize::try_from(self.next() % bound).expect("below the bound")
    }

    fn point(&mut self, bound: i64) -> FixedVec3 {
        let span = u64::try_from(2 * bound + 1).expect("positive");
        vec3([(); 3].map(|()| i64::try_from(self.next() % span).expect("small") - bound))
    }
}

struct Scene {
    triangles: Vec<TraceTriangle>,
    root: Option<CompactRoot>,
    nodes: Vec<CompactNode>,
    prims: Vec<u32>,
}

type Builder = fn(&mut [BvhNode], &mut [u32], &[FixedAabb3]) -> Option<usize>;

impl Scene {
    fn seeded(seeds: &mut Seeds, count: usize, build: Builder) -> Self {
        let mut triangles: Vec<TraceTriangle> = (0..count)
            .filter_map(|_| {
                let v0 = seeds.point(6 * ONE_RAW);
                TraceTriangle::new(v0, v0 + seeds.point(ONE_RAW), v0 + seeds.point(ONE_RAW))
            })
            .collect();
        // Copies of every third triangle make ties: equal distances and equal box bounds.
        let copies: Vec<TraceTriangle> = triangles.iter().step_by(3).copied().collect();
        triangles.extend(copies);
        let boxes: Vec<FixedAabb3> = triangles.iter().map(TraceTriangle::bounds).collect();
        let blank = BvhNode { bounds: away(), first_child_or_prim_idx: 0, prim_count: 0 };
        let (mut tree, mut prims) = (vec![blank; 2 * boxes.len()], vec![0; boxes.len()]);
        let used = build(&mut tree, &mut prims, &boxes).expect("buffers fit");
        let empty = CompactNode { bounds: [away(); 2], children: [CompactChild::default(); 2] };
        let mut nodes = vec![empty; used / 2];
        let (root, written) = compact_bvh(&tree[..used], &mut nodes)
            .expect("a tree")
            .map_or((None, 0), |(r, n)| (Some(r), n));
        nodes.truncate(written);
        Self { triangles, root, nodes, prims }
    }

    fn view(&self) -> CompactView<'_> {
        CompactView { root: self.root, nodes: &self.nodes, prim_indices: &self.prims }
    }

    fn distance(&self, ray: &TraceRay, primitive: u32, limit: FixedQ32_32) -> Option<FixedQ32_32> {
        let triangle = self.triangles.get(primitive as usize)?;
        ray.hit_triangle(triangle, FixedQ32_32::ZERO, limit).map(|hit| hit.distance)
    }
}

fn seeded_ray(seeds: &mut Seeds) -> Option<TraceRay> {
    let origin = seeds.point(8 * ONE_RAW);
    let toward = seeds.point(6 * ONE_RAW) - origin;
    TraceRay::new(origin, vec3([toward.x.raw() / 16, toward.y.raw() / 16, toward.z.raw() / 16]))
}

/// A recursive search with its own counters: the stack of the loop is the recursion, and an
/// entry's depth is the number of siblings still pending below it.
struct Reference<'a> {
    scene: &'a Scene,
    ray: &'a TraceRay,
    limit: FixedQ32_32,
    stop_at_first: bool,
    best: Option<ClosestHit>,
    done: bool,
    counts: TraversalCounts,
    calls: Vec<(u32, FixedQ32_32)>,
}

impl Reference<'_> {
    fn bound(&mut self, bounds: &FixedAabb3) -> Option<FixedQ32_32> {
        let enter = self.ray.box_entry(bounds, self.limit);
        self.counts.box_tests += 1;
        self.counts.box_entries += u64::from(enter.is_some());
        enter
    }

    fn reach(&mut self, depth: u64) {
        self.counts.peak_stack = self.counts.peak_stack.max(depth);
    }

    fn search(&mut self) {
        let Some(root) = self.scene.root else { return };
        if let Some(enter) = self.bound(&root.bounds) {
            self.reach(1);
            self.enter(root.child, enter, 0);
        }
    }

    fn enter(&mut self, child: CompactChild, enter: FixedQ32_32, below: u64) {
        if self.done {
            return;
        }
        if enter > self.limit {
            self.counts.popped_skipped += 1;
        } else if child.is_leaf() {
            self.leaf(child);
        } else {
            self.interior(child.first as usize, below);
        }
    }

    fn interior(&mut self, index: usize, below: u64) {
        self.counts.interior_visits += 1;
        let node = self.scene.nodes[index];
        let left = self.bound(&node.bounds[0]).map(|b| (node.children[0], b));
        let right = self.bound(&node.bounds[1]).map(|b| (node.children[1], b));
        let (near, far) = match (left, right) {
            (Some(l), Some(r)) if r.1 < l.1 => (Some(r), Some(l)),
            (None, Some(r)) => (Some(r), None),
            other => other,
        };
        let pending = below + u64::from(far.is_some());
        self.reach(pending + u64::from(near.is_some()));
        for (entry, under) in [(near, pending), (far, below)] {
            if let Some((child, enter)) = entry {
                self.enter(child, enter, under);
            }
        }
    }

    fn leaf(&mut self, leaf: CompactChild) {
        self.counts.leaf_visits += 1;
        let start = leaf.first as usize;
        for &primitive in &self.scene.prims[start..start + leaf.prim_count as usize] {
            self.counts.primitive_tests += 1;
            // An any-hit closure takes no limit; its calls are recorded with `FAR`.
            self.calls.push((primitive, if self.stop_at_first { FAR } else { self.limit }));
            let Some(distance) = self.scene.distance(self.ray, primitive, self.limit) else {
                continue;
            };
            let distance = if self.stop_at_first { FixedQ32_32::ZERO } else { distance };
            let better =
                self.best.is_none_or(|b| (distance, primitive) < (b.distance, b.primitive));
            if (FixedQ32_32::ZERO..=self.limit).contains(&distance) && better {
                (self.best, self.limit) = (Some(ClosestHit { primitive, distance }), distance);
                self.done = self.stop_at_first;
                if self.done {
                    return;
                }
            }
        }
    }
}

type Traced = (Option<ClosestHit>, TraversalCounts, Vec<(u32, FixedQ32_32)>);

fn reference(scene: &Scene, ray: &TraceRay, t_max: FixedQ32_32, stop_at_first: bool) -> Traced {
    let mut search = Reference {
        scene,
        ray,
        limit: t_max,
        stop_at_first,
        best: None,
        done: false,
        counts: TraversalCounts::default(),
        calls: Vec::new(),
    };
    search.search();
    (search.best, search.counts, search.calls)
}

/// Builder, triangle count and rays per scene.
fn corpus() -> Vec<(Builder, usize, usize)> {
    let builders: [Builder; 2] = [build_bvh, build_bvh_sah];
    builders
        .into_iter()
        .flat_map(|b| [(b, 1, 50), (b, 2, 100), (b, 7, 200), (b, 40, 300), (b, 300, 600)])
        .collect()
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_416_counts_equal_a_recursive_reference_search() {
    let mut seeds = Seeds(COUNT_SEED);
    let (mut hits, mut skips) = (0, 0);
    for (case, (build, count, rays)) in corpus().into_iter().enumerate() {
        let scene = Scene::seeded(&mut seeds, count, build);
        for _ in 0..rays {
            let state = seeds.0;
            let Some(ray) = seeded_ray(&mut seeds) else { continue };
            let expected = reference(&scene, &ray, FAR, false);
            let limits = expected.0.map_or(vec![FAR], |hit| vec![FAR, hit.distance]);
            for t_max in limits {
                let expected = reference(&scene, &ray, t_max, false);
                let mut calls = Vec::new();
                let found = compact_closest_hit_counted(scene.view(), &ray, t_max, |p, limit| {
                    calls.push((p, limit));
                    scene.distance(&ray, p, limit)
                });
                let found = found.map(|(hit, counts)| (hit, counts, calls));
                assert_eq!(found, Ok(expected.clone()), "closest: case {case}, seed {state:#x}");
                let any = reference(&scene, &ray, t_max, true);
                let mut calls = Vec::new();
                let found = compact_any_hit_counted(scene.view(), &ray, t_max, |p| {
                    calls.push((p, FAR));
                    scene.distance(&ray, p, t_max).is_some()
                });
                let found = found.map(|(hit, counts)| (hit, counts, calls));
                assert_eq!(
                    found,
                    Ok((any.0.is_some(), any.1, any.2)),
                    "any: case {case}, seed {state:#x}"
                );
                hits += usize::from(expected.0.is_some());
                skips += usize::from(expected.1.popped_skipped > 0);
            }
        }
    }
    assert!(hits > 500 && skips > 100, "corpus needs hits and skipped pops: {hits}, {skips}");
}

/// A seeded fault in one reference of the tree: a child out of range, a child naming any node
/// (a cycle or a shared subtree), or a leaf range past the buffer.
fn corrupt(scene: &mut Scene, seeds: &mut Seeds) {
    let count = u32::try_from(scene.nodes.len()).expect("small");
    let past = u32::try_from(scene.prims.len()).expect("small");
    let fault = match seeds.below(3) {
        0 => node(count + u32::try_from(seeds.below(3)).expect("small")),
        1 => node(u32::try_from(seeds.below(scene.nodes.len().max(1))).expect("small")),
        _ => CompactChild { first: past.saturating_sub(1), prim_count: 2 },
    };
    match scene.nodes.len() {
        0 => scene.root = scene.root.map(|root| CompactRoot { child: fault, ..root }),
        n => scene.nodes[seeds.below(n)].children[seeds.below(2)] = fault,
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_417_counted_and_uncounted_searches_agree_on_valid_and_malformed_trees() {
    let mut seeds = Seeds(COUNT_SEED ^ 1);
    let mut scenes = Vec::new();
    for (build, count, _) in corpus() {
        scenes.push(Scene::seeded(&mut seeds, count, build));
        let mut broken = Scene::seeded(&mut seeds, count, build);
        corrupt(&mut broken, &mut seeds);
        scenes.push(broken);
    }
    // Each node keeps a leaf underneath and descends into the next: past the 64-entry stack.
    let chain = (1..=70)
        .map(|next| CompactNode { bounds: [slab(-8, 8); 2], children: [node(next), leaf(0)] });
    let root = Some(CompactRoot { bounds: slab(-8, 8), child: node(0) });
    let prims = vec![0];
    scenes.push(Scene { triangles: Vec::new(), root, nodes: chain.collect(), prims });
    let (mut errors, mut compared) = (0, 0);
    for (case, scene) in scenes.iter().enumerate() {
        for _ in 0..200 {
            let state = seeds.0;
            let Some(mut ray) = seeded_ray(&mut seeds) else { continue };
            if case + 1 == scenes.len() {
                ray = TraceRay::new(vec3([0, 0, 0]), vec3([0, 0, -ONE_RAW])).expect("in domain");
            }
            let mut calls = [Vec::new(), Vec::new()];
            let [plain_calls, counted_calls] = &mut calls;
            let plain = compact_closest_hit(scene.view(), &ray, FAR, |p, limit| {
                plain_calls.push((p, limit));
                scene.distance(&ray, p, limit)
            });
            let counted = compact_closest_hit_counted(scene.view(), &ray, FAR, |p, limit| {
                counted_calls.push((p, limit));
                scene.distance(&ray, p, limit)
            });
            assert_eq!(counted.map(|c| c.0), plain, "closest: case {case}, seed {state:#x}");
            assert_eq!(calls[0], calls[1], "closest calls: case {case}, seed {state:#x}");
            let mut any_calls = [Vec::new(), Vec::new()];
            let [plain_calls, counted_calls] = &mut any_calls;
            let any = compact_any_hit(scene.view(), &ray, FAR, |p| {
                plain_calls.push(p);
                scene.distance(&ray, p, FAR).is_some()
            });
            let counted_any = compact_any_hit_counted(scene.view(), &ray, FAR, |p| {
                counted_calls.push(p);
                scene.distance(&ray, p, FAR).is_some()
            });
            assert_eq!(counted_any.map(|c| c.0), any, "any: case {case}, seed {state:#x}");
            assert_eq!(any_calls[0], any_calls[1], "any calls: case {case}, seed {state:#x}");
            errors += usize::from(plain.is_err());
            compared += 1;
        }
    }
    assert!(errors > 150 && compared > 4_000, "{errors} errors in {compared} comparisons");
}
