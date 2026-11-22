//! The work one compact search did, as returned by [`crate::compact_closest_hit_counted`] and
//! [`crate::compact_any_hit_counted`].

use core::ops::{Add, AddAssign};

/// Event counts of one compact search, or a sum over several.
///
/// Each counter is incremented at one place of the search loop shared with
/// [`crate::compact_closest_hit`] and [`crate::compact_any_hit`]; see
/// `docs/topics/ray-tracing/README.md#counting-traversals`. A search pops every entry it
/// pushes unless it stops early, so for a closest-hit search `box_entries` equals
/// `popped_skipped + interior_visits + leaf_visits`, and `box_tests` is
/// `2 * interior_visits` plus one for the root's box.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct TraversalCounts {
    /// Calls of `TraceRay::box_entry`: the root's box, and both boxes of every interior visit.
    pub box_tests: u64,
    /// Box tests that returned a bound, so the child was pushed.
    pub box_entries: u64,
    /// Popped entries dropped because their bound exceeds the limit, which shrank since the push.
    pub popped_skipped: u64,
    /// Interior children searched (both child boxes tested).
    pub interior_visits: u64,
    /// Leaves searched.
    pub leaf_visits: u64,
    /// Calls of the caller's primitive test.
    pub primitive_tests: u64,
    /// The most entries the stack held at once (at most 64).
    pub peak_stack: u64,
}

impl Add for TraversalCounts {
    type Output = Self;

    /// Sums every counter, saturating at `u64::MAX`, and keeps the larger `peak_stack`.
    fn add(self, other: Self) -> Self {
        Self {
            box_tests: self.box_tests.saturating_add(other.box_tests),
            box_entries: self.box_entries.saturating_add(other.box_entries),
            popped_skipped: self.popped_skipped.saturating_add(other.popped_skipped),
            interior_visits: self.interior_visits.saturating_add(other.interior_visits),
            leaf_visits: self.leaf_visits.saturating_add(other.leaf_visits),
            primitive_tests: self.primitive_tests.saturating_add(other.primitive_tests),
            peak_stack: self.peak_stack.max(other.peak_stack),
        }
    }
}

impl AddAssign for TraversalCounts {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

/// Where the search reports its events: [`NoTally`] for the uncounted functions, whose calls
/// compile to nothing, and [`TraversalCounts`] for the counted ones.
pub(crate) trait Tally: Default {
    fn box_test(&mut self, entered: bool);
    fn skipped(&mut self);
    fn interior(&mut self);
    fn leaf(&mut self);
    fn primitive(&mut self);
    fn depth(&mut self, len: usize);
}

/// The tally of the uncounted functions: zero-sized, it records nothing.
#[derive(Default)]
pub(crate) struct NoTally;

impl Tally for NoTally {
    #[inline]
    fn box_test(&mut self, _: bool) {}
    #[inline]
    fn skipped(&mut self) {}
    #[inline]
    fn interior(&mut self) {}
    #[inline]
    fn leaf(&mut self) {}
    #[inline]
    fn primitive(&mut self) {}
    #[inline]
    fn depth(&mut self, _: usize) {}
}

/// Each count is bounded far below `u64::MAX` (see the overflow argument in the topic
/// chapter); the increments saturate so the rule holds even without that argument.
impl Tally for TraversalCounts {
    fn box_test(&mut self, entered: bool) {
        self.box_tests = self.box_tests.saturating_add(1);
        self.box_entries = self.box_entries.saturating_add(u64::from(entered));
    }

    fn skipped(&mut self) {
        self.popped_skipped = self.popped_skipped.saturating_add(1);
    }

    fn interior(&mut self) {
        self.interior_visits = self.interior_visits.saturating_add(1);
    }

    fn leaf(&mut self) {
        self.leaf_visits = self.leaf_visits.saturating_add(1);
    }

    fn primitive(&mut self) {
        self.primitive_tests = self.primitive_tests.saturating_add(1);
    }

    fn depth(&mut self, len: usize) {
        self.peak_stack = self.peak_stack.max(u64::try_from(len).unwrap_or(u64::MAX));
    }
}
