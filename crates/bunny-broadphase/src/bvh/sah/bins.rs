//! Centroid bins and the plane sweep of the binned SAH builder.
//!
//! A centroid is kept doubled (`min + max` per axis, as `i128`) so no rounding is needed.
//! On an axis whose doubled centroids span `origin..=origin + span` with `span > 0`, a
//! primitive with doubled centroid `c` falls in bin `(c - origin) * BINS / (span + 1)`,
//! which is always below `BINS`. Plane `p` (1 to `BINS - 1`) sends bins below `p` left.

use bunny_geom::FixedAabb3;

use super::cost::{split_cost, Scale};
use crate::bvh::SAH_BIN_COUNT;
use crate::utils::{get_axis_val, union_aabb};

const AXES: [usize; 3] = [0, 1, 2];

/// The doubled centroids of a node along one axis.
#[derive(Clone, Copy)]
pub(super) struct Span {
    origin: i128,
    end: i128,
}

impl Span {
    const fn at(key: i128) -> Self {
        Self { origin: key, end: key }
    }

    fn include(&mut self, key: i128) {
        self.origin = self.origin.min(key);
        self.end = self.end.max(key);
    }

    /// The width of the span, or `None` when every centroid is equal (no plane exists).
    fn width(self) -> Option<u128> {
        let width = u128::try_from(self.end.checked_sub(self.origin)?).ok()?;
        (width > 0).then_some(width)
    }

    fn bin(self, key: i128) -> Option<usize> {
        let offset = u128::try_from(key.checked_sub(self.origin)?).ok()?;
        let bins = u128::try_from(SAH_BIN_COUNT).ok()?;
        let bin = offset.checked_mul(bins)?.checked_div(self.width()?.checked_add(1)?)?;
        usize::try_from(bin).ok()
    }
}

/// The bounds of a node's primitives and the span of their centroids on each axis.
pub(super) struct Summary {
    pub(super) bounds: FixedAabb3,
    spans: [Span; 3],
}

impl Summary {
    pub(super) fn of(slots: &[u32], primitives: &[FixedAabb3]) -> Option<Self> {
        let (first, rest) = slots.split_first()?;
        let bounds = primitive(primitives, *first)?;
        let mut summary = Self { bounds, spans: AXES.map(|axis| Span::at(key(&bounds, axis))) };
        for &slot in rest {
            summary.include(primitive(primitives, slot)?);
        }
        Some(summary)
    }

    fn include(&mut self, bounds: FixedAabb3) {
        self.bounds = union_aabb(self.bounds, bounds);
        for (axis, span) in self.spans.iter_mut().enumerate() {
            span.include(key(&bounds, axis));
        }
    }
}

/// A failure the bounds rule out: an arithmetic overflow or an index out of range. The
/// builder turns it into a failed build rather than a choice.
pub(super) struct Unreachable;

type Checked<T> = Result<T, Unreachable>;

/// A chosen split plane: the primitives whose bin on `axis` is below `index` go left.
#[derive(Clone, Copy)]
pub(super) struct Plane {
    span: Span,
    axis: usize,
    index: usize,
    pub(super) cost: u128,
    pub(super) left_count: usize,
}

impl Plane {
    pub(super) fn sends_left(&self, bounds: &FixedAabb3) -> Option<bool> {
        Some(self.span.bin(key(bounds, self.axis))? < self.index)
    }
}

#[derive(Clone, Copy)]
struct Bin {
    bounds: Option<FixedAabb3>,
    count: usize,
}

impl Bin {
    const EMPTY: Self = Self { bounds: None, count: 0 };

    fn add(&mut self, bounds: Option<FixedAabb3>, count: usize) {
        self.bounds = match (self.bounds, bounds) {
            (Some(own), Some(other)) => Some(union_aabb(own, other)),
            (own, other) => own.or(other),
        };
        self.count += count;
    }

    fn weighted(self, scale: Scale) -> Checked<Option<u128>> {
        self.bounds.map(|bounds| scale.weighted(&bounds, self.count).ok_or(Unreachable)).transpose()
    }
}

type AxisBins = [Bin; SAH_BIN_COUNT];

/// The cheapest plane of the node: lowest cost, then lowest axis (x, y, z), then lowest
/// plane. `Ok(None)` means no axis has two distinct centroids.
pub(super) fn best_plane(
    summary: &Summary,
    slots: &[u32],
    primitives: &[FixedAabb3],
) -> Checked<Option<Plane>> {
    let bins = fill(summary, slots, primitives).ok_or(Unreachable)?;
    let scale = Scale::of(&summary.bounds);
    let node_area = scale.area(&summary.bounds).ok_or(Unreachable)?;
    let mut best: Option<Plane> = None;
    for (axis, (span, axis_bins)) in summary.spans.iter().zip(bins.iter()).enumerate() {
        let sweep = Sweep { span: *span, axis, scale, node_area };
        if let Some(found) = sweep.run(axis_bins)? {
            best = Some(best.filter(|kept| kept.cost <= found.cost).unwrap_or(found));
        }
    }
    Ok(best)
}

fn fill(summary: &Summary, slots: &[u32], primitives: &[FixedAabb3]) -> Option<[AxisBins; 3]> {
    let mut bins = [[Bin::EMPTY; SAH_BIN_COUNT]; 3];
    for &slot in slots {
        let bounds = primitive(primitives, slot)?;
        for (axis, (span, axis_bins)) in summary.spans.iter().zip(bins.iter_mut()).enumerate() {
            add_to_bin(axis_bins, *span, axis, bounds)?;
        }
    }
    Some(bins)
}

fn add_to_bin(bins: &mut AxisBins, span: Span, axis: usize, bounds: FixedAabb3) -> Option<()> {
    if span.width().is_none() {
        return Some(());
    }
    bins.get_mut(span.bin(key(&bounds, axis))?)?.add(Some(bounds), 1);
    Some(())
}

struct Sweep {
    span: Span,
    axis: usize,
    scale: Scale,
    node_area: u128,
}

impl Sweep {
    /// The cheapest plane on this axis; among equal costs, the lowest plane.
    fn run(&self, bins: &AxisBins) -> Checked<Option<Plane>> {
        if self.span.width().is_none() {
            return Ok(None);
        }
        let right = right_weights(bins, self.scale)?;
        let mut left = Bin::EMPTY;
        let mut best: Option<Plane> = None;
        for (index, (bin, right)) in bins.iter().zip(right.iter().skip(1)).enumerate() {
            left.add(bin.bounds, bin.count);
            if let (Some(left_weight), Some(right_weight)) = (left.weighted(self.scale)?, right) {
                let cost =
                    split_cost(self.node_area, left_weight, *right_weight).ok_or(Unreachable)?;
                let plane = self.plane(index + 1, cost, left.count);
                best = Some(best.filter(|kept| kept.cost <= cost).unwrap_or(plane));
            }
        }
        Ok(best)
    }

    const fn plane(&self, index: usize, cost: u128, left_count: usize) -> Plane {
        Plane { span: self.span, axis: self.axis, index, cost, left_count }
    }
}

/// Entry `i` is the weighted area of bins `i..BINS`, or `None` when they are all empty.
fn right_weights(bins: &AxisBins, scale: Scale) -> Checked<[Option<u128>; SAH_BIN_COUNT]> {
    let mut weights = [None; SAH_BIN_COUNT];
    let mut right = Bin::EMPTY;
    for (bin, weight) in bins.iter().zip(weights.iter_mut()).rev() {
        right.add(bin.bounds, bin.count);
        *weight = right.weighted(scale)?;
    }
    Ok(weights)
}

/// The doubled centroid of `bounds` on `axis`.
fn key(bounds: &FixedAabb3, axis: usize) -> i128 {
    i128::from(get_axis_val(&bounds.min, axis).raw())
        + i128::from(get_axis_val(&bounds.max, axis).raw())
}

pub(super) fn primitive(primitives: &[FixedAabb3], slot: u32) -> Option<FixedAabb3> {
    primitives.get(usize::try_from(slot).ok()?).copied()
}
