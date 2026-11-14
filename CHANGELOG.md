# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Fixed

* The published `bunny-codec` package no longer contains a test that cannot
  compile: `tests/ply_ascii_stanford_tests.rs` is excluded together with the
  Stanford fixture it reads with `include_bytes!`. `bunny-contract` had the same
  fault, with `tests/generated_version_tests.rs` reading the generated files at
  the repository root, and is fixed the same way. `scripts/publish-crates.sh
  packaged-tests`, now part of `verify` and so of the release workflow, packages
  every crate, unpacks the archives side by side with the sibling crates patched
  to the unpacked copies, and runs `cargo check --tests` over them; on the old
  manifests it fails and names both tests. No library code changes
  (flyingrobots/bunny#283).

### Fixed

* The ASCII PLY chapter states what the reader does at three points where it said
  otherwise; no code changes. A face element before the vertex element, or before `x`, `y`
  and `z` are all declared, is `MissingVertexElement` at its `element face` line (not the
  `end_header` line). A face count that is not a run of digits (`+3`, `3.0`, `x`) is
  `InvalidIndex`, and a run of digits other than 3 is `NonTriangularFace`; `03` is accepted
  as 3. New tests pin these, the certain-overflow and certain-underflow shortcuts of the
  decimal conversion (changing `CERTAIN_OVERFLOW_MAGNITUDE` from 11 to 12 passed every test
  before) and the type check of skipped vertex properties (removing it passed every test
  before) (flyingrobots/bunny#323).

### Added

* `bunny-codec`'s `parse_ascii_ply_with_limits(input, &AsciiPlyLimits)` refuses an ASCII
  PLY input whose declared vertex count, declared face count or line length (bytes
  without the terminator, blank lines included) is past an inclusive limit, with an
  `AsciiPlyLimitedError::Limit` naming the limit, the maximum, the observed value and
  the line; counts are refused before any body line is read. `AsciiPlyLimits::DEFAULT`
  is `2^24` vertices, `2^24` faces and 4096-byte lines; `UNLIMITED` is `usize::MAX`
  for each. `parse_ascii_ply` is not changed: a differential test over 20 000
  generated valid and invalid inputs shows the new function with unreachable limits
  returns the same meshes and the same errors, and the reader already used constant
  memory and linear time, so the limits refuse early rather than close a resource
  hole. Text in skipped vertex columns is still counted, never parsed. A Lumen caller
  keeping its retired reader's protection passes
  `AsciiPlyLimits::new(1 << 20, 1 << 20, 256)` (flyingrobots/bunny#300).

* `bunny-perf`'s `smooth` group times smooth shading per hit on 1 024 closest
  hits on the benchmark sphere: `shade/smooth_normal` (the shipped
  interpolation from exact numerators), `shade/smooth_normal_divided` (a
  replica of the rejected alternative with two floored `barycentrics`
  weights), and `shade/smooth_hit_shading` beside `shade/flat_hit_shading_mesh`
  on the same hits. A test checks that the replica and `smooth_normal` agree
  within both bounds. Ledger entry 13 records the cost; closes
  flyingrobots/bunny#289.

* `bunny-optics` tests pin the start point's two-step case and its tie rule:
  kernel-reported hits that take zero, one and two steps (a regular-octahedron
  face, whose equal normal components are the proof's worst case, and a general
  triangle), five hits on planes whose largest normal components tie, and a
  generated check of 100 000 rays against the rule with an exact plane test.
  Before them only the comparison with the retained 0.7.1 code caught a
  one-step limit, and nothing caught the highest axis on a tie. The optics
  chapter no longer says that two steps were never needed: 1 808 of 99 616
  generated hits needed two. No library function changes
  (flyingrobots/bunny#288).

* `bunny-optics` tests pin the three rounding rules of
  `bunny-diffuse-shading/1` that had none: the light cosine rounds to nearest,
  and `round_shift` (cosine and hemisphere direction) and the frame division
  round ties to even. Exact ties with even and odd truncations, in both signs
  where the sign is observable, and the nearest inputs either side of a tie,
  including frame-division ties on real triangles found by search.
  OPTICS-TP-042 now asserts the exact ties-to-even result instead of "within
  half a raw unit". Before them, flooring the cosine or rounding either kind of
  tie away from zero passed every test. No library function changes
  (flyingrobots/bunny#298).

* `bunny-query` tests check `TraceRay::hit_triangle` against an independent
  signed-volume oracle in 256-bit integers (TRACE-TP-315): 40 000 generated
  cases on and within three numerator units of every edge and vertex, with
  domain-corner, scaled, long-direction, grazing and degenerate families, and
  asserted minimum counts of each kind. Before it, relaxing the edge rule by one
  numerator unit failed only the comparison with the frozen copy of the kernel.
  TRACE-TP-005 and TRACE-TP-203 now count their three crossings. No library
  function changes (flyingrobots/bunny#294).

* `bunny-optics`'s `SmoothHit::bounce` samples the bounce around the shading
  normal and returns a `SmoothBounce`: `Ray` when the direction is strictly
  above the exact geometric plane (`SmoothHit::is_above_surface`, an exact
  integer sign), `BelowSurface` otherwise, with no ray traced and a zero
  contribution. This loses energy at strongly tilted shading normals, by design
  of `bunny-smooth-shading/1`. The `smooth_normal` oracle now sums exactly with
  big integers, so determinants up to `2^117` and every `rho` are checked
  against the stated bound. Part of flyingrobots/bunny#289.

* `bunny-num` tests pin the exact ties-to-even result at every rounding site:
  `checked_mul` and `Mul`, `checked_div`, `Div` and `FixedDivisor`, and float
  ingress and egress, with hand-derived raw values for both signs, both
  neighbour parities, near-ties and operands at the widest intermediates. Before
  them no `bunny-num` test failed when multiplication or float conversion rounded
  ties away from zero or truncated. No library function changes. See the new
  `docs/topics/fixed-point-rounding/` chapter (flyingrobots/bunny#287).

* `bunny-optics`'s `SmoothHit` (`bunny-smooth-shading/1`, `SMOOTH_SHADING_ID`)
  shades a `SurfaceHit` with the `smooth_normal` of its triangle's vertex
  normals at the hit's exact weights. The frame faces the arrival side by the
  sign of an exact dot product with the face's exact plane normal (flipped when
  negative), and falls back to the geometric frame when that product is zero, a
  vertex normal is missing or the normals cancel. `light_sample` keeps the
  geometric shadow ray and side test and takes the cosine against the shading
  normal, clamped at zero. The `SurfaceHit`, its start point and
  `bunny-diffuse-shading/1` are unchanged. Part of flyingrobots/bunny#289; see
  `docs/topics/optics/README.md#smooth-shading`.

* `bunny-query`'s `TriangleHit::barycentric_numerators` returns the exact `u`,
  `v` and `det` (each below `2^117`) that the triangle test decided the hit
  with, as a `BarycentricNumerators`, which `BarycentricNumerators::new` also
  builds from any point on a triangle. `barycentrics` is unchanged.
  `bunny-optics`'s `smooth_normal` (`bunny-smooth-normal/1`,
  `SMOOTH_NORMAL_ID`) interpolates three vertex normals with those weights:
  a normal shared by every weighted vertex is returned unchanged; otherwise the
  weights are shifted right by a common amount (floored) until `det` is below
  `2^62`, combined exactly and normalized once, each component within
  `1 + 2^-26 / rho` raw units of the exact interpolation's unit vector, and
  `None` when the normals cancel. Every rounding has a golden input on its
  boundary. No existing output changes. Part of flyingrobots/bunny#289; see
  `docs/topics/optics/README.md#smooth-normal`.

* `bunny-optics` computes area-weighted vertex normals for an indexed triangle
  mesh: `VertexNormals::new(&positions, &triangles)` (`bunny-vertex-normals/1`,
  `VERTEX_NORMALS_ID`) sums the exact `i128` cross products of each vertex's
  faces and normalizes once, as `TriangleSurface::new` does, so each component is
  within one raw unit of the exact unit vector and the result does not depend on
  face order, a cyclic rotation of a face's corners or vertex labels (reversing a
  face's winding negates its contribution). A vertex with no face or a zero sum
  has `None`; positions outside the trace domain, an out-of-range index or more
  than `MAX_NORMAL_TRIANGLES` (`2^32`) triangles return a `VertexNormalError`.
  No existing output changes. The `bunny-perf` group `normals` times a build
  (`shade/vertex_normals`). Part of flyingrobots/bunny#289; see
  `docs/topics/optics/README.md#vertex-normals`.

* `bunny-mesh` and `bunny-codec` have a `mesh-hash` feature, on by default.
  In `bunny-mesh` it provides `compute_mesh_hash`, the only item that uses
  `sha2`; `bunny-codec` uses none of it and forwards the feature. With default
  features on, both crates' public items and dependency graphs are unchanged.
  With `default-features = false`, neither crate reaches a crates.io crate and
  every `bunny-codec` reader (`parse_ascii_ply`, `parse_decimal_q32_32`,
  `parse_binary_ply`, `parse_obj_text`, `decode_compressed_mesh`) is still
  available. The full gate lists both crates' dependency graphs with default
  features off and fails on a third-party crate, then lints, tests and
  wasm-checks them in that configuration (flyingrobots/bunny#291).

* `bunny-codec` reads ASCII PLY triangle meshes: `parse_ascii_ply` accepts
  `format ascii 1.0` with `x`, `y`, `z` and any other scalar vertex properties
  (skipped by their declaration) and `list uchar int vertex_indices` faces, and
  returns a borrowed, validated `AsciiPlyMesh` with Q32.32 positions
  (`AsciiPlyVertex`), forward iterators, and `AsciiPlyError` values that carry a
  kind and a 1-based line number. Decimal coordinates convert to Q32.32 exactly,
  with integer arithmetic only, by the rule `bunny-decimal-q32-32/1`: the exact value
  times 2^32, rounded to nearest with ties to even; `parse_decimal_q32_32` exposes
  it with typed errors `Invalid`, `Overflow` (decided after rounding) and `TooLong`
  (more than 28 significant digits for a value of magnitude in [1e-10, 1e10)).
  LF and CRLF line endings, blank lines, header comments and trailing whitespace each
  have a stated rule; see `docs/topics/ascii-ply/` (flyingrobots/bunny#247). The
  binary PLY reader is unchanged.

* Code Dojo enforces every rule of Echo's `ban-nondeterminism` scan under
  Echo's rule IDs (`time-system`, `time-instant`, `thread-sleep`,
  `host-parallelism`, `random`, `host-callback`, `network`,
  `unordered-container`, `hand-hashed-table`, `json-serialization`,
  `wasm-serde`, `std-env`, `std-fs`, `std-process`) in the core crates,
  `bunny-codec` and `bunny-contract`, tests included. The rules resolve
  paths through `use` declarations, so renamed imports, module aliases,
  `use` groups and glob imports are caught, and they read macro bodies.
  The full gate also rejects banned crates (`rand`, `js-sys`,
  `serde_json` and others) in those crates' normal dependency graphs.
  `docs/CODE_DOJO.md` maps each Echo rule to its Code Dojo rule
  (flyingrobots/bunny#279).

* `bunny_optics::SurfaceHit::try_new`, `SurfaceHit::try_bounce_ray` and
  `ShadingFrame::try_cosine_direction` return `Result<_, SecondaryRayError>`
  and say why an input was refused. `SecondaryRayError` (`#[non_exhaustive]`)
  has one variant per cause, each carrying the values the check saw:
  `HitPointUnrepresentable { distance }`, `HitPointOutsideTraceDomain { point }`,
  `NotOnArrivalSide { point }`, `StartOutsideTraceDomain { start }`,
  `SampleOutsideUnitInterval { sample }` and the defensive
  `BounceRayRefused { origin, direction }`. They are the diagnostic path: the
  existing `Option` functions stay the fast path, with their 0.7.1 bodies and
  cost, and a caller asks the `try_` form for the reason only after a `None`.
  A differential test over generated inputs (a fixed and a fresh seed) shows
  that each fast form returns `Some(x)` exactly when its `try_` form returns
  `Ok(x)` and that both keep every output of the 0.7.1 code, retained in the
  tests (flyingrobots/bunny#280).

### Fixed

* Code Dojo enforces what `CODE_STANDARDS.md` Rule 4 says beyond Echo's
  rule list: the `Path` methods that read the filesystem (`exists`,
  `metadata`, `canonicalize` and seven more), `std::os::<platform>::fs`,
  `io`, `prelude`, `net` and `process`, `std::os::fd`, `std::path::absolute`
  and `std::io::pipe` are findings, terminal input is a new `host-input`
  rule, and doc tests are scanned like other code. The docs compare the list
  with Lumen's gate. Part of flyingrobots/bunny#321.

* Code Dojo names every item of `std::thread` and `std::time` deliberately
  instead of matching whole-segment prefixes, checked by a test against the
  pinned toolchain's documentation: `sleep_ms`, `sleep_until`, `park`,
  `park_timeout`, `park_timeout_ms` and `yield_now` are now `thread-sleep`
  findings and `SystemTimeError` a `time-system` one. In core crates any name
  ending in `_f32` or `_f64`, also in a `use` declaration, is a
  `float-boundary` finding, so
  `d.mul_f64(d.as_secs_f64())` and `Duration::from_secs_f64(..)` no longer
  pass with no float token; six float DTO conversions in `bunny-linalg` and
  `bunny-geom` that the gate could not see before now carry waivers. A waiver
  between a doc comment and `fn` now covers the function. Part of
  flyingrobots/bunny#321.

* Code Dojo no longer lets code or host files into a deterministic crate
  unscanned: `include!` is banned in every spelling (`include-source`), a
  `#[path]` module (also under a nested `cfg_attr` or emitted by a macro)
  must name a `.rs` file inside the crate directory that the gate scans, and
  `include_str!`/`include_bytes!` must name a literal path inside it
  (`include-data`). `env!` and `option_env!` are now `std-env` findings, as
  in Lumen's gate; the `bunny-contract` test that reads `CARGO_PKG_VERSION`
  and the committed generator outputs carries waivers. Part of
  flyingrobots/bunny#321.

* Code Dojo waivers are comments only. `dojo: allow` text inside a string,
  raw-string or byte-string literal, a doc comment, a `#[doc]` attribute or a
  macro's tokens no longer waives anything, and a comment with two clauses
  waives neither rule. A waiver on a line of its own now covers the item,
  statement, field, variant or match arm that starts below it, every line of
  it, instead of a fixed window of three lines: one waiver no longer silences
  the next statement, and a `float-boundary` waiver above a function covers
  the whole function (the waiver-scope part of flyingrobots/bunny#286). A
  refused waiver no longer hides a valid one, a rule ID must match exactly
  (`std-fs-extra` does not waive `std-fs`), and a manifest waiver must be a
  whole-line TOML comment outside any multi-line string. A block comment over
  several lines is read whole, so two clauses in it waive neither rule. Part
  of flyingrobots/bunny#321.

* Code Dojo's nondeterminism rules resolve `extern crate` renames like imports
  (`extern crate std as host; host::env::var(..)` is a `std-env` finding, and
  `extern crate rand;` a `random` one) and compare identifiers without their
  `r#` prefix, so `r#std::fs::read`, `use std::collections::r#HashMap`,
  `r#f64`, `x.r#unwrap()` and `r#panic!` no longer get past the gate. A test
  now fails if `self` in a `use` group is treated as a path segment. Part of
  flyingrobots/bunny#321.

* `bunny_optics::SurfaceHit::new` no longer overflows `i128` for a hit whose
  point is far outside the trace domain (for example a `hit.distance` of
  `-2^30` units set by the caller): 0.7.1 panicked there in debug builds and
  returned `None` in release builds; it now returns `None` in both. Hits the
  trace kernel reports are not affected (flyingrobots/bunny#280).

* `bunny_broadphase::intersect_aabb`, `intersect_ray`, `closest_hit` and
  `any_hit` no longer loop forever on a node buffer with a reference cycle
  (a child index naming an ancestor). Each allows at most
  `nodes.len() / 2` interior visits, which no tree can exceed, and returns
  `TraversalError::InvalidNodeIndex` at the next one, after a bounded number
  of callbacks. On a tree, including one whose only faults are out-of-range
  indices or a depth past 64, every result, error and callback is unchanged,
  which a differential test against the previous functions checks. Buffers
  that are not trees can now get a different answer: a cycle that used to
  overflow the stack returns `InvalidNodeIndex` instead of `StackOverflow`,
  and a shared subtree whose unfolding needs more than `nodes.len() / 2`
  interior visits returns `InvalidNodeIndex` instead of a result that
  reported its primitives more than once. No variant was added:
  `TraversalError` is not `#[non_exhaustive]`, so a new variant would break
  exhaustive matches, and `InvalidNodeIndex` is what the compact searches
  already return for a cycle; it does not tell a cycle from an out-of-range
  index (flyingrobots/bunny#269).

### Changed

* Code Dojo's `ambient-state` rule is split into the Echo rule IDs above,
  and `nondeterministic-map` is renamed `unordered-container`; a waiver now
  names one ban. No waiver in the repository used the old names.
* `float-boundary` also rejects float paths such as `f32::NAN` or
  `f64::from_bits` and floats inside macro bodies in core crates. Three
  `f32::from_bits` calls in `bunny_num::fixed_q32_32::to_f32`, the
  documented float egress, carry waivers; no code changed.

* `bunny-optics` no longer denies the whole `clippy::nursery` group, which
  `CODE_STANDARDS.md` Rule 10 bans; it denies three nursery lints by name
  (`missing_const_for_fn`, `redundant_pub_crate`, `use_self`), each with its
  reason, and every lint allowance in the crate now states its reason. Rule 9
  of `CODE_STANDARDS.md` lists `bunny-optics` as a core crate, as `xtask`
  already enforced (flyingrobots/bunny#280).

## [0.7.1] - 2026-11-14

This is the first 0.7 release on crates.io. `0.7.0` was tagged and has a GitHub release,
but nothing was published to crates.io (the upload of the first crate was refused with an
authentication error), so every Bunny crate on crates.io was still at `0.6.0` and
`bunny-optics` had never been published. `0.7.1` is built from `main` and includes
everything listed under `0.7.0` below plus the changes listed here. For a reader coming
from `0.6.0` both lists apply, and the first entry below is the breaking change.

### Changed (behaviour)

* `bunny_query::ray_intersects_aabb`, and so
  `bunny_broadphase::intersect_ray`, no longer report a miss when a slab
  distance or a coordinate difference overflows. The inputs that change are
  exactly those for which the old function stopped on an overflowing
  `bound - origin` subtraction or `(bound - origin) / direction` division:
  a direction component of a few raw units (for a slab of half-width `k`
  centred on the origin, any component of at most `2k` raw units), or a bound
  2^31 units or more from the origin. For them the function now returns
  `Some((t_enter, t_exit))` when the ray reaches the box ahead of its origin
  and `None` when it truly misses, decided exactly with `i128` rationals; a
  slab distance that does not fit Q32.32 saturates to the minimum or maximum
  by sign, and coordinate differences are widened to `i128` instead of
  saturated. Every hit the function reported before is reported with the same
  bits, and every miss not caused by overflow is still a miss; a differential
  test against the retained old implementation and an exact rational oracle
  check both. Per the Fidelity Rule this contract change needs a version that signals it,
  which is why the version moved from `0.6.0` to the `0.7` line and this is not a patch
  release (flyingrobots/bunny#211, PR #236).

* `FixedMat4::try_look_at` is corrected from its 0.7.0 form (flyingrobots/bunny#273).
  It is new relative to 0.6.0, so a reader upgrading from 0.6.0 sees only the
  corrected behaviour described here; the two input classes below are the ones on which
  it differs from the tagged 0.7.0 code, and every other input that returned `Ok` keeps
  every output bit, checked against the 0.7.0 implementation on 60 000 seeded
  inputs. First, an `up` vector that is exactly parallel or antiparallel to
  `target - eye` (their exact integer cross product is zero), or zero, now
  returns `LookAtError::ParallelUp`; before, rounding of the normalized view
  direction made some such inputs return `Ok` with a roll made of rounding
  error (for example `eye = (0, 0, 0)` and `target = up = (1, 2, 3)`). Nearly
  parallel inputs are still accepted. Second, a translation entry whose finished
  value `-(row . eye)` is representable no longer returns `Overflow` because an
  intermediate term or sum was out of range (for example `eye.x = i64::MIN` raw,
  `target = (0, 0, 0)`, `up = (0, 1, 0)`, where the entry is `-2^31` units);
  each entry is now summed in a wider integer and only the finished entry is
  checked. The seeded orthonormality test now draws the 2^38 range it states;
  the documented bounds hold over it.

### Added

* Added a compact BVH node layout to `bunny-broadphase` (flyingrobots/bunny#215):
  `compact_bvh` converts a tree from `build_bvh` into nodes that hold both
  children's boxes and references, with leaf ranges in the parent (`CompactNode`,
  `CompactChild`, `CompactRoot`, `CompactError`), and `compact_closest_hit` and
  `compact_any_hit` search it over a `CompactView`. They make the same primitive
  calls and return the same results as `closest_hit` and `any_hit` on the source
  tree; malformed input is a typed error, including reference cycles. On one
  benchmark scene both searches are about 10 % faster than `closest_hit` and
  `any_hit` (9.6 to 10.2 % in two runs of the final code, best p50, Apple M5 Pro in
  the rust-dev:1.96.0 container; ledger entry 8). Existing functions are unchanged.
* Added `bunny-perf` cases that estimate the memory share of a box test
  (`trace/kernel_box_load`, `trace/kernel_box_load_chain`,
  `trace/kernel_box_entry_addr_chain`), a two-box replica of the box test
  (`trace/kernel_box_single`, `trace/kernel_box_pair`), `closest_hit` on a tree
  with shuffled sibling pairs (`trace/closest_hit_shuffled`), and a
  `trace_compact` group that times the compact layout, its conversion and three
  node orders.

* Added `build_bvh_sah` to `bunny-broadphase`: a binned surface-area-heuristic
  BVH builder that writes the same buffers and node format as `build_bvh`, so
  every traversal accepts its trees. 16 centroid bins per axis, integer `u128`
  costs with a per-node shift that rules out overflow, stated tie rules (lower
  axis, then lower plane; an equal split and leaf cost makes a leaf), leaves of
  at most four primitives below a depth cap of 60, a stable partition (leaves
  list primitives in ascending index order), no recursion and no allocation.
  Its parameters are public constants in `bunny_broadphase::bvh`. `build_bvh`'s
  output is unchanged, and both builders' bytes are now locked by golden
  fingerprints. The new BVH construction chapter (`docs/topics/bvh-construction/`)
  states both builders' contracts.
* Added the `build` and `sah` feature groups to `bunny-perf`: build-time cases
  for both builders (`build/bvh`, `build/bvh_sah`) and the benchmark rays traced
  through the SAH tree (`trace/closest_hit_sah`, `trace/any_hit_sah`), with a
  check that both trees give the same `closest_hit` and `any_hit` answers on
  every ray. Ledger entry 9 in `docs/topics/performance/` records the
  measurement.
* Added diffuse shading for a one-bounce path tracer to `bunny-optics`, as
  `bunny-diffuse-shading/1` (`SHADING_ID`), integer-only, with golden vectors
  and stated bounds (flyingrobots/bunny#160, the diffuse slice).
  `TriangleSurface` holds a triangle's exact plane, a unit normal within one
  raw unit, and the Duff et al. orthonormal frame (`ShadingFrame`); zero-area
  triangles return `None`. `SurfaceHit` applies the facing rule (the frame
  faces the incoming ray, from `TriangleHit::front_face`) and starts every
  secondary ray at the hit point moved at most two raw units along one axis to
  lie strictly on the arrival side of the exact plane, so bounce and shadow
  rays never hit their own triangle, a coplanar neighbour or a coincident
  duplicate, with no distance epsilon. `ShadingFrame::cosine_direction` maps a
  sampler point to a cosine-weighted direction (polar disk lifted to the
  hemisphere) within 4 raw units of the ideal map, with every component within
  1.0 and a dot product with the normal of at least `2^15` raw units.
  `DirectionalLight` takes irradiance on a perpendicular surface (a white
  surface reflects `E * cos / pi`, with `FRAC_1_PI` raw 1 367 130 551; `PI` is
  raw 13 493 037 705), gives the shadow ray along `toward_light` and the
  any-hit limit `SHADOW_T_MAX` (`2^41`, which covers the trace domain).
  `SkyLight` and `Albedo::reflect` (`albedo * (direct + indirect)`; the
  cosine-weighted bounce has no `pi`) complete the step, and the optics chapter
  states the order of operations for one path. Adds the `shade` group to
  `bunny-perf`. No existing function changes.

* Added two `bunny-perf` cases in the `trace` group, `trace/triangle_hit` and
  `trace/triangle_beyond_limit`, which time `TraceRay::hit_triangle` alone on the
  benchmark scene's nearest hits with no limit and with the limit one raw unit
  short. Added a frozen reference copy of the triangle test's arithmetic
  (`crates/bunny-query/tests/support/trace_reference.rs`) with equivalence
  tests over seeded, constructed and domain-corner corpora at limits around the
  true distance, and scene tests that compare every triangle test of
  `trace/closest_hit` with it and count how many reach the division. No library
  function changes. A pre-check that rejects hits beyond the limit before the
  division (flyingrobots/bunny#214) was measured and not shipped: only 24 of 714
  crossings on the scene are beyond the limit, and the result is within noise;
  see "Tried And Rejected" in the performance chapter.
* Added deterministic sampling to `bunny-optics`. `PixelStream`
  (`bunny-sample-hash/1`) is a counter-based hash: 64 mixed bits per (seed,
  pixel x, pixel y, sample index, dimension) from three chained SplitMix64
  absorptions, with no state, so values do not depend on evaluation order or
  thread count; keys that differ only in the pixel, or only in (sample,
  dimension), never collide. `fraction_from_bits` and `PixelStream::fraction`
  give the top 32 bits as a Q32.32 fraction in `[0, 1)`, never 1.0.
  `PixelSampler` (`bunny-sobol-owen/1`) gives points in `[0, 1)^2` per (seed,
  pixel, sample, dimension pair): the first two Sobol dimensions with a
  hash-based Owen scramble (Burley 2020), where every aligned block of `2^k`
  samples is a (0, k, 2)-net, and `camera_sample` returns pair 0 as a
  `PixelSample` for `PinholeCamera::ray`. `Xoroshiro128Plus` carries Echo's
  `warp-math` `Prng` bit for bit (an empty `next_int` range returns `None`
  instead of panicking; `next_unit` is `next_f32` as exact Q32.32). Golden
  vectors, an exact stratification property, order-independence and
  avalanche guards run natively and under WebAssembly. The `bunny-perf`
  harness has a new `sample` group, and `cargo xtask perf` now leaves out of
  the base manifest any harness crate the base revision lacks.
* Recorded the ray-packet spike (flyingrobots/bunny#217) as entry 7 under "Tried
  And Rejected" in the performance chapter: a packet traversal that returned exactly
  `closest_hit`'s hit for every ray, with an optional whole-packet interval test whose
  exactness is proved against `box_entry`'s rounding, measured on the benchmark's 1 024
  camera rays in 2 x 2, 4 x 4 and 8 x 8 tiles and in shuffled packets. On the Apple M5 Pro
  no variant won outside noise (shared descent -4 % at best, culling 14 to 43 % slower;
  shuffled packets 30 to 86 % slower), so no packet API ships; the x86-64 CI readings,
  which favour the shared descent, are recorded for a later target-specific measurement.
* Added pixel-value math to `bunny-optics`, integer-only and over
  caller-provided values and slices (no image type, no allocation).
  `PixelAccumulator` sums linear radiance samples exactly in `i64` (each
  channel `0..=2^52` raw, at most 1 024 samples, so no sum exceeds `2^62`), so
  the result does not depend on sample order or on how partial accumulators are
  merged; out-of-range samples and over-full pixels return a typed `PixelError`
  and leave it unchanged. `resolve` returns the mean rounded to nearest with
  ties to even. `ToneCurve::SrgbV1` (`bunny-tone-srgb/1`) is the sRGB transfer
  function from a 1 153-knot table with floored linear interpolation, within
  `1.49 * 10^-6` of the exact curve and monotone; `ToneCurve::LinearClampV1`
  (`bunny-tone-linear-clamp/1`) clamps to `[0, 1]`. `quantize_8` and
  `quantize_16` (`bunny-unorm8/1`, `bunny-unorm16/1`) round to nearest with ties
  to even; 8-bit and 16-bit sRGB codes are within one code of the correctly
  rounded codes (exceptions within 9 600 inputs of a code boundary, stated in
  the optics chapter). `write_rgba8` (`bunny-rgba8-opaque/1`) writes one pixel
  with alpha 255 into a four-byte slice and refuses any other length. Golden
  vectors, an `f64` reference oracle in the tests and eight recorded mutations;
  `bunny-perf` gains a `pixel` group with `pixel/resolve_srgb_rgba8`.

### Changed

* `TraceRay::box_entry` is cheaper, with the same result for every input
  (flyingrobots/bunny#258): slab differences in `i64` (bitwise not turns a
  negative direction into a forward one) instead of `i128`, parallel axes in
  the same path, and no data-dependent branch of its own. `TraceRay` is
  unchanged (72 bytes, same fields). On one benchmark scene (Apple M5 Pro in the rust-dev:1.96.0
  container, one full-table run, best p50) `trace/closest_hit` is 16.9 % faster
  (221.72 to 184.33 ns per ray), `trace/kernel_box_entry` 21.1 % and the compact-layout
  search `trace/closest_hit_compact` 17.4 %. With the compact layout of ledger entry 8
  as well, `trace/closest_hit_compact` takes 164 ns per ray against 222 ns for
  `trace/closest_hit` before either change. On x86-64 (GitHub's shared runner, coarse)
  `trace/closest_hit` is 8.0 % faster. See ledger entry 10 in the performance chapter. The pre-#258 box test
  is kept as a reference oracle (`crates/bunny-query/tests/support/box_entry_reference.rs`)
  with equivalence tests over seeded, extreme-coordinate and benchmark-scene
  corpora (`TRACE-TP-308` to `TRACE-TP-312`).

### Fixed

* The pixel path in `bunny-optics` no longer slows down when unrelated code is added to the
  crate (flyingrobots/bunny#271): the small per-channel helpers and the public entry points
  are marked `#[inline]`, so their inlining no longer depends on how the release profile
  splits the crate into codegen units. No output bit changes.

## [0.7.0] - 2026-11-13

This release was tagged but not published to crates.io; it is superseded by 0.7.1.

This is an early cut of the 0.7 line. It exists so downstream repositories can
use the view, projection, ray-tracing and clipping APIs now; the rest of the
`v0.7.0 Geometry Law and Primitive Coverage` milestone (quaternions,
interpolation and curves, degeneracy policy, robust predicates, shapes,
topology, mass properties) is not in this release and continues as additive
`0.7.x` releases. `bunny-optics` is a new published crate. `bunny-perf` is a
workspace tool and is not published.

### Changed (behaviour)

* `bunny_query::ray_intersects_aabb`, and so
  `bunny_broadphase::intersect_ray`, no longer report a miss when a slab
  distance or a coordinate difference overflows. The inputs that change are
  exactly those for which the old function stopped on an overflowing
  `bound - origin` subtraction or `(bound - origin) / direction` division:
  a direction component of a few raw units (for a slab of half-width `k`
  centred on the origin, any component of at most `2k` raw units), or a bound
  2^31 units or more from the origin. For them the function now returns
  `Some((t_enter, t_exit))` when the ray reaches the box ahead of its origin
  and `None` when it truly misses, decided exactly with `i128` rationals; a
  slab distance that does not fit Q32.32 saturates to the minimum or maximum
  by sign, and coordinate differences are widened to `i128` instead of
  saturated. Every hit the function reported before is reported with the same
  bits, and every miss not caused by overflow is still a miss; a differential
  test against the retained old implementation and an exact rational oracle
  check both. Per the Fidelity Rule this contract change needs a version that signals it,
  which is why this release is `0.7.0` and not a patch release (flyingrobots/bunny#211,
  PR #236).

### Added

* Added the `bunny-optics` crate with a deterministic pinhole camera.
  `PinholeCamera::new` takes a position, a look direction, an up hint, the
  tangent of half the vertical field of view (no trigonometry) and an integer
  image size, derives a right-handed basis (`right = forward x up`) and refuses
  degenerate input with a typed `CameraError`. `PinholeCamera::ray` returns a
  `bunny_query::TraceRay` for a `PixelSample` (pixel plus sub-pixel offset in
  `[0, 1)`; pixel `(0, 0)` is top-left and rows grow downward, as in the
  viewport conversion) with no square
  root or division per ray: directions are not normalized but scaled by
  `2^-scale_shift`, chosen at construction from the image corners so every
  component is at most 1.0. Each component is within one raw unit plus
  `(width + height) / 2^33` of the exact image-plane point. `point_at` gives the
  point at a hit distance. Images up to 32768 pixels per side and tangents up
  to 256.0 are accepted. See the new `docs/topics/optics/` chapter and test plan.
* Added `bunny-perf --stats` and the `trace/kernel_*` cases: per-ray counters
  for `closest_hit` and `any_hit` (nodes, box tests, triangle-test stages,
  divisions, tree shape, shared nodes between neighbouring rays) from a harness
  replica of the search loop that a test checks against the shipped functions on
  every benchmark ray, and timed replays of the box and triangle tests the
  search makes. The performance chapter's new profile section says where the
  time of `trace/closest_hit` goes and ranks the stage 2 options. No library
  function changed and the core crates carry no counters.
* Added `FixedDivisor`, a Q32.32 divisor prepared for repeated exact division
  (Möller–Granlund division by an invariant integer): `divisor.checked_div(a)`
  returns exactly `a.checked_div(divisor)`, `None` included. Best p50 on an
  Apple M5 Pro: 3.5 to 4.0 ns against 6.4 to 6.5 ns for
  `FixedQ32_32::checked_div` (two runs, ledger entry 5).
* Added `PreparedRay3` and `ray_intersects_aabb_prepared`: the same
  `(enter, exit)` bits as `ray_intersects_aabb` on every input, including the
  overflow inputs that `ray_intersects_aabb` now decides exactly
  (flyingrobots/bunny#211), with no division per box. Best p50: 21.5 to 21.7 ns
  against 51.5 to 52.3 ns in the same runs, after a 26 ns preparation. For ray
  traversal that does not need exact distances, `TraceRay::box_entry` (6.0 ns
  after a 1.8 ns preparation) is faster, and on an x86-64 CI runner the
  prepared box test is only 1.11 times faster; see the ledger entry.
* Added `Viewport` and `ViewportError` to `bunny-linalg`: checked Q32.32
  conversion between NDC and viewport coordinates (`try_ndc_to_viewport`,
  `try_viewport_to_ndc`). The origin is the top-left corner, pixel `y` runs
  down, coordinates are continuous with pixel corners on integers, each
  coordinate is the floor of the exact raw value, and depth passes through. The
  round trip is within one raw unit per axis. The coordinate law's new
  Viewport Conversion section records the choices; golden vectors run natively
  and under WebAssembly. Existing functions are unchanged.
* Added `FixedMat4::try_look_at` and `LookAtError` to `bunny-linalg`: a checked
  Q32.32 right-handed look-at `view_from_world` matrix (camera looks down `-Z`).
  The basis is orthonormal within 2 raw units in length and pairwise dot
  product; eye equal to target, a zero or parallel `up`, and overflow return
  typed errors. Golden vectors run natively and under WebAssembly.
* Added the `scene` and `trace` feature groups to `bunny-perf`: a whole-scene
  ray benchmark (27 650 triangles, 1 024 camera rays) timing closest hit through
  the visitor API and through `closest_hit`, with a check that both choose the
  same primitive on every ray, and added `bunny-broadphase` to the `xtask perf`
  base crates. Ledger entry 4 in `docs/topics/performance/` records the
  measurement.
* Added `FixedPlane3::clip_polygon`, `PolygonClip` and `PolygonClipError` to
  `bunny-geom`: Sutherland-Hodgman clipping of a convex polygon to the closed
  front half-space, written into a caller-provided buffer (`n + 1` slots always
  suffice; a smaller one returns `OutputTooSmall`). Winding is preserved and
  output starts at the first kept input vertex; a vertex on the plane is kept
  without a duplicate cut; a result with fewer than three vertices is
  `Outside`. Cut points are computed in a canonical edge direction (ascending
  raw `(x, y, z)`) so polygons sharing an edge get identical points, because
  `clip_segment` can differ by direction at a rounding tie. Existing functions
  are unchanged. See `docs/topics/plane-clipping/`.
* Added `FixedMat4::try_orthographic` and `FixedMat4::try_perspective` to
  `bunny-linalg`: checked Q32.32 `clip_from_view` projection matrices that take
  a tangent (no trigonometry) and return the new `ProjectionError`. The
  coordinate law now defines the canonical NDC depth range `[0, 1]` (near to
  `0`, far to `1`) and a `-Z` camera, with golden vectors that run natively and
  under WebAssembly.
* Added closest-hit and any-hit BVH ray traversal to `bunny-broadphase`:
  `closest_hit`, `any_hit`, `ClosestHit` and `BvhView` (the borrowed node and
  primitive-index buffers). The search is front to back over existing
  `BvhNode` trees, uses `TraceRay::box_entry` for nodes and a caller closure
  for primitives, skips subtrees whose entry bound is beyond the best hit, and
  gives ties to the lowest primitive index. Its answer equals a test of every
  primitive whatever the tree's shape, under the caller conditions stated in
  the `docs/topics/ray-tracing/` chapter, which now documents the traversal
  and its correctness argument. `intersect_ray` and the other existing
  functions are unchanged.
* Added ray-tracing kernels to `bunny-query`: `TraceRay`, `TraceTriangle` and
  `TriangleHit`. `TraceRay::box_entry` is a division-free conservative ray-box
  test that never rejects a box the ray reaches and returns an entry bound
  within `2^11 + 2` raw units plus `distance / 2^30`; `TraceRay::hit_triangle`
  is an exact integer ray-triangle test (two-sided, inclusive edges, floored
  distance, barycentrics on demand). Inputs are limited to a trace domain of
  coordinates below 256.0 and direction components at most 1.0. Existing
  `bunny-query` functions are unchanged. See the new
  `docs/topics/ray-tracing/` chapter and test plan.
* Added `FixedPlane3`, `PlaneSide`, `FixedSegment3` and `SegmentClip` to
  `bunny-geom`: checked signed distance, three-way point classification, and
  segment clipping to the closed front half-space (a point exactly on the plane
  is kept). The clipped endpoint uses one checked division and rounds
  ties-to-even. See `docs/topics/plane-clipping/`.
* Added `crates/bunny-perf`, a timing harness for Bunny's math and query
  kernels (p50/p95/p99 nanoseconds per call on seeded inputs), and
  `cargo run --locked -p xtask -- perf --base <rev>`, which builds the same
  harness against a base revision and the working tree, runs them interleaved
  and prints a before/after table (median and best p50 across rounds;
  `--align-code` removes code-layout luck).
* `cargo xtask perf --target <triple> --runner <command>` measures other
  targets, such as WebAssembly under wasmtime, and every report names the
  target it measured.
* The `Perf` workflow has a hand-started, report-only platform matrix (x86-64
  and ARM Linux, Windows, Apple Silicon macOS, WebAssembly), and the
  performance chapter states the rules for target-specific implementations.
* Added the `Perf` GitHub workflow: pull requests get a before/after table in
  the job summary and fail when a case's best p50 slows by more than 20 %.
* Added the `docs/topics/performance/` chapter: the fidelity rule every
  optimization must satisfy, how measurements are made and read, and the
  optimization ledger.
* Added `FixedAngle`, Bunny's canonical angle (#110): a binary angle with
  `2^64` units per turn, wrapping arithmetic, and exact or symmetric-rounding
  conversions from turns, degrees and radians.
* Added deterministic `sin`, `cos` and `sin_cos` on `FixedAngle`: a
  quarter-wave table built at compile time by an integer Taylor series, cubic
  Hermite interpolation, and quadrant symmetry. Results are within one raw
  Q32.32 unit of the exact value, exact at quarter turns and
  bit-exact under negation, with golden vectors and the
  `docs/topics/angle-trigonometry/` chapter.

### Changed

* Added `isqrt_u128_two_divides` (crate-private), an exact integer square root that
  finishes with two hardware `u64` divisions, with unit tests against core's
  `u128::isqrt`. Re-measured on the corrected harness it makes `sqrt` 1.33 times and
  `length` 1.28 times faster but `normalize` about 4 % slower on aarch64, so every
  target keeps core's implementation; `sqrt_u128` output and speed are unchanged. An early
  version that marked `sqrt_u128` `#[inline]` read 71 % slower on `num/sqrt` on x86-64 CI
  and was fixed (see the performance ledger, entry 3).
* `FixedQ32_32::sqrt_u128` (and so `sqrt`, `FixedVec3::length`, `normalize`
  and the ray queries) now uses the table-seeded integer square root from the
  core library: the same exact bits as before, 4.9 times faster for `sqrt`,
  2.1 times for `normalize` and 1.6 times for a sphere hit (best p50, Apple
  M5 Pro in the rust-dev container). Equivalence tests keep the old bit-by-bit loop as the reference;
  the performance chapter's ledger records the method, trade-offs and full
  measurements.
* The lockfile takes `ninelives` 0.2.1 and `tokio` 1.54.1 (transitive
  dependencies of the host-side `wesley-core` tooling) to clear RUSTSEC-2026-0386.
  No published crate's dependency requirements changed.
* Bumped every publishable Bunny crate to `0.7.0`, including the inter-crate
  dependency requirements; regenerated contract witnesses now identify the
  generator as `bunny-wesley/0.7.0`. `bunny-optics` is added to the publish
  order after `bunny-query`.

### Fixed

* Fixed `bunny-perf` so every feature group builds on its own: the `case` helper
  compiled only with the `scene` or `trace` group, which made `cargo xtask perf`
  report the `num`, `linalg`, `query` and `angle` cases as new instead of
  comparing them against a base revision.
* Two `bunny-codec` test helpers use `as_chunks`, so clippy from newer
  toolchains no longer fails the gate.

## [0.6.0] - 2026-06-26

### Added

* Added the `docs/topics/coordinate-law/` living topic chapter and test plan for
  Bunny's right-handed coordinate frame, unit policy, transform naming
  convention, NDC reservations, and documentation source-of-truth rules.
* Added `docs/README.md` as the documentation spine for living topic chapters,
  test plans, historical design records, and PR documentation gates.
* Added `CONTRIBUTING.md` to explain Bunny's current-truth documentation
  technique and contract-graph workflow for new contributors.
* Added `cargo run --locked -p xtask -- topic-docs` and wired it into Code
  Dojo so topic test plans validate stable requirement IDs, case IDs, explicit
  oracles, evidence status, and implemented Rust test names.
* Added universal repo-respect receipt enforcement to Code Dojo and local Git
  hooks, plus `cargo run --locked -p xtask -- repo-respect receipt
  <short-topic>` for creating receipt templates before commit or PR handoff.
* Added the `docs/topics/deterministic-contract-profile/` living topic chapter
  and test plan for Bunny-owned deterministic scalar profiles, generated
  witnesses, and future codec boundaries.
* Extended `bunny-wesley` scalar profiles with deterministic wire metadata for
  integer, fixed-byte, bounded-byte, UTF-8, fixed-point, and boundary-float
  profiles, plus generated Rust, TypeScript, and manifest witnesses.
* Added checked Q32.32 addition, subtraction, negation, and multiplication APIs
  so geometry code can reject overflow instead of consuming saturated
  intermediates.
* Added deterministic `FixedMat2`, `FixedMat3`, and `FixedMat4` matrix
  primitives with row-major layout, checked multiplication, `determinant()`, and
  `try_inverse()` APIs.
* Added deterministic `FixedAffine2` and `FixedAffine3` affine transform
  primitives with `checked_transform_point()`, `checked_transform_vector()`,
  `checked_mul_affine()`, and `try_inverse()` APIs.
* Added deterministic seeded property-test corpora for Q32.32 raw/order
  invariants, vector algebra identities, query symmetry and bounds, and mesh
  quantization round trips.

### Changed

* Bumped the publishable Bunny workspace crates to `0.6.0`; regenerated contract
  witnesses now identify the released generator as `bunny-wesley/0.6.0`.
* Reworked `docs/MATH_GEOMETRY_CAPABILITY_MAP.md` into a prose-led planning
  reference with compact tables for navigation and issue anchors.
* Added explicit release cut policy and cut gates to `ROADMAP.md`, including
  must-ship, may-slip, and not-included boundaries for the planned release
  train, and refreshed `docs/BEARING.md` through the `v0.6.0` release cut and
  next `v0.7.0` sequence.
* Ray intersection queries now return `None` when checked intermediate
  arithmetic overflows, rather than accepting saturated Q32.32 values as valid
  geometric hits.
* `scripts/publish-crates.sh publish` now refuses dirty worktrees
  unconditionally; `ALLOW_DIRTY=1` is limited to local `verify` and `dry-run`
  diagnostics.
* Removed global `clippy::module_name_repetitions` and
  `clippy::must_use_candidate` allowances after auditing that the workspace
  passes without them.

### Fixed

* `bunny-wesley` now rejects schema type names reserved for generated helper
  types instead of emitting Rust duplicate type items or TypeScript interface
  merges.
* `bunny-wesley` now rejects field-level `@bunnyScalarProfile` directives until
  field override semantics are implemented, instead of silently omitting them
  from generated witnesses.
* Repo-respect fixture repositories now disable commit signing during tests so
  the gate does not depend on the contributor's local GPG configuration.
* Repo-respect receipt coverage now includes deleted and typechanged paths,
  validates staged receipt contents from the Git index, rejects placeholder-only
  receipt sections, and enforces receipt trailers across non-merge branch
  commits in the full gate; the repo-respect topic records the evidence as
  `RR-TP-003` through `RR-TP-009`.
* Repo-respect branch commit validation now checks only commits on the PR side
  of the merge base, uses shared sanitized Git subprocess helpers, and rejects
  receipt sections whose only apparent content is prose field names or HTML
  comments; the repo-respect topic records the evidence as `RR-TP-004`,
  `RR-TP-005`, `RR-TP-010`, and `RR-TP-011`.
* `FixedMat2::try_inverse` now divides off-diagonal entries before negating
  them, so minimum raw values can still invert when the divided inverse entry is
  representable.
* `FixedMat3::try_inverse` and `FixedMat4::try_inverse` now divide negative
  cofactors before negating them, so minimum raw cofactors can still invert when
  the divided inverse entry is representable.
* Affine inverse computation now applies the inverse linear transform before
  negating translation, so minimum raw translations can still invert when the
  scaled inverse translation is representable.
* Historical goalpost evidence now distinguishes retired `ci.yml` anchors from
  the current Code Dojo workflow.

## [0.5.0] - 2026-06-19

### Added

* Rust Code Standards Editor's Edition docs, Numeric Constitution, Sensei's
  Wisdom, Code Dojo docs, repo-local Git hooks, and repo-respect receipt
  scaffolding.
* Code Dojo local and CI enforcement scripts for source-shape policy,
  deterministic test receipts, formatting, Clippy, tests, and WASM checks.
* Deterministic receipt integration tests and crate README/package metadata
  needed by the new standards gate.
* Validating Q32.32 float ingress through `FixedQ32_32::try_from_f32`,
  `TryFrom<f32>`, and the raw `fixed_q32_32::try_from_f32` helper.
* Math and geometry capability map covering Bunny's owned surface, non-goals,
  missing deterministic math layers, SIMD policy, and recommended build order.
* Planned post-v0.4.0 roadmap release train with GitHub milestones, goalpost
  trackers, and slice issue references.

### Changed

* `FixedQ32_32` now keeps its raw field private, derives `Hash`, exposes
  `raw()` as the canonical raw accessor, and keeps `to_raw()` as a compatibility
  alias.
* Float-to-fixed geometry ingress now rejects finite values outside the Q32.32
  range instead of allowing them to saturate silently.
* Replaced the old quality CI workflow with `.github/workflows/code-dojo.yml`.
* Root and crate manifests now inherit the workspace lint baseline from the new
  standards pack.
* Rust policy checks now run through the `xtask` AST-backed Code Dojo checker
  plus strict package-scoped Clippy enforcement instead of regex source scans.
* Code Dojo now scans tracked and untracked nonignored Rust files in full-gate
  mode and no longer exposes rollout skip knobs for Cargo, deterministic
  receipts, or WASM checks.
* `ROADMAP.md` now reserves `v0.5.0` for this standards and numeric-law release,
  and shifts the planned math and geometry feature train to `v0.6.0` onward.
* `docs/BEARING.md` now avoids branch and pull-request tracking, leaving live
  work state to GitHub while keeping durable release posture in the repo.
* Front-door signposts now describe the current workspace crate surface,
  generator command, release posture, and future math train without claiming
  shipped crates or volatile PR state as planned work.
* Dependency policy is now part of Code Dojo through `cargo deny check`.
  Duplicate transitive-version findings from the current `wesley-core` graph
  remain visible cargo-deny warnings rather than hidden skip exemptions.
* Refactored broadphase, codec, numeric, query, mesh, and generated contract
  code until the full Code Dojo and headless WASM gates pass.

### Fixed

* crates.io publication recovery now tolerates registry propagation delays and
  rate-limit responses while publishing workspace crates in dependency order.
* Generated contract witnesses now report the `bunny-wesley/0.5.0` generator
  identifier used by this release.

### Removed

* Removed the local issue generator in favor of GitHub as the backlog source of
  truth.

## [0.4.0] - 2026-06-16

### Added

* 16-bit integer coordinate quantization mapping for 3D vertices relative to an AABB boundary (`bunny-mesh`).
* Memory-stable, zero-allocation index buffer layouts with validation rules for 16-bit and 32-bit triangle indices (`bunny-mesh`).
* Cryptographic SHA-256 content-addressable hashing for mesh vertex and index buffers (`bunny-mesh`).
* Zero-copy binary PLY and OBJ text parser contracts in `bunny-codec`, with
  borrowed mesh views and Stanford Bunny fixture regressions.
* Zero-allocation OBJ vertex and triangle iterators in `bunny-codec` for
  forward full-mesh traversal without quadratic indexed-access scans.
* Captured GP2 witness table with repo-truth anchors for each completed file
  format adapter claim.
* GP3 Bunny-native compressed mesh decoder in `bunny-codec`, including
  `decode_compressed_mesh`, a zero-allocation borrowed `CompressedMesh` view,
  `CompressedIndexWidth`, typed compressed triangle variants, and explicit
  malformed-input errors.
* Canonical compressed triangle fixture bytes plus width-16, width-32,
  malformed-input, native allocation, and WASM-compatible decoder regressions.
* crates.io release automation that verifies publishable package archives and
  publishes all public Bunny crates in dependency order when a GitHub Release is
  published.

### Changed

* Raised the workspace Rust toolchain contract and CI pin to Rust 1.96.0.
* `Aabb3` to `FixedAabb3` conversion now rejects non-finite coordinates and
  inverted float bounds before fixed-point quantization.
* Mesh hashes now include quantization bounds so identical quantized buffers under different AABBs produce distinct verification IDs.
* `Ray3` to `FixedRay3` conversion now rejects non-finite origin and direction
  coordinates before fixed-point canonicalization.
* `Sphere3` to `FixedSphere3` conversion now rejects non-finite centers plus
  non-finite and negative radii before fixed-point quantization.
* BVH traversal now returns explicit errors for malformed node and primitive
  index buffers instead of panicking.
* `bunny-codec` now rejects non-finite OBJ/PLY vertex coordinates and
  out-of-bounds binary PLY face indices before returning borrowed mesh views.
* `bunny-codec` now rejects duplicate or late PLY `format` directives and
  classifies non-triangle PLY polygon payloads before generic trailing-data
  checks.
* `bunny-codec` now handles extreme OBJ float exponents without panicking and
  preserves finite OBJ coordinates with very large decimal mantissas.
* The GP3 compressed profile records its allocation contract explicitly: accepted
  decoding borrows raw payload byte sections and exposes typed checked accessors,
  rather than unsafely reinterpreting arbitrary bytes as typed slices.
* The GP3 compressed decoder now classifies oversized declared payload lengths
  against the canonical profile length before host pointer-width-dependent slice
  checks.
* The workspace crate version now matches the `v0.4.0` release tag, and
  internal crate dependencies carry crates.io version requirements for publish.

## [0.3.0] - 2026-06-13

### Added

* Memory-stable, zero-allocation flat array-backed bounding volume hierarchy (BVH) tree builder.
* SAH (Surface Area Heuristic) split selection solver for optimal tree building.
* Stack-based, deterministic tree traversal solvers for AABB overlap and ray intersection queries.
* Multi-axis Sweep-and-Prune broadphase collision solver with dynamic optimal axis selection.
* Zero-allocation Sweep-and-Prune active overlap pair generator with stable lexicographical index sorting.

## [0.2.0] - 2026-06-12

### Added

* `FixedRay3`, `FixedAabb3`, and `FixedSphere3` bounding volumes defined using fixed-point coordinate structures inside `bunny-geom`.
* Compile-time normalized vector wrappers `FixedUnitVec2` and `FixedUnitVec3` to enforce normalization invariants.
* Ray-Sphere, Ray-AABB, and Ray-Triangle (Möller-Trumbore) deterministic query intersection solvers in `bunny-query`.
* Point-to-Triangle, Segment-to-Segment, and AABB-to-Sphere closest point and minimum distance query solvers in `bunny-query`.

## [0.1.1] - 2026-06-12

### Added

* AST parser mappings to dynamically resolve custom `@bunnyScalarProfile` schema directives in `bunny-wesley` instead of hardcoded strings.
* Saturation-checked mathematical division `FixedQ32_32::checked_div` returning `Option<FixedQ32_32>` for mathematical division guards.
* Q32.32 vector boundary-condition and coordinate saturation verification suites.
* Headless WebAssembly unit testing suite executing inside Node.js using `wasm-pack test`.

## [0.1.0] - 2026-06-12

### Added

* Type-safe `FixedQ32_32` newtype wrapper for deterministic fixed-point math profiles.
* Addition, subtraction, multiplication, division, negation, and assignment operators for `FixedQ32_32` with Ties-to-Even rounding and saturation.
* Deterministic integer square root `FixedQ32_32::sqrt` via a software-defined digit-by-digit binary algorithm.
* Type-safe deterministic fixed-point vectors `FixedVec2` and `FixedVec3` inside `bunny-linalg` with arithmetic operators and dot/cross product utilities.
* Double-width boundary mapping `From`/`Into` conversions between DTO float vectors (`Vec2`, `Vec3`) and fixed-point vectors.
* Strict Rust development rules via `CODE_STANDARDS.md` and crate linter denials.
* Cross-platform GitHub Actions CI suite verifying Ubuntu, macOS, and Windows determinism, plus WebAssembly checks.
* Independent `README.md` layouts for all 5 workspace crates.
* Long-term developmental `ROADMAP.md` mapping versions, goalposts, and slices.
* Repository signposts `VISION.md`, `BEARING.md`, `PROCESS.md`, and `TESTING.md` defining standard workflows and testing rules.
