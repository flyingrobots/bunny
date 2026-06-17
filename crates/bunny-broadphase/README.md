# bunny-broadphase

Bounding volume hierarchy and sweep-and-prune broadphase structures for Bunny.

This crate provides fixed-point spatial acceleration helpers for collision and
query workloads. The public API is allocation-free at the call boundary: callers
provide node, primitive-index, and pair buffers, and the crate reports whether a
buffer is large enough for the requested operation.

## Features

* Flat array-backed BVH construction into caller-owned buffers.
* Stack-bounded BVH traversal for AABB and ray queries.
* Deterministic sweep-and-prune pair generation with sorted pair output.
* Q32.32 fixed-point geometry throughout the core broadphase path.

## License

Apache-2.0.
