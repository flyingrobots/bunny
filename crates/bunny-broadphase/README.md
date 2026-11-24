# bunny-broadphase

Bounding volume hierarchy and sweep-and-prune broadphase structures for Bunny.

This crate provides fixed-point spatial acceleration helpers for collision and
query workloads. The public API is allocation-free at the call boundary: callers
provide node, primitive-index, and pair buffers, and the crate reports whether a
buffer is large enough for the requested operation.

## Features

* Flat array-backed BVH construction into caller-owned buffers, with two
  builders that write the same format: `build_bvh` (quartile splits) and
  `build_bvh_sah` (binned surface area heuristic, integer costs, stated tie
  rules, a depth cap and no recursion). See `docs/topics/bvh-construction/` in
  the repository.
* Stack-bounded BVH traversal for AABB and ray queries.
* Front-to-back closest-hit and any-hit ray traversal (`closest_hit`,
  `any_hit`) over the same trees, with a caller-supplied primitive test.
  Ties go to the lowest primitive index, and the answer does not depend on
  the tree's shape; see `docs/topics/ray-tracing/` in the repository.
* A compact copy of a built tree for ray traversal (`compact_bvh`), whose
  nodes hold both children's boxes, searched by `compact_closest_hit` and
  `compact_any_hit` with the same calls and answers as the searches above.
* Counted forms of the compact searches (`compact_closest_hit_counted`,
  `compact_any_hit_counted`) that also return a `TraversalCounts` record of
  the box tests, visits and primitive tests one search made.
* Refit of a built tree to moved primitives (`refit_bvh`): exact new bounds
  with the topology and primitive order unchanged, in one pass with no
  allocation, refusing a malformed buffer without writing to it. See
  `docs/topics/bvh-construction/` in the repository.
* Deterministic sweep-and-prune pair generation with sorted pair output.
* A uniform grid broadphase (`uniform_grid_pairs`, `bunny-uniform-grid/1`)
  with exact integer cell coordinates (`floor(raw / cell_raw)`), sorted
  (cell, box) keys in caller scratch sized by `grid_key_count`, and the same
  pairs in the same order as `sweep_and_prune`; a short pair buffer keeps the
  first pairs and the report counts all of them. See `docs/topics/uniform-grid/`
  in the repository.
* Q32.32 fixed-point geometry throughout the core broadphase path.

## Examples

| Command | Shows |
| --- | --- |
| `cargo run -p bunny-broadphase --example bvh_ray_query` | `build_bvh_sah` into caller-sized buffers over four `TraceTriangle`s, `closest_hit` with its hit distance, facing and barycentrics, `any_hit` for a ray that misses, and a ray refused for leaving the trace domain. |

## License

Apache-2.0.
