//! `BVH-TP-031`: the buffer checks on lengths no test can allocate.

use super::{check_buffers, BvhBuildError};
use crate::bvh::build::validated_lengths;

/// `BVH-TP-031`: the length checks agree with the builders' own on lengths no test can
/// allocate, on and one either side of `2^31` primitives and `u32::MAX` nodes.
#[test]
fn bvh_tp_031_too_many_primitives_is_named_and_agrees_with_the_builders() {
    let limit: usize = 1 << 31;
    let roomy = usize::MAX;
    for n in [limit - 1, limit] {
        assert_eq!(check_buffers(2 * n - 1, n, n), Ok(()), "{n} primitives fit");
        assert!(validated_lengths(2 * n - 1, n, n).is_some());
    }
    for n in [limit + 1, limit + 2] {
        let many = Err(BvhBuildError::TooManyPrimitives { primitives: n });
        assert_eq!(check_buffers(roomy, roomy, n), many, "{n} primitives");
        assert!(validated_lengths(roomy, roomy, n).is_none(), "{n} primitives");
    }
    // No node buffer holds `2 * usize::MAX - 1` nodes, so the node check fires first.
    let n = usize::MAX;
    let nodes = Err(BvhBuildError::NodeBufferTooSmall { nodes: roomy, primitives: n });
    assert_eq!(check_buffers(roomy, roomy, n), nodes);
    assert!(validated_lengths(roomy, roomy, n).is_none());
    // The node buffer is checked before the count.
    let short = Err(BvhBuildError::NodeBufferTooSmall { nodes: 3, primitives: limit + 1 });
    assert_eq!(check_buffers(3, roomy, limit + 1), short);
    // Every small length triple agrees with the builders' check.
    for n in 0..12 {
        for nodes in 0..26 {
            for indices in 0..14 {
                let (ours, theirs) =
                    (check_buffers(nodes, indices, n), validated_lengths(nodes, indices, n));
                assert_eq!(ours.is_ok(), theirs.is_some(), "{nodes} {indices} {n}");
            }
        }
    }
}
