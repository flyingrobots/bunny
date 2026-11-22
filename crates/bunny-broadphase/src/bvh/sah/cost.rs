//! Integer surface-area costs for the binned SAH builder.
//!
//! Areas are half surface areas (`dx*dy + dy*dz + dz*dx`) of box extents in raw Q32.32
//! units, as `u128`. Before multiplying, every extent of one node's candidates is shifted
//! right by the same number of bits, chosen so the node's widest extent is below
//! `2^SCALE_BITS`. Children lie inside their node, so their extents are below that bound too.
//! With extents below `2^45`, an area is below `3 * 2^90 < 2^92`; times a count below `2^32`
//! it is below `2^124`, and a split cost (traversal weight times the node area, plus the
//! intersection weight times two such terms) stays below `2^128`. Every operation is still
//! checked: an overflow returns `None`, which fails the build instead of choosing a split.

use bunny_geom::FixedAabb3;
use bunny_num::FixedQ32_32;

use crate::bvh::{SAH_INTERSECTION_COST, SAH_TRAVERSAL_COST};

/// Extents of one node's candidates are shifted below `2^SCALE_BITS` raw units.
const SCALE_BITS: u32 = 45;

/// The right shift applied to every extent while costing one node.
#[derive(Clone, Copy)]
pub(super) struct Scale(u32);

impl Scale {
    /// The shift for a node whose bounds are `bounds`: zero when its widest extent is below
    /// `2^45` raw units (8 192 units), so areas are exact there.
    pub(super) fn of(bounds: &FixedAabb3) -> Self {
        let [x, y, z] = extents(bounds);
        let widest = x.max(y).max(z);
        Self((u64::BITS - widest.leading_zeros()).saturating_sub(SCALE_BITS))
    }

    /// The half surface area of `bounds` at this scale.
    pub(super) fn area(self, bounds: &FixedAabb3) -> Option<u128> {
        let [x, y, z] = extents(bounds).map(|extent| u128::from(extent >> self.0));
        x.checked_mul(y)?.checked_add(y.checked_mul(z)?)?.checked_add(z.checked_mul(x)?)
    }

    /// The area of `bounds` times `count`.
    pub(super) fn weighted(self, bounds: &FixedAabb3, count: usize) -> Option<u128> {
        self.area(bounds)?.checked_mul(u128::try_from(count).ok()?)
    }
}

/// The cost of a split: `traversal * node_area + intersection * (left + right)`, where
/// `left` and `right` are the children's weighted areas.
pub(super) fn split_cost(node_area: u128, left: u128, right: u128) -> Option<u128> {
    let children = SAH_INTERSECTION_COST.checked_mul(left.checked_add(right)?)?;
    SAH_TRAVERSAL_COST.checked_mul(node_area)?.checked_add(children)
}

/// The cost of a leaf: `intersection * node_area * count`. Comparable with [`split_cost`]
/// because both are scaled by the node's area.
pub(super) const fn leaf_cost(node_weighted: u128) -> Option<u128> {
    SAH_INTERSECTION_COST.checked_mul(node_weighted)
}

fn extents(bounds: &FixedAabb3) -> [u64; 3] {
    [
        extent(bounds.min.x, bounds.max.x),
        extent(bounds.min.y, bounds.max.y),
        extent(bounds.min.z, bounds.max.z),
    ]
}

/// `max - min` in raw units, or zero for an empty or inverted extent.
fn extent(min: FixedQ32_32, max: FixedQ32_32) -> u64 {
    if max > min {
        max.raw().abs_diff(min.raw())
    } else {
        0
    }
}
