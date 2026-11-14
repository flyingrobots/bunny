//! Counters for `closest_hit` and `any_hit` on the benchmark scene, and kernel replay cases.
//!
//! The core crates carry no counters. This module re-runs the traversal in the harness: a
//! replica of the search loop built on the public [`TraceRay::box_entry`], [`BvhNode`] data
//! and the same primitive closure the benchmark uses. A test checks that the replica tests
//! the same primitives, with the same limits and in the same order, as `closest_hit` and
//! `any_hit` on every benchmark ray, and returns the same hit.
//!
//! The replica also records every box test and triangle test it makes, so the kernel cases
//! can replay exactly those calls and time them in isolation (see [`kernel_cases`]).

use std::collections::{BTreeMap, HashSet};
use std::fmt::Write as _;
use std::hint::black_box;

use bunny_broadphase::BvhNode;
use bunny_num::FixedQ32_32;
use bunny_query::{TraceRay, TraceTriangle};

use crate::cases::{case, Case};
use crate::scene::{scene, trace_rays, trace_triangles, Scene, RAYS_PER_SIDE};

const FAR: FixedQ32_32 = FixedQ32_32::from_raw(i64::MAX);

macro_rules! counters {
    ($($(#[$doc:meta])* $name:ident),* $(,)?) => {
        /// Event counts of one ray, or the sum over several rays.
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
        pub(crate) struct Counts {
            $($(#[$doc])* pub(crate) $name: u64,)*
        }

        impl Counts {
            fn add(&mut self, other: &Self) {
                $(self.$name += other.$name;)*
            }

            /// Every counter with its name, in table order.
            pub(crate) fn fields(&self) -> Vec<(&'static str, u64)> {
                vec![$((stringify!($name), self.$name)),*]
            }
        }
    };
}

counters! {
    /// Stack entries popped.
    popped,
    /// Popped entries dropped because their entry bound exceeded the best distance.
    skipped_at_pop,
    /// Interior nodes visited.
    interior_visited,
    /// Leaves visited.
    leaf_visited,
    /// Calls to `box_entry`.
    box_calls,
    /// `box_entry` calls that returned a bound.
    box_some,
    /// `box_entry` calls that rejected the box.
    box_none,
    /// Interior visits with both children pushed.
    children_both,
    /// Interior visits with one child pushed.
    children_one,
    /// Interior visits with no child pushed.
    children_none,
    /// Largest stack depth reached.
    stack_peak,
    /// Triangle tests.
    tri_tests,
    /// Tests that failed at the determinant (ray parallel to the plane).
    fail_determinant,
    /// Tests that failed the first barycentric check.
    fail_first_barycentric,
    /// Tests that failed the second barycentric check.
    fail_second_barycentric,
    /// Tests that passed the edge checks but lie behind the origin (no division).
    behind_origin,
    /// Tests that reached the division.
    divided,
    /// Divided tests whose distance does not fit (none expected).
    divided_overflow,
    /// Divided tests rejected by the distance limit.
    divided_beyond_limit,
    /// Divided tests in range but not better than the best (ties).
    divided_not_better,
    /// Divided tests accepted as the new best.
    divided_accepted,
    /// Tests where the replica's stage disagreed with `hit_triangle` (must be zero).
    stage_mismatch,
}

/// How far a triangle test got.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stage {
    /// Zero determinant.
    Determinant,
    /// `u` outside `0..=det`.
    FirstBarycentric,
    /// `v < 0` or `u + v > det`.
    SecondBarycentric,
    /// Edge checks passed, numerator of the distance negative.
    Behind,
    /// The division ran.
    Divided,
}

type Wide3 = [i128; 3];

fn wide(v: bunny_linalg::FixedVec3) -> Wide3 {
    [v.x, v.y, v.z].map(|c| i128::from(c.raw()))
}

fn sub(a: Wide3, b: Wide3) -> Wide3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross([a0, a1, a2]: Wide3, [b0, b1, b2]: Wide3) -> Wide3 {
    [a1 * b2 - a2 * b1, a2 * b0 - a0 * b2, a0 * b1 - a1 * b0]
}

fn dot([a0, a1, a2]: Wide3, [b0, b1, b2]: Wide3) -> i128 {
    a0 * b0 + a1 * b1 + a2 * b2
}

/// The decision stages of `TraceRay::hit_triangle`, redone from its public inputs in `i128`.
pub(crate) fn stage(ray: &TraceRay, triangle: &TraceTriangle) -> Stage {
    let [v0, v1, v2] = triangle.vertices().map(wide);
    let (edge1, edge2) = (sub(v1, v0), sub(v2, v0));
    let direction = wide(ray.direction());
    let pvec = cross(direction, edge2);
    let signed_det = dot(edge1, pvec);
    let sign = signed_det.signum();
    if sign == 0 {
        return Stage::Determinant;
    }
    let det = signed_det * sign;
    let tvec = sub(wide(ray.origin()), v0);
    let u = dot(tvec, pvec) * sign;
    if !(0..=det).contains(&u) {
        return Stage::FirstBarycentric;
    }
    let qvec = cross(tvec, edge1);
    let v = dot(direction, qvec) * sign;
    if v < 0 || u + v > det {
        return Stage::SecondBarycentric;
    }
    if dot(edge2, qvec) * sign < 0 {
        return Stage::Behind;
    }
    Stage::Divided
}

/// Which search to replicate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    /// `closest_hit`.
    Closest,
    /// `any_hit`.
    Any,
}

/// One triangle test the search made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Test {
    pub(crate) primitive: u32,
    pub(crate) limit: FixedQ32_32,
    pub(crate) stage: Stage,
}

/// Everything one replicated search did.
pub(crate) struct Walk {
    pub(crate) counts: Counts,
    /// The accepted hit: primitive and distance (any hit reports distance zero).
    pub(crate) hit: Option<(u32, FixedQ32_32)>,
    /// `(node, limit)` of every `box_entry` call, in order.
    pub(crate) boxes: Vec<(u32, FixedQ32_32)>,
    /// Every triangle test, in order.
    pub(crate) tests: Vec<Test>,
}

/// The scene with its prepared triangles and rays.
pub(crate) struct Rig {
    pub(crate) scene: Scene,
    pub(crate) triangles: Vec<TraceTriangle>,
    pub(crate) rays: Vec<TraceRay>,
}

impl Rig {
    pub(crate) fn new() -> Self {
        let scene = scene();
        let triangles = trace_triangles(&scene);
        Self { scene, triangles, rays: trace_rays() }
    }
}

type Entry = (u32, Option<FixedQ32_32>);

struct Walker<'a> {
    rig: &'a Rig,
    ray: &'a TraceRay,
    mode: Mode,
    limit: FixedQ32_32,
    stack: Vec<(u32, FixedQ32_32)>,
    done: bool,
    out: Walk,
}

/// Replicates the search loop of `closest_hit` or `any_hit` for one ray, with counters.
pub(crate) fn walk(rig: &Rig, ray: &TraceRay, mode: Mode) -> Walk {
    let out = Walk { counts: Counts::default(), hit: None, boxes: Vec::new(), tests: Vec::new() };
    let mut walker = Walker { rig, ray, mode, limit: FAR, stack: Vec::new(), done: false, out };
    walker.run();
    walker.out
}

impl Walker<'_> {
    fn run(&mut self) {
        if !self.rig.scene.nodes.is_empty() {
            let root = self.entry(0);
            self.push((0, root));
        }
        while let Some((index, enter)) = self.stack.pop() {
            self.out.counts.popped += 1;
            if enter > self.limit {
                self.out.counts.skipped_at_pop += 1;
                continue;
            }
            self.visit(index);
            if self.done {
                break;
            }
        }
    }

    fn entry(&mut self, index: u32) -> Option<FixedQ32_32> {
        let node = self.rig.scene.nodes.get(index as usize)?;
        let found = self.ray.box_entry(&node.bounds, self.limit);
        let counts = &mut self.out.counts;
        counts.box_calls += 1;
        if found.is_some() {
            counts.box_some += 1;
        } else {
            counts.box_none += 1;
        }
        self.out.boxes.push((index, self.limit));
        found
    }

    fn push(&mut self, (index, enter): Entry) {
        if let Some(enter) = enter {
            self.stack.push((index, enter));
            let depth = self.stack.len() as u64;
            self.out.counts.stack_peak = self.out.counts.stack_peak.max(depth);
        }
    }

    fn visit(&mut self, index: u32) {
        let Some(&BvhNode { first_child_or_prim_idx: first, prim_count, .. }) =
            self.rig.scene.nodes.get(index as usize)
        else {
            return;
        };
        if prim_count > 0 {
            self.out.counts.leaf_visited += 1;
            self.visit_leaf(first as usize, prim_count as usize);
            return;
        }
        self.out.counts.interior_visited += 1;
        let (left, right) = (self.entry(first), self.entry(first + 1));
        let kept = u8::from(left.is_some()) + u8::from(right.is_some());
        let counts = &mut self.out.counts;
        match kept {
            2 => counts.children_both += 1,
            1 => counts.children_one += 1,
            _ => counts.children_none += 1,
        }
        let (near, far) = nearer_first((first, left), (first + 1, right));
        self.push(far);
        self.push(near);
    }

    fn visit_leaf(&mut self, start: usize, count: usize) {
        let Some(primitives) = self.rig.scene.prim_indices.get(start..start + count) else {
            return;
        };
        for &primitive in primitives {
            if self.test(primitive) && self.mode == Mode::Any {
                self.done = true;
                return;
            }
        }
    }

    /// Tests one primitive as the benchmark closure does; returns whether the hit was kept.
    fn test(&mut self, primitive: u32) -> bool {
        let Some(triangle) = self.rig.triangles.get(primitive as usize) else {
            return false;
        };
        let limit = if self.mode == Mode::Any { FAR } else { self.limit };
        let stage = stage(self.ray, triangle);
        self.out.tests.push(Test { primitive, limit, stage });
        self.count_stage(stage);
        let found = self.ray.hit_triangle(triangle, FixedQ32_32::ZERO, limit).map(|h| h.distance);
        if (stage == Stage::Divided) != found.is_some() {
            self.classify_divided(triangle, found);
        }
        let Some(distance) = found else { return false };
        let distance = if self.mode == Mode::Any { FixedQ32_32::ZERO } else { distance };
        let kept = self.accept(primitive, distance);
        let counts = &mut self.out.counts;
        if stage == Stage::Divided {
            if kept {
                counts.divided_accepted += 1;
            } else {
                counts.divided_not_better += 1;
            }
        }
        kept
    }

    /// A divided test with no result is either beyond the limit or an overflow; a result
    /// without a division is a replica error.
    fn classify_divided(&mut self, triangle: &TraceTriangle, found: Option<FixedQ32_32>) {
        let counts = &mut self.out.counts;
        if found.is_some() {
            counts.stage_mismatch += 1;
            return;
        }
        match self.ray.hit_triangle(triangle, FixedQ32_32::ZERO, FAR) {
            Some(_) => counts.divided_beyond_limit += 1,
            None => counts.divided_overflow += 1,
        }
    }

    fn count_stage(&mut self, stage: Stage) {
        let counts = &mut self.out.counts;
        counts.tri_tests += 1;
        match stage {
            Stage::Determinant => counts.fail_determinant += 1,
            Stage::FirstBarycentric => counts.fail_first_barycentric += 1,
            Stage::SecondBarycentric => counts.fail_second_barycentric += 1,
            Stage::Behind => counts.behind_origin += 1,
            Stage::Divided => counts.divided += 1,
        }
    }

    fn accept(&mut self, primitive: u32, distance: FixedQ32_32) -> bool {
        let better = (FixedQ32_32::ZERO..=self.limit).contains(&distance)
            && self.out.hit.is_none_or(|(p, d)| (distance, primitive) < (d, p));
        if better {
            self.out.hit = Some((primitive, distance));
            self.limit = distance;
        }
        better
    }
}

/// Nearer entry bound first; a tie keeps the left child first.
fn nearer_first(left: Entry, right: Entry) -> (Entry, Entry) {
    match (left.1, right.1) {
        (Some(l), Some(r)) if r < l => (right, left),
        _ => (left, right),
    }
}

/// Which kind of ray a benchmark ray is, by what `closest_hit` finds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Class {
    Sphere,
    Ground,
    Miss,
}

const CLASSES: [(Class, &str); 3] = [
    (Class::Sphere, "hit the sphere"),
    (Class::Ground, "hit only the ground"),
    (Class::Miss, "hit nothing"),
];

fn class_of(rig: &Rig, hit: Option<(u32, FixedQ32_32)>) -> Class {
    let sphere = rig.triangles.len().saturating_sub(2);
    match hit {
        None => Class::Miss,
        Some((primitive, _)) if (primitive as usize) < sphere => Class::Sphere,
        Some(_) => Class::Ground,
    }
}

/// Counts summed per class of ray, with the number of rays in each.
pub(crate) fn totals(rig: &Rig, mode: Mode) -> BTreeMap<Class, (u64, Counts)> {
    let mut sums: BTreeMap<Class, (u64, Counts)> = BTreeMap::new();
    for ray in &rig.rays {
        let found = walk(rig, ray, Mode::Closest);
        // Any-hit rays are classed by what the closest hit finds, so the columns match.
        let class = class_of(rig, found.hit);
        let counts = if mode == Mode::Closest { found.counts } else { walk(rig, ray, mode).counts };
        let entry = sums.entry(class).or_default();
        entry.0 += 1;
        entry.1.add(&counts);
    }
    sums
}

#[allow(clippy::cast_precision_loss)]
fn per_ray(total: u64, rays: u64) -> f64 {
    if rays == 0 {
        0.0
    } else {
        total as f64 / rays as f64
    }
}

fn render_counts(rig: &Rig, mode: Mode) -> String {
    let sums = totals(rig, mode);
    let mut all = Counts::default();
    let mut rays = 0;
    for (n, counts) in sums.values() {
        rays += n;
        all.add(counts);
    }
    let mut out = String::from("| counter per ray | all rays |");
    for (_, label) in CLASSES {
        let _ = write!(out, " {label} |");
    }
    out.push_str("\n| --- | ---: | ---: | ---: | ---: |\n| rays |");
    let _ = write!(out, " {rays} |");
    for (class, _) in CLASSES {
        let _ = write!(out, " {} |", sums.get(&class).map_or(0, |s| s.0));
    }
    out.push('\n');
    for (row, (name, total)) in all.fields().into_iter().enumerate() {
        let _ = write!(out, "| `{name}` | {:.2} |", per_ray(total, rays));
        for (class, _) in CLASSES {
            let (n, counts) = sums.get(&class).copied().unwrap_or_default();
            let value = counts.fields().get(row).map_or(0, |f| f.1);
            let _ = write!(out, " {:.2} |", per_ray(value, n));
        }
        out.push('\n');
    }
    out
}

/// Shape of the tree.
pub(crate) struct TreeStats {
    pub(crate) nodes: usize,
    pub(crate) leaves: usize,
    pub(crate) max_depth: usize,
    pub(crate) mean_leaf_depth: f64,
    pub(crate) leaf_sizes: BTreeMap<u32, usize>,
    pub(crate) primitives: usize,
    /// Sum over interior nodes of surface area over root surface area.
    pub(crate) interior_area: f64,
    /// Sum over leaves of primitive count times surface area over root surface area.
    pub(crate) leaf_area: f64,
}

#[allow(clippy::cast_precision_loss)]
fn area(node: &BvhNode) -> f64 {
    let side = |lo: FixedQ32_32, hi: FixedQ32_32| (hi.raw() - lo.raw()) as f64 / 4_294_967_296.0;
    let (lo, hi) = (&node.bounds.min, &node.bounds.max);
    let (dx, dy, dz) = (side(lo.x, hi.x), side(lo.y, hi.y), side(lo.z, hi.z));
    2.0 * (dx * dy + dy * dz + dz * dx)
}

#[allow(clippy::cast_precision_loss)]
pub(crate) fn tree_stats(nodes: &[BvhNode]) -> TreeStats {
    let root_area = nodes.first().map_or(1.0, area);
    let mut stats = TreeStats {
        nodes: nodes.len(),
        leaves: 0,
        max_depth: 0,
        mean_leaf_depth: 0.0,
        leaf_sizes: BTreeMap::new(),
        primitives: 0,
        interior_area: 0.0,
        leaf_area: 0.0,
    };
    let mut depth_sum = 0_usize;
    let mut stack = vec![(0_u32, 0_usize)];
    while let Some((index, depth)) = stack.pop() {
        let Some(node) = nodes.get(index as usize) else { continue };
        let share = area(node) / root_area;
        if node.prim_count == 0 {
            stats.interior_area += share;
            stack.push((node.first_child_or_prim_idx, depth + 1));
            stack.push((node.first_child_or_prim_idx + 1, depth + 1));
            continue;
        }
        stats.leaves += 1;
        stats.max_depth = stats.max_depth.max(depth);
        depth_sum += depth;
        stats.primitives += node.prim_count as usize;
        *stats.leaf_sizes.entry(node.prim_count).or_default() += 1;
        stats.leaf_area += f64::from(node.prim_count) * share;
    }
    stats.mean_leaf_depth = depth_sum as f64 / stats.leaves.max(1) as f64;
    stats
}

fn render_tree(rig: &Rig) -> String {
    let t = tree_stats(&rig.scene.nodes);
    let sizes: Vec<String> = t.leaf_sizes.iter().map(|(k, v)| format!("{k}: {v}")).collect();
    format!(
        "| nodes | {} |\n| leaves | {} |\n| interior nodes | {} |\n| primitives in leaves | {} |\n\
         | maximum leaf depth (root is 0) | {} |\n| mean leaf depth | {:.2} |\n\
         | leaf sizes (primitives: leaves) | {} |\n\
         | interior area / root area | {:.2} |\n| primitive-weighted leaf area / root area | {:.2} |\n",
        t.nodes,
        t.leaves,
        t.nodes - t.leaves,
        t.primitives,
        t.max_depth,
        t.mean_leaf_depth,
        sizes.join(", "),
        t.interior_area,
        t.leaf_area,
    )
}

/// For square blocks of neighbouring rays (the camera grid is row-major): the box tests the
/// rays make in total, and the number of distinct nodes they test. Their ratio bounds what
/// a packet that tests each node once for the whole block could share.
pub(crate) fn packet_sharing(rig: &Rig, block: usize) -> (usize, usize) {
    let side = usize::try_from(RAYS_PER_SIDE).unwrap_or(0);
    let tested: Vec<Vec<u32>> = rig
        .rays
        .iter()
        .map(|ray| walk(rig, ray, Mode::Closest).boxes.iter().map(|b| b.0).collect())
        .collect();
    let (mut total, mut distinct) = (0, 0);
    for row in (0..side).step_by(block) {
        for column in (0..side).step_by(block) {
            let mut seen = HashSet::new();
            for (r, c) in
                (row..row + block).flat_map(|r| (column..column + block).map(move |c| (r, c)))
            {
                for &node in tested.get(r * side + c).into_iter().flatten() {
                    total += 1;
                    seen.insert(node);
                }
            }
            distinct += seen.len();
        }
    }
    (total, distinct)
}

fn render_packets(rig: &Rig) -> String {
    let mut out = String::from(
        "| block | box tests, summed | distinct nodes | shared |\n| --- | ---: | ---: | ---: |\n",
    );
    for block in [1, 2, 4, 8] {
        let (total, distinct) = packet_sharing(rig, block);
        let shared = 100.0 * (1.0 - per_ray(distinct as u64, total as u64));
        let _ = writeln!(out, "| {block} x {block} rays | {total} | {distinct} | {shared:.0} % |");
    }
    out
}

/// The `--stats` report, as Markdown tables.
pub(crate) fn report() -> String {
    let rig = Rig::new();
    format!(
        "## BVH\n\n| | |\n| --- | ---: |\n{}\n## closest_hit\n\n{}\n## any_hit\n\n{}\n## Neighbouring rays\n\n{}",
        render_tree(&rig),
        render_counts(&rig, Mode::Closest),
        render_counts(&rig, Mode::Any),
        render_packets(&rig),
    )
}

/// Timed replays of the kernel calls the traversal makes on the benchmark rays.
///
/// Each case replays the recorded calls of all 1 024 `closest_hit` searches, so its time is
/// per ray: what that kernel costs one ray, outside the traversal. The replays read the real
/// node boxes in the order the search read them. The `_chain` cases make each call's limit
/// depend on the previous result, which measures latency instead of overlapped throughput.
pub(crate) fn kernel_cases() -> Vec<Case> {
    use std::rc::Rc;

    let rig = Rc::new(Rig::new());
    let mut boxes = Vec::new();
    let mut rejected = Vec::new();
    let mut divided = Vec::new();
    for (ray, path) in rig.rays.iter().enumerate() {
        let found = walk(&rig, path, Mode::Closest);
        boxes.extend(found.boxes.iter().map(|&(node, limit)| (ray, node, limit)));
        for t in found.tests {
            let list = if t.stage == Stage::Divided { &mut divided } else { &mut rejected };
            list.push((ray, t.primitive, t.limit));
        }
    }
    let all_tests: Vec<_> = {
        let mut v = rejected.clone();
        v.extend(divided.iter().copied());
        v
    };
    let (boxes, rejected, divided) = (Rc::new(boxes), Rc::new(rejected), Rc::new(divided));
    let all_tests = Rc::new(all_tests);
    vec![
        case("trace/kernel_box_entry", {
            let (rig, calls) = (Rc::clone(&rig), Rc::clone(&boxes));
            move || replay_boxes(&rig, &calls, false)
        }),
        case("trace/kernel_box_entry_chain", {
            let (rig, calls) = (Rc::clone(&rig), Rc::clone(&boxes));
            move || replay_boxes(&rig, &calls, true)
        }),
        case("trace/kernel_tri_rejected", {
            let (rig, calls) = (Rc::clone(&rig), Rc::clone(&rejected));
            move || replay_triangles(&rig, &calls, false)
        }),
        case("trace/kernel_tri_divided", {
            let (rig, calls) = (Rc::clone(&rig), Rc::clone(&divided));
            move || replay_triangles(&rig, &calls, false)
        }),
        case("trace/kernel_tri_all_chain", {
            let (rig, calls) = (Rc::clone(&rig), Rc::clone(&all_tests));
            move || replay_triangles(&rig, &calls, true)
        }),
    ]
}

fn replay_boxes(rig: &Rig, calls: &[(usize, u32, FixedQ32_32)], chain: bool) -> i64 {
    let mut total = 0_i64;
    let mut previous = 0_i64;
    for &(ray, node, limit) in calls {
        let (Some(ray), Some(node)) = (rig.rays.get(ray), rig.scene.nodes.get(node as usize))
        else {
            continue;
        };
        let tie = if chain { previous & black_box(0) } else { 0 };
        let limit = FixedQ32_32::from_raw(limit.raw() | tie);
        previous =
            black_box(ray.box_entry(&node.bounds, black_box(limit))).map_or(0, FixedQ32_32::raw);
        total = total.wrapping_add(previous);
    }
    total
}

fn replay_triangles(rig: &Rig, calls: &[(usize, u32, FixedQ32_32)], chain: bool) -> i64 {
    let mut total = 0_i64;
    let mut previous = 0_i64;
    for &(ray, primitive, limit) in calls {
        let (Some(ray), Some(triangle)) =
            (rig.rays.get(ray), rig.triangles.get(primitive as usize))
        else {
            continue;
        };
        let tie = if chain { previous & black_box(0) } else { 0 };
        let limit = FixedQ32_32::from_raw(limit.raw() | tie);
        let hit = black_box(ray.hit_triangle(triangle, FixedQ32_32::ZERO, black_box(limit)));
        previous = hit.map_or(0, |h| h.distance.raw());
        total = total.wrapping_add(previous);
    }
    total
}

#[cfg(test)]
mod tests {
    use bunny_broadphase::{any_hit, closest_hit, BvhView};

    use super::*;

    type RealClosest = (Vec<(u32, FixedQ32_32)>, Option<(u32, i64)>);

    /// Primitive, limit and result of every call `closest_hit` makes to the closure.
    fn real_closest(rig: &Rig, ray: &TraceRay) -> RealClosest {
        let mut calls = Vec::new();
        let bvh = BvhView::new(&rig.scene.nodes, &rig.scene.prim_indices);
        let hit = closest_hit(bvh, ray, FAR, |primitive, limit| {
            calls.push((primitive, limit));
            let triangle = rig.triangles.get(primitive as usize)?;
            ray.hit_triangle(triangle, FixedQ32_32::ZERO, limit).map(|h| h.distance)
        });
        let hit = hit.ok().flatten().map(|h| (h.primitive, h.distance.raw()));
        (calls, hit)
    }

    fn real_any(rig: &Rig, ray: &TraceRay) -> (Vec<u32>, bool) {
        let mut calls = Vec::new();
        let bvh = BvhView::new(&rig.scene.nodes, &rig.scene.prim_indices);
        let found = any_hit(bvh, ray, FAR, |primitive| {
            calls.push(primitive);
            rig.triangles
                .get(primitive as usize)
                .is_some_and(|t| ray.hit_triangle(t, FixedQ32_32::ZERO, FAR).is_some())
        });
        (calls, found.unwrap_or(false))
    }

    #[test]
    fn the_replica_closest_search_agrees_with_closest_hit_on_every_ray() {
        let rig = Rig::new();
        let mut bad = Vec::new();
        for (index, ray) in rig.rays.iter().enumerate() {
            let (calls, hit) = real_closest(&rig, ray);
            let replica = walk(&rig, ray, Mode::Closest);
            let tested: Vec<_> = replica.tests.iter().map(|t| (t.primitive, t.limit)).collect();
            let found = replica.hit.map(|(p, d)| (p, d.raw()));
            if tested != calls || found != hit || replica.counts.stage_mismatch != 0 {
                bad.push(index);
            }
        }
        assert_eq!(bad, Vec::<usize>::new());
    }

    #[test]
    fn the_replica_any_search_agrees_with_any_hit_on_every_ray() {
        let rig = Rig::new();
        let mut bad = Vec::new();
        for (index, ray) in rig.rays.iter().enumerate() {
            let (calls, found) = real_any(&rig, ray);
            let replica = walk(&rig, ray, Mode::Any);
            let tested: Vec<_> = replica.tests.iter().map(|t| t.primitive).collect();
            if tested != calls || replica.hit.is_some() != found {
                bad.push(index);
            }
        }
        assert_eq!(bad, Vec::<usize>::new());
    }

    #[test]
    fn counters_obey_their_identities() {
        let rig = Rig::new();
        for ray in &rig.rays {
            let c = walk(&rig, ray, Mode::Closest).counts;
            let visited = c.interior_visited + c.leaf_visited;
            assert_eq!(c.popped, c.skipped_at_pop + visited);
            assert_eq!(c.box_calls, c.box_some + c.box_none);
            assert_eq!(c.box_calls, 1 + 2 * c.interior_visited);
            assert_eq!(c.interior_visited, c.children_both + c.children_one + c.children_none);
            let staged = c.fail_determinant
                + c.fail_first_barycentric
                + c.fail_second_barycentric
                + c.behind_origin
                + c.divided;
            assert_eq!(c.tri_tests, staged);
            assert_eq!(
                c.divided,
                c.divided_overflow
                    + c.divided_beyond_limit
                    + c.divided_not_better
                    + c.divided_accepted
            );
            assert_eq!(c.divided_overflow, 0);
        }
    }

    #[test]
    fn the_tree_statistics_account_for_every_primitive() {
        let rig = Rig::new();
        let t = tree_stats(&rig.scene.nodes);
        assert_eq!(t.primitives, rig.triangles.len());
        assert_eq!(t.nodes, 2 * t.leaves - 1);
        assert_eq!(t.leaf_sizes.values().sum::<usize>(), t.leaves);
        assert!(t.max_depth < 64);
    }

    #[test]
    fn packet_sharing_is_exact_for_one_ray_and_never_negative() {
        let rig = Rig::new();
        let (total, distinct) = packet_sharing(&rig, 1);
        assert_eq!(total, distinct);
        let (total_four, distinct_four) = packet_sharing(&rig, 4);
        assert_eq!(total_four, total);
        assert!(distinct_four < total_four);
    }

    #[test]
    fn rays_split_into_the_three_classes() {
        let rig = Rig::new();
        let sums = totals(&rig, Mode::Closest);
        assert_eq!(sums.values().map(|s| s.0).sum::<u64>(), 1024);
        assert!(sums.contains_key(&Class::Sphere));
    }

    #[test]
    fn the_stage_replica_matches_hit_triangle_on_a_hand_built_case() {
        use bunny_linalg::FixedVec3;
        use bunny_num::fixed_q32_32::ONE_RAW;
        let units = |n: i64| FixedQ32_32::from_raw(n * ONE_RAW);
        let at = |x, y, z| FixedVec3::new(units(x), units(y), units(z));
        let triangle = TraceTriangle::new(at(-1, -1, 0), at(1, -1, 0), at(0, 1, 0));
        let triangle = triangle.expect("in domain");
        let shoot = |origin, direction| TraceRay::new(origin, direction).expect("in domain");
        let down = at(0, 0, -1);
        assert_eq!(stage(&shoot(at(0, 0, 5), down), &triangle), Stage::Divided);
        assert_eq!(stage(&shoot(at(0, 0, -5), down), &triangle), Stage::Behind);
        assert_eq!(stage(&shoot(at(0, 0, 5), at(1, 0, 0)), &triangle), Stage::Determinant);
        assert_eq!(stage(&shoot(at(5, 0, 5), down), &triangle), Stage::FirstBarycentric);
        assert_eq!(stage(&shoot(at(1, 1, 5), down), &triangle), Stage::SecondBarycentric);
    }
}
