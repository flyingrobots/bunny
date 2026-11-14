# bunny-broadphase

Bounding volume hierarchy and sweep-and-prune broadphase structures for Bunny.

This crate provides fixed-point spatial acceleration helpers for collision and
query workloads. The public API is allocation-free at the call boundary: callers
provide node, primitive-index, and pair buffers, and the crate reports whether a
buffer is large enough for the requested operation.

## Features

* Flat array-backed BVH construction into caller-owned buffers.
* Stack-bounded BVH traversal for AABB and ray queries.
* Front-to-back closest-hit and any-hit ray traversal (`closest_hit`,
  `any_hit`) over the same trees, with a caller-supplied primitive test.
  Ties go to the lowest primitive index, and the answer does not depend on
  the tree's shape; see `docs/topics/ray-tracing/` in the repository.
* A compact copy of a built tree for ray traversal (`compact_bvh`), whose
  nodes hold both children's boxes, searched by `compact_closest_hit` and
  `compact_any_hit` with the same calls and answers as the searches above.
* Deterministic sweep-and-prune pair generation with sorted pair output.
* Q32.32 fixed-point geometry throughout the core broadphase path.

## License

Apache-2.0.
