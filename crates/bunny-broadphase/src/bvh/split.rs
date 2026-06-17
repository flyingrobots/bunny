use bunny_geom::FixedAabb3;
use bunny_num::FixedQ32_32;

use super::build::PrimRange;
use crate::utils::{get_axis_val, get_centroid, surface_area, union_aabb};

#[derive(Clone, Copy)]
pub(super) struct SplitChoice {
    axis: usize,
    coord: FixedQ32_32,
    cost: FixedQ32_32,
}

pub(super) enum SplitSearch {
    Empty,
    Found(SplitChoice),
}

impl SplitChoice {
    pub(super) fn sends_left(self, primitive: &FixedAabb3) -> bool {
        get_axis_val(&get_centroid(primitive), self.axis) < self.coord
    }
}

#[derive(Clone, Copy)]
struct AxisSpan {
    min: FixedQ32_32,
    max: FixedQ32_32,
}

#[derive(Clone, Copy)]
struct SteppedAxis {
    axis: usize,
    min: FixedQ32_32,
    step: FixedQ32_32,
}

impl AxisSpan {
    fn step(self) -> Option<FixedQ32_32> {
        let step = (self.max - self.min).checked_div(FixedQ32_32::from_f32(4.0))?;
        (step != FixedQ32_32::ZERO).then_some(step)
    }
}

struct SplitBuckets {
    left: Option<FixedAabb3>,
    right: Option<FixedAabb3>,
    left_count: usize,
    right_count: usize,
}

impl SplitBuckets {
    const fn new() -> Self {
        Self { left: None, right: None, left_count: 0, right_count: 0 }
    }

    fn push(&mut self, primitive: FixedAabb3, split: SplitChoice) {
        if split.sends_left(&primitive) {
            self.add_left(primitive);
        } else {
            self.add_right(primitive);
        }
    }

    fn add_left(&mut self, primitive: FixedAabb3) {
        self.left = Some(self.left.map_or(primitive, |bounds| union_aabb(bounds, primitive)));
        self.left_count += 1;
    }

    fn add_right(&mut self, primitive: FixedAabb3) {
        self.right = Some(self.right.map_or(primitive, |bounds| union_aabb(bounds, primitive)));
        self.right_count += 1;
    }

    fn choice(self, axis: usize, coord: FixedQ32_32) -> Option<SplitSearch> {
        let (Some(left), Some(right)) = (self.left, self.right) else {
            return Some(SplitSearch::Empty);
        };
        let cost = bucket_cost(left, self.left_count, right, self.right_count)?;
        Some(SplitSearch::Found(SplitChoice { axis, coord, cost }))
    }
}

pub(super) struct SplitEvaluator<'a> {
    prim_indices: &'a [u32],
    primitives: &'a [FixedAabb3],
}

impl<'a> SplitEvaluator<'a> {
    pub(super) const fn new(prim_indices: &'a [u32], primitives: &'a [FixedAabb3]) -> Self {
        Self { prim_indices, primitives }
    }

    pub(super) fn accepted_split(
        &self,
        range: PrimRange,
        bounds: FixedAabb3,
    ) -> Option<SplitSearch> {
        match self.best_split(range)? {
            SplitSearch::Found(split) if keep_split(range, &bounds, split)? => {
                Some(SplitSearch::Found(split))
            }
            SplitSearch::Empty | SplitSearch::Found(_) => Some(SplitSearch::Empty),
        }
    }

    fn best_split(&self, range: PrimRange) -> Option<SplitSearch> {
        let mut best = SplitSearch::Empty;
        for axis in 0..3 {
            best = better_split(best, self.axis_split(range, axis)?);
        }
        Some(best)
    }

    fn axis_split(&self, range: PrimRange, axis: usize) -> Option<SplitSearch> {
        let span = self.axis_span(range, axis)?;
        let Some(step) = span.step() else {
            return Some(SplitSearch::Empty);
        };
        self.best_stepped_split(range, SteppedAxis { axis, min: span.min, step })
    }

    fn best_stepped_split(&self, range: PrimRange, stepped: SteppedAxis) -> Option<SplitSearch> {
        let mut best = SplitSearch::Empty;
        for units in 1_i64..4 {
            let coord = stepped.min + stepped.step * fixed_units(units)?;
            best = better_split(best, self.split_cost(range, stepped.axis, coord)?);
        }
        Some(best)
    }

    fn axis_span(&self, range: PrimRange, axis: usize) -> Option<AxisSpan> {
        let first = self.primitive_at(range.first)?;
        let mut min = get_axis_val(&get_centroid(&first), axis);
        let mut max = min;
        for offset in 1..range.count {
            let primitive = self.primitive_at(range.slot(offset)?)?;
            let centroid = get_axis_val(&get_centroid(&primitive), axis);
            min = std::cmp::min(min, centroid);
            max = std::cmp::max(max, centroid);
        }
        Some(AxisSpan { min, max })
    }

    fn split_cost(&self, range: PrimRange, axis: usize, coord: FixedQ32_32) -> Option<SplitSearch> {
        let split = SplitChoice { axis, coord, cost: FixedQ32_32::ZERO };
        let mut buckets = SplitBuckets::new();
        for offset in 0..range.count {
            buckets.push(self.primitive_at(range.slot(offset)?)?, split);
        }
        buckets.choice(axis, coord)
    }

    fn primitive_at(&self, slot: usize) -> Option<FixedAabb3> {
        let primitive_idx = usize::try_from(*self.prim_indices.get(slot)?).ok()?;
        self.primitives.get(primitive_idx).copied()
    }
}

fn fixed_units(units: i64) -> Option<FixedQ32_32> {
    let raw = units.checked_mul(FixedQ32_32::ONE.to_raw())?;
    Some(FixedQ32_32::from_raw(raw))
}

fn bucket_cost(
    left: FixedAabb3,
    left_count: usize,
    right: FixedAabb3,
    right_count: usize,
) -> Option<FixedQ32_32> {
    let left_cost = surface_area(&left) * fixed_count(left_count)?;
    let right_cost = surface_area(&right) * fixed_count(right_count)?;
    Some(left_cost + right_cost)
}

fn fixed_count(count: usize) -> Option<FixedQ32_32> {
    fixed_units(i64::try_from(count).ok()?)
}

fn keep_split(range: PrimRange, bounds: &FixedAabb3, split: SplitChoice) -> Option<bool> {
    if range.count > 4 {
        return Some(true);
    }
    Some(split.cost < leaf_cost(bounds, range.count)?)
}

fn leaf_cost(bounds: &FixedAabb3, count: usize) -> Option<FixedQ32_32> {
    Some(surface_area(bounds) * fixed_count(count)?)
}

fn better_split(best: SplitSearch, next: SplitSearch) -> SplitSearch {
    match (best, next) {
        (SplitSearch::Found(current), SplitSearch::Found(candidate))
            if candidate.cost < current.cost =>
        {
            SplitSearch::Found(candidate)
        }
        (SplitSearch::Found(current), _) => SplitSearch::Found(current),
        (SplitSearch::Empty, candidate) => candidate,
    }
}
