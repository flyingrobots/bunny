//! The stable partition of the binned SAH builder.

use bunny_geom::FixedAabb3;

use super::bins::{primitive, Plane};

/// Reorders `slots` so the primitives `plane` sends left come first, and returns how many
/// went left.
///
/// The result is stable: the builder keeps every node's slots in ascending primitive order
/// (the root starts as `0..n`), and both sides are left in ascending order again. The swap
/// pass is followed by a sort of each side; slot values are distinct, so the sorted order
/// is the only ascending one and does not depend on the sort's algorithm. `sort_unstable`
/// sorts in place and does not allocate.
pub(super) fn partition(
    slots: &mut [u32],
    primitives: &[FixedAabb3],
    plane: &Plane,
) -> Option<usize> {
    let mut left = 0_usize;
    for index in 0..slots.len() {
        if plane.sends_left(&primitive(primitives, *slots.get(index)?)?)? {
            slots.swap(left, index);
            left += 1;
        }
    }
    let (left_slots, right_slots) = slots.split_at_mut_checked(left)?;
    left_slots.sort_unstable();
    right_slots.sort_unstable();
    Some(left)
}
