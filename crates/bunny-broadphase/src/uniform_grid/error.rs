//! The refusals of the uniform grid broadphase.

use core::fmt;

/// Why a cell size or a grid input was refused.
///
/// A refusal writes nothing: neither the key scratch nor the pair buffer is touched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UniformGridError {
    /// The cell size is zero or negative: it must be at least one raw unit.
    NonPositiveCellSize {
        /// The refused raw value.
        raw: i64,
    },
    /// More than `u32::MAX` boxes, so a box index would not fit a pair.
    TooManyBoxes {
        /// The number of boxes given.
        count: usize,
    },
    /// The box at `index` has a minimum above its maximum on some axis.
    InvertedBox {
        /// The lowest index of an inverted box.
        index: usize,
    },
    /// The number of (cell, object) keys passes `usize` at the box at `index`.
    KeyCountOverflow {
        /// The box whose keys made the running total overflow.
        index: usize,
    },
    /// The key scratch is shorter than `grid_key_count` for the input.
    KeyScratchTooSmall {
        /// The keys needed.
        required: usize,
        /// The keys given.
        provided: usize,
    },
}

impl fmt::Display for UniformGridError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::NonPositiveCellSize { raw } => {
                write!(f, "cell size {raw} raw is not positive")
            }
            Self::TooManyBoxes { count } => write!(f, "{count} boxes is more than u32::MAX"),
            Self::InvertedBox { index } => {
                write!(f, "box {index} has a minimum above its maximum")
            }
            Self::KeyCountOverflow { index } => {
                write!(f, "the grid key count overflows usize at box {index}")
            }
            Self::KeyScratchTooSmall { required, provided } => {
                write!(f, "key scratch has {provided} keys; {required} are required")
            }
        }
    }
}

impl core::error::Error for UniformGridError {}
