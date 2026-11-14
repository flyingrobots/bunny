//! The counted compact searches (`compact_closest_hit_counted`, `compact_any_hit_counted`) on
//! the benchmark scene: `trace/closest_hit_compact_counted` and `trace/any_hit_compact_counted`
//! trace the same 1 024 rays through the same compact tree as `trace/closest_hit_compact` and
//! `trace/any_hit_compact`, so the difference is the cost of counting.

use std::hint::black_box;
use std::rc::Rc;

use bunny_broadphase::{
    compact_any_hit_counted, compact_bvh, compact_closest_hit_counted, CompactChild, CompactNode,
    CompactRoot, CompactView, TraversalCounts,
};
use bunny_geom::FixedAabb3;
use bunny_linalg::FixedVec3;
use bunny_num::FixedQ32_32;
use bunny_query::{TraceRay, TraceTriangle};

use crate::cases::{case, Case};
use crate::scene::{scene, trace_rays, trace_triangles, Scene};

const FAR: FixedQ32_32 = FixedQ32_32::from_raw(i64::MAX);

/// The scene, its compact tree, its prepared triangles and the camera rays.
struct Data {
    scene: Scene,
    root: Option<CompactRoot>,
    nodes: Vec<CompactNode>,
    triangles: Vec<TraceTriangle>,
    rays: Vec<TraceRay>,
}

impl Data {
    fn new() -> Self {
        let scene = scene();
        let zero = FixedVec3::new(FixedQ32_32::ZERO, FixedQ32_32::ZERO, FixedQ32_32::ZERO);
        let empty = CompactNode {
            bounds: [FixedAabb3::new(zero, zero); 2],
            children: [CompactChild::default(); 2],
        };
        let mut nodes = vec![empty; scene.nodes.len() / 2];
        let (root, used) = match compact_bvh(&scene.nodes, &mut nodes) {
            Ok(Some((root, used))) => (Some(root), used),
            _ => (None, 0),
        };
        nodes.truncate(used);
        Self { triangles: trace_triangles(&scene), rays: trace_rays(), scene, root, nodes }
    }

    fn view(&self) -> CompactView<'_> {
        CompactView { root: self.root, nodes: &self.nodes, prim_indices: &self.scene.prim_indices }
    }

    /// The benchmark's closest-hit search of one ray, with its counts.
    fn closest(&self, ray: &TraceRay) -> Option<(Option<i64>, TraversalCounts)> {
        let found = compact_closest_hit_counted(self.view(), ray, FAR, |primitive, limit| {
            let triangle = self.triangles.get(primitive as usize)?;
            ray.hit_triangle(triangle, FixedQ32_32::ZERO, limit).map(|hit| hit.distance)
        });
        found.ok().map(|(hit, counts)| (hit.map(|hit| hit.distance.raw()), counts))
    }

    /// The benchmark's any-hit search of one ray, with its counts.
    fn any(&self, ray: &TraceRay) -> Option<(bool, TraversalCounts)> {
        let found = compact_any_hit_counted(self.view(), ray, FAR, |primitive| {
            self.triangles.get(primitive as usize).is_some_and(|triangle| {
                ray.hit_triangle(triangle, FixedQ32_32::ZERO, FAR).is_some()
            })
        });
        found.ok()
    }
}

/// The counted cases. Each returns the sum of the distances or hits and of the box tests, so
/// neither the answer nor the counts can be optimized away.
pub(crate) fn counted_cases() -> Vec<Case> {
    let data = Rc::new(Data::new());
    let closest = Rc::clone(&data);
    let any = data;
    vec![
        case("trace/closest_hit_compact_counted", move || {
            let sum = |(distance, counts): (Option<i64>, TraversalCounts)| {
                distance.unwrap_or(0).wrapping_add_unsigned(counts.box_tests)
            };
            let traced = closest.rays.iter().map(|ray| closest.closest(black_box(ray)));
            traced.map(|found| black_box(found).map_or(0, sum)).fold(0, i64::wrapping_add)
        }),
        case("trace/any_hit_compact_counted", move || {
            let sum = |(hit, counts): (bool, TraversalCounts)| {
                i64::from(hit).wrapping_add_unsigned(counts.box_tests)
            };
            let traced = any.rays.iter().map(|ray| any.any(black_box(ray)));
            traced.map(|found| black_box(found).map_or(0, sum)).fold(0, i64::wrapping_add)
        }),
    ]
}

#[cfg(all(test, feature = "trace"))]
mod tests {
    use super::*;
    use crate::profile::{walk, Counts, Mode, Rig};

    fn replica(c: Counts) -> [u64; 7] {
        [
            c.box_calls,
            c.box_some,
            c.skipped_at_pop,
            c.interior_visited,
            c.leaf_visited,
            c.tri_tests,
            c.stack_peak,
        ]
    }

    fn counted(c: TraversalCounts) -> [u64; 7] {
        [
            c.box_tests,
            c.box_entries,
            c.popped_skipped,
            c.interior_visits,
            c.leaf_visits,
            c.primitive_tests,
            c.peak_stack,
        ]
    }

    /// TRACE-TP-418: on every benchmark ray, both counted searches on the compact copy of the
    /// scene's tree give the replica's answer and counters (the replica is checked against
    /// `closest_hit` and `any_hit` in `profile.rs`).
    #[test]
    fn the_counted_compact_search_matches_the_replica_on_every_ray() {
        let (data, rig) = (Data::new(), Rig::new());
        let mut bad = Vec::new();
        let mut box_tests = 0;
        for (index, ray) in data.rays.iter().enumerate() {
            let (closest, any) = (walk(&rig, ray, Mode::Closest), walk(&rig, ray, Mode::Any));
            let expected = (
                (closest.hit.map(|hit| hit.1.raw()), replica(closest.counts)),
                (any.hit.is_some(), replica(any.counts)),
            );
            let found = (
                data.closest(ray).map(|(hit, counts)| (hit, counted(counts))),
                data.any(ray).map(|(hit, counts)| (hit, counted(counts))),
            );
            if found != (Some(expected.0), Some(expected.1)) {
                bad.push(index);
            }
            box_tests += found.0.map_or(0, |found| found.1[0]);
        }
        assert_eq!(bad, Vec::<usize>::new());
        // Change detector: the total of the performance chapter's profile table (19.76 per ray).
        assert_eq!(box_tests, 20_238);
    }
}
