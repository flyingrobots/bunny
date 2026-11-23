//! `TraversalError` and `CompactError` implement `Display` and `core::error::Error`
//! (flyingrobots/bunny#445). Each message states the condition its variant documents.

use core::error::Error;

use bunny_broadphase::{CompactError, TraversalError};
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_728_traversal_errors_display_their_condition_and_are_errors() {
    let cases = [
        (TraversalError::StackOverflow, "traversal needed more than its 64 stack entries"),
        (
            TraversalError::InvalidNodeIndex,
            "a node names a child outside the node buffer, or a node is reached twice",
        ),
        (
            TraversalError::InvalidPrimitiveRange,
            "a leaf names primitives outside the primitive-index buffer",
        ),
    ];
    for (error, text) in cases {
        assert_eq!(error.to_string(), text);
        let error: &dyn Error = &error;
        assert!(error.source().is_none(), "no underlying cause");
    }
}

#[wasm_bindgen_test(unsupported = test)]
fn trace_tp_729_compact_errors_display_their_condition_and_are_errors() {
    let cases = [
        (CompactError::InvalidNodeIndex, "an interior node names children outside the node buffer"),
        (CompactError::OutputTooSmall, "the output buffer holds fewer nodes than the tree's interior"),
        (CompactError::NotATree, "more interior nodes are reached than a tree of this size has"),
        (CompactError::TooDeep, "more than 64 interior subtrees would wait at once"),
    ];
    for (error, text) in cases {
        assert_eq!(error.to_string(), text);
        let error: &dyn Error = &error;
        assert!(error.source().is_none(), "no underlying cause");
    }
}
