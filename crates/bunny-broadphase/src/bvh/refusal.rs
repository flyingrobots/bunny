//! Why [`super::try_build_bvh_sah`] refused its buffers (flyingrobots/bunny#445).

use core::fmt;

/// The most primitives a tree can index: `2n - 1` nodes must fit `u32` indices, so
/// `n <= 2^31`.
const MAX_PRIMITIVES: u128 = 1 << 31;

/// Why [`super::try_build_bvh_sah`] refused to build. Each variant carries the lengths the
/// check saw.
///
/// [`super::build_bvh_sah`] returns `None` exactly when `try_build_bvh_sah` returns an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BvhBuildError {
    /// The node buffer holds fewer than `2 * primitives - 1` nodes.
    NodeBufferTooSmall {
        /// The node buffer's length.
        nodes: usize,
        /// The number of primitives.
        primitives: usize,
    },
    /// The primitive-index buffer holds fewer entries than there are primitives.
    IndexBufferTooSmall {
        /// The primitive-index buffer's length.
        indices: usize,
        /// The number of primitives.
        primitives: usize,
    },
    /// More than `2^31` primitives: the `2n - 1` node indices would not fit `u32`. No slice
    /// of `FixedAabb3` on a supported target is this long together with a node buffer that
    /// passes the first check, so this is defensive.
    TooManyPrimitives {
        /// The number of primitives.
        primitives: usize,
    },
    /// The buffers passed every check above and a step of the build still failed one of its
    /// own checks (an overflow or an index out of range). This is a defensive refusal: the
    /// build's bounds rule these out and no input is known to reach it. The buffers may then
    /// be partly written.
    InternalCheckFailed,
}

/// The buffer checks of the builders, in the order they make them, from the three lengths.
///
/// The first failing check is reported: the node buffer, then the index buffer, then the
/// primitive count. Computed in `u128`, so no length overflows on any target.
pub(super) fn check_buffers(
    nodes: usize,
    indices: usize,
    primitives: usize,
) -> Result<(), BvhBuildError> {
    let wide = |len: usize| u128::try_from(len).unwrap_or(u128::MAX);
    if primitives == 0 {
        return Ok(());
    }
    let required = 2 * wide(primitives) - 1;
    if wide(nodes) < required {
        Err(BvhBuildError::NodeBufferTooSmall { nodes, primitives })
    } else if indices < primitives {
        Err(BvhBuildError::IndexBufferTooSmall { indices, primitives })
    } else if wide(primitives) > MAX_PRIMITIVES {
        Err(BvhBuildError::TooManyPrimitives { primitives })
    } else {
        Ok(())
    }
}

impl fmt::Display for BvhBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NodeBufferTooSmall { nodes, primitives } => {
                write!(f, "node buffer of {nodes} is below 2 * {primitives} - 1 nodes")
            }
            Self::IndexBufferTooSmall { indices, primitives } => {
                write!(f, "primitive-index buffer of {indices} is below {primitives} entries")
            }
            Self::TooManyPrimitives { primitives } => {
                write!(f, "{primitives} primitives exceed the {MAX_PRIMITIVES} a u32 tree indexes")
            }
            Self::InternalCheckFailed => f.write_str("an internal check of the build failed"),
        }
    }
}

impl core::error::Error for BvhBuildError {}

#[cfg(test)]
mod tests;
