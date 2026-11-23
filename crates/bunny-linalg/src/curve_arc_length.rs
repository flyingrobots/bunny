//! Arc-length tables and constant-speed parameter lookup over a [`Curve3`] (`bunny-curve/1`).
//!
//! The rules and their bounds are stated in `docs/topics/curve/README.md#arc-length`.

use bunny_num::FixedQ32_32;

use crate::{Curve3, FixedVec3};

/// The largest `segments_per_span` that [`ArcLengthTable::build`] accepts.
pub const MAX_SEGMENTS_PER_SPAN: u32 = 4096;

/// Fractional bits below the raw unit that each chord length keeps before the sum is rounded.
const CHORD_EXTRA_BITS: u32 = 16;

/// Why [`ArcLengthTable::build`] refused a curve.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArcLengthError {
    /// `segments_per_span` is outside `1..=4096`.
    SegmentsOutOfRange,
    /// The curve's `span_count()` is outside `1..=2^31 - 1`.
    SpanCountOutOfRange,
    /// `span_count * segments_per_span` is above `u32::MAX`.
    TooManySamples,
    /// The storage slice is shorter than `required`, which is the sample count plus one.
    StorageTooSmall {
        /// The number of entries the table needs.
        required: usize,
    },
    /// The curve returned `None` at sample `sample`.
    CurveRefused {
        /// The index of the refused sample.
        sample: u32,
    },
    /// A component of chord `chord` (from sample `chord` to `chord + 1`) is outside Q32.32.
    ChordOverflow {
        /// The index of the chord.
        chord: u32,
    },
    /// Cumulative entry `entry` rounds above the Q32.32 range.
    LengthOverflow {
        /// The index of the entry.
        entry: u32,
    },
    /// Every chord has length zero after rounding: the total is zero.
    ZeroLength,
}

/// Cumulative chord lengths of a curve sampled at equal parameter steps, in caller storage.
#[derive(Debug, PartialEq, Eq)]
pub struct ArcLengthTable<'a> {
    lengths: &'a [FixedQ32_32],
    segments_per_span: u32,
    end: FixedQ32_32,
}

/// Samples per span and spans, both in range, multiplied without overflow.
const fn sample_count(span_count: u32, segments_per_span: u32) -> Result<u32, ArcLengthError> {
    if segments_per_span == 0 || segments_per_span > MAX_SEGMENTS_PER_SPAN {
        return Err(ArcLengthError::SegmentsOutOfRange);
    }
    if span_count == 0 || span_count > i32::MAX as u32 {
        return Err(ArcLengthError::SpanCountOutOfRange);
    }
    match span_count.checked_mul(segments_per_span) {
        Some(samples) => Ok(samples),
        None => Err(ArcLengthError::TooManySamples),
    }
}

/// The parameter of sample `k`: `round_half_even(k * 2^32 / segments_per_span)`.
fn parameter(k: u32, segments_per_span: u32) -> Result<FixedQ32_32, ArcLengthError> {
    FixedQ32_32::from_ratio(k, segments_per_span).ok_or(ArcLengthError::TooManySamples)
}

fn sample<C: Curve3 + ?Sized>(
    curve: &C,
    k: u32,
    per_span: u32,
) -> Result<FixedVec3, ArcLengthError> {
    curve.at(parameter(k, per_span)?).ok_or(ArcLengthError::CurveRefused { sample: k })
}

/// `floor(2^16 * |to - from|)` from the exact squared length, or `None` when a component of
/// `to - from` is outside `i64`.
fn scaled_chord(from: FixedVec3, to: FixedVec3) -> Option<u128> {
    let square = |a: FixedQ32_32, b: FixedQ32_32| {
        b.raw().checked_sub(a.raw()).map(|d| u128::from(d.unsigned_abs()).pow(2))
    };
    // Each square is at most 2^126, so the sum is below 3 * 2^126.
    let sum = square(from.x, to.x)? + square(from.y, to.y)? + square(from.z, to.z)?;
    let root = FixedQ32_32::sqrt_u128(sum);
    // remainder <= 2 root < 2^65, so the scaled remainder is below 2^97.
    let scaled_remainder = (sum - root * root) << (2 * CHORD_EXTRA_BITS);
    // (root * 2^16 + c)^2 <= sum * 2^32 exactly when this holds; each side is below 2^98.
    let fits = |c: u128| ((root * c) << (CHORD_EXTRA_BITS + 1)) + c * c <= scaled_remainder;
    // The condition is monotone in c, so the largest c below 2^16 is set bit by bit.
    let low = (0..CHORD_EXTRA_BITS).rev().fold(0_u128, |low, bit| {
        let candidate = low | 1 << bit;
        if fits(candidate) {
            candidate
        } else {
            low
        }
    });
    Some((root << CHORD_EXTRA_BITS) + low)
}

/// `round_half_even(sum / 2^16)`, or `None` above `i64::MAX`.
fn rounded_entry(sum: u128) -> Option<FixedQ32_32> {
    let (floor, rest) = (sum >> CHORD_EXTRA_BITS, sum & ((1 << CHORD_EXTRA_BITS) - 1));
    let half = 1 << (CHORD_EXTRA_BITS - 1);
    let rounded = floor + u128::from(rest > half || (rest == half && floor & 1 == 1));
    i64::try_from(rounded).ok().map(FixedQ32_32::from_raw)
}

/// `round_half_even(numerator / denominator)` for `denominator > 0`.
fn divide_half_even(numerator: u128, denominator: u128) -> u128 {
    let (quotient, rest) = (numerator / denominator, numerator % denominator);
    let complement = denominator - rest;
    quotient + u128::from(rest > complement || (rest == complement && quotient & 1 == 1))
}

fn fill<C: Curve3 + ?Sized>(
    curve: &C,
    per_span: u32,
    samples: u32,
    lengths: &mut [FixedQ32_32],
) -> Result<(), ArcLengthError> {
    let mut previous = sample(curve, 0, per_span)?;
    let mut sum = 0_u128;
    for (k, entry) in (0..=samples).zip(lengths.iter_mut()) {
        if k > 0 {
            let point = sample(curve, k, per_span)?;
            let chord = ArcLengthError::ChordOverflow { chord: k - 1 };
            // sum is at most (i64::MAX + 1/2) * 2^16 before the add, so it stays below 2^81.
            sum += scaled_chord(previous, point).ok_or(chord)?;
            previous = point;
        }
        *entry = rounded_entry(sum).ok_or(ArcLengthError::LengthOverflow { entry: k })?;
    }
    Ok(())
}

impl<'a> ArcLengthTable<'a> {
    /// Samples `curve` at `span_count * segments_per_span + 1` equal parameter steps and
    /// stores the cumulative chord lengths in the front of `storage` (`bunny-curve/1`).
    ///
    /// Sample `k` is at `u_k = round_half_even(k * 2^32 / segments_per_span)` raw, so
    /// `u_0 = 0` and the last is exactly `span_count`. Each chord between consecutive samples
    /// is measured from its exact squared length to `2^-16` raw, rounding down, the chords are
    /// summed exactly, and each entry is that sum rounded once to the nearest raw unit, ties
    /// to even. So entry `k` is within `1/2 + k / 2^16` raw of the exact length of the
    /// polyline through the sampled points, and the entries never decrease. The chapter
    /// states what that gives against the curve's true arc length.
    ///
    /// # Errors
    ///
    /// The [`ArcLengthError`] variants, checked in their declared order; the curve is
    /// sampled only once the counts and storage pass, and refuses at the first failing
    /// sample, chord or entry. A total of zero is [`ArcLengthError::ZeroLength`].
    pub fn build<C: Curve3 + ?Sized>(
        curve: &C,
        segments_per_span: u32,
        storage: &'a mut [FixedQ32_32],
    ) -> Result<Self, ArcLengthError> {
        let span_count = curve.span_count();
        let samples = sample_count(span_count, segments_per_span)?;
        let required = usize::try_from(samples).ok().and_then(|n| n.checked_add(1));
        let required = required.ok_or(ArcLengthError::TooManySamples)?;
        let lengths =
            storage.get_mut(..required).ok_or(ArcLengthError::StorageTooSmall { required })?;
        fill(curve, segments_per_span, samples, lengths)?;
        if lengths.last() == Some(&FixedQ32_32::ZERO) {
            return Err(ArcLengthError::ZeroLength);
        }
        let end = FixedQ32_32::from_raw(i64::from(span_count) << 32);
        Ok(Self { lengths, segments_per_span, end })
    }

    /// The cumulative lengths: entry `k` is the length up to sample `k`, entry 0 is zero.
    #[must_use]
    pub const fn lengths(&self) -> &[FixedQ32_32] {
        self.lengths
    }

    /// The table's total length, its last entry.
    #[must_use]
    pub fn total(&self) -> FixedQ32_32 {
        self.lengths.last().copied().unwrap_or(FixedQ32_32::ZERO)
    }

    /// The parameter at the end of the curve, exactly `span_count`.
    #[must_use]
    pub const fn end(&self) -> FixedQ32_32 {
        self.end
    }

    /// The parameter at distance `s` along the table (`bunny-curve/1`).
    ///
    /// `s = total()` returns exactly [`Self::end`]. Otherwise the bracketing entry is the
    /// first `j` with `lengths[j] >= s`, found by binary search: when `lengths[j] == s` the
    /// result is exactly `u_j` (so `s = 0` gives 0, and a distance on a run of equal entries,
    /// where the curve did not move, gives the first sample of the run); otherwise the result
    /// is `u_{j-1} + round_half_even((s - L_{j-1}) (u_j - u_{j-1}) / (L_j - L_{j-1}))`,
    /// computed exactly in `u128` and rounded once. The result never decreases as `s` grows.
    ///
    /// Returns `None` when `s` is below zero or above `total()`.
    #[must_use]
    pub fn param_at_distance(&self, s: FixedQ32_32) -> Option<FixedQ32_32> {
        if s.raw() < 0 || s > self.total() {
            return None;
        }
        if s == self.total() {
            return Some(self.end);
        }
        let j = self.lengths.partition_point(|length| *length < s);
        let (upper_length, upper) = (*self.lengths.get(j)?, self.param(j)?);
        if upper_length == s {
            return Some(upper);
        }
        // s > lengths[0] = 0, so j >= 1, and lengths[j - 1] < s < lengths[j].
        let (lower_length, lower) = (*self.lengths.get(j.checked_sub(1)?)?, self.param(j - 1)?);
        let offset = u128::from(s.raw().abs_diff(lower_length.raw()));
        let span = u128::from(upper_length.raw().abs_diff(lower_length.raw()));
        let step = u128::from(upper.raw().abs_diff(lower.raw()));
        // offset < 2^63 and step <= 2^32, so the product is exact; the quotient is <= step.
        let rounded = divide_half_even(offset * step, span);
        lower.checked_add(FixedQ32_32::from_raw(i64::try_from(rounded).ok()?))
    }

    fn param(&self, index: usize) -> Option<FixedQ32_32> {
        parameter(u32::try_from(index).ok()?, self.segments_per_span).ok()
    }
}
