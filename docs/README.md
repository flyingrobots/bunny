# Bunny Documentation

This directory is Bunny's living technical book.

The repository keeps different kinds of documents because they answer different
questions. Current behavior, historical rationale, test design, roadmap state,
and release chronology must not compete with each other.

## Current Truth

Living topic chapters describe how Bunny works now. They are updated in place
when code, public behavior, invariants, or supported usage changes. They do
not describe intended behavior before the implementation and executable
evidence exist.

| Chapter | Owns |
| --- | --- |
| [`topics/angle-trigonometry/`](topics/angle-trigonometry/) | Binary angles (`FixedAngle`), conversions from and to turns, degrees and radians (unsigned and signed), table-based deterministic sine and cosine, `atan2`, `atan`, `asin` and `acos`, with error bounds. |
| [`topics/ascii-ply/`](topics/ascii-ply/) | The ASCII PLY triangle-mesh profiles `bunny-ascii-ply-triangle-mesh/1` read by `parse_ascii_ply` and `/2` read by `parse_ascii_ply_v2`, which adds `s`/`t` or `u`/`v` texture coordinates, their line, whitespace and comment rules and error contract, the caller-chosen limits of `parse_ascii_ply_with_limits`, and the exact integer-only decimal to Q32.32 conversion rule `bunny-decimal-q32-32/1`. |
| [`topics/bounding-volumes/`](topics/bounding-volumes/) | Box helpers on `FixedAabb3` (`bunny-bounds/1`): bounds from points, a triangle or a sphere, merge, expand by a margin with typed refusals for inverted boxes and the raw range, the bounding sphere with its ties-to-even centre and outward-rounded radius, and `mesh_bounds` over a mesh's indexed vertices. |
| [`topics/bvh-construction/`](topics/bvh-construction/) | The flat BVH node format and its two builders: the quartile builder `build_bvh` and the binned SAH builder `build_bvh_sah`, with costs, overflow rule, tie rules, depth cap and stable partition; and `refit_bvh`, which gives a built tree moved primitives' boxes with its topology unchanged. |
| [`topics/code-dojo-determinism/`](topics/code-dojo-determinism/) | The Code Dojo rules that enforce Echo's `ban-nondeterminism` rule IDs: banned paths and crates, import and macro resolution, crate scope, waivers, and the dependency check. |
| [`topics/contract-generation/`](topics/contract-generation/) | Schema-to-artifact generation, generated Rust/TypeScript DTO shape, manifest witnesses, and Bunny-owned object boundaries. |
| [`topics/coordinate-law/`](topics/coordinate-law/) | Coordinate spaces, handedness, units, winding, transform naming, and projection and NDC conventions. |
| [`topics/curve/`](topics/curve/) | Cubic Bezier curves and Catmull-Rom splines (`bunny-curve/1`): `CubicBezier3` evaluation by de Casteljau over `bunny-lerp/1` in a stated order with its 3/2 raw unit bound, the derivative from checked differences with its 3 raw unit bound and overflow rule, the split into de Casteljau halves and what it does not preserve, and `None` for `t` outside `[0, 1]`. Part 2: `CatmullRom3`, uniform Catmull-Rom splines through borrowed points, open (duplicated end points) or closed, converted to Bezier segments with inner control points rounded once, ties to even; typed refusals, the parameter split, the 15/8 and 9/2 raw unit bounds, exact passage through every point and exact tangent continuity at joints. Conservative bounds: the exact control box (no widening needed for the rounded evaluation), split boxes widened by a derived 3 raw units, and the union of a spline's segment boxes. |
| [`topics/deterministic-contract-profile/`](topics/deterministic-contract-profile/) | Bunny-owned scalar profile vocabulary, generated witnesses, and wire-profile boundaries for future codecs. |
| [`topics/ease/`](topics/ease/) | Easing curves (`bunny-ease/1`), part 1: the power eases `power_in`, `power_out` and `power_in_out` for `n` in `2..=5`, each its exact polynomial rounded once, with exact ends, monotonicity and bit-exact reflections, and `steps`; part 2: GSAP's back (any overshoot) and bounce rounded once, and circ as the floor of an exact 128-bit root, within one raw unit; part 3: the sine eases `sine_in`, `sine_out` and `sine_in_out`, read from `sin_cos` within one raw unit. |
| [`topics/fixed-point-rounding/`](topics/fixed-point-rounding/) | Where `bunny-num` rounds a Q32.32 result (multiplication, division, the prepared divisor, float ingress and egress), the ties-to-even rule at each site and its exact-value tie tests. |
| [`topics/frustum/`](topics/frustum/) | `FixedFrustum`: six planes facing into the frustum with a closed inside, held as given; the classification of points, boxes and spheres into `Inside`, `Intersecting` and `Outside` with a certain `Outside` that wins over overflow, the conservative `Intersecting` and its false positives, the exact wide-integer sphere comparison, and the exact row-sum extraction from a clip matrix for the `[0, 1]` depth range with its 3/2 raw unit bound. |
| [`topics/gpu-u32-div-round-even/`](topics/gpu-u32-div-round-even/) | Unsigned 32-bit ties-to-even division (`bunny-u32-div-round-even/1`): `bunny-num` CPU reference, canonical `[value, status]` wire format, portable WGSL helper, ordered vector fixture, manifest, and `bunny-gpu` `wgpu` equivalence harness. |
| [`topics/gpu-u32-ellipse-classify/`](topics/gpu-u32-ellipse-classify/) | Exact unsigned 64-bit ellipse trichotomy (`bunny-u32-ellipse-classify/1`): `bunny-num` CPU reference, explicit zero-semiaxis contract, canonical `[class, status]` wire format, portable 2-limb `u32` WGSL helper, ordered vector fixture, manifest, and `bunny-gpu` `wgpu` equivalence harness. |
| [`topics/lerp/`](topics/lerp/) | Exact interpolation and ratios (`bunny-lerp/1`): `FixedQ32_32::lerp` with its single rounding of the step, exact ends, monotonicity and extrapolation, `FixedQ32_32::from_ratio`, and the component-wise `FixedVec3::lerp` and `lerp_channels`. `FixedQ32_32::inverse_lerp` and `remap`, each one rounding of an exact wide quotient, with refused degenerate ranges, exact ends and the two round-trip bounds; the exact `checked_clamp`, which refuses an inverted range, and `Ord::min` and `max`. |
| [`topics/lint-policy/`](topics/lint-policy/) | Rule 10 for the core crates: `clippy::nursery` cherry-picked by name, never set as a group, and the Code Dojo `blanket-nursery` check that enforces it. |
| [`topics/look-rotation/`](topics/look-rotation/) | `FixedMat3::try_look_rotation` (`bunny-look-rotation/1`): the local `-Z` to forward convention, the shared look-at basis and its rounding, the column and row orthonormality bounds, and the refusals. |
| [`topics/matrix-types/`](topics/matrix-types/) | Fixed matrix layout, affine point/vector transforms, determinants, inverses, and transform boundaries. |
| [`topics/mesh-topology/`](topics/mesh-topology/) | Topology validation of quantized triangle meshes (`bunny-mesh-topology/1`): index-degenerate and exactly decided geometrically degenerate triangles, exact and flipped duplicate faces, boundary, winding-inconsistent and non-manifold edges, their severities and total record order, the caller-provided scratch that keeps the kernel allocation-free, and the refusals. |
| [`topics/no-std/`](topics/no-std/) | The default `std` feature of `bunny-num`, `bunny-linalg`, `bunny-geom` and `bunny-query`, what their `no_std` builds (default features off) provide, the audit of every `std` use the last three had, and the `code-dojo-no-std` gate that builds them for `thumbv7em-none-eabihf`, `wasm32-unknown-unknown` and the host. |
| [`topics/optics/`](topics/optics/) | The pinhole, thin-lens and orthographic cameras and the projection round trip through the view and projection matrices; the pinhole camera: inputs and refusals, the right-handed camera frame, the image convention (top-left pixel origin, as in viewport conversion), ray generation, the direction scaling rule, its accuracy bound, and what trace distances mean for unnormalized directions; pixel values: exact accumulation and its bit budget, the mean's rounding, the sRGB and linear tone curves with their error bound, 8-bit and 16-bit codes and their versioned identities; diffuse shading: triangle normals and frames, the facing and self-intersection rules, the cosine-weighted hemisphere map and its bounds, the radiometric convention, the directional light and its shadow-ray limit, the sky, and the order of operations of one path. |
| [`topics/performance/`](topics/performance/) | The fidelity rule for optimizations, the `bunny-perf` harness, `cargo xtask perf` before/after comparisons, CI regression gating, and the ledger of landed speedups. |
| [`topics/plane-clipping/`](topics/plane-clipping/) | Fixed plane classification of points, segments, triangles and boxes, the on-plane rule, segment and convex polygon clipping with its rounding contract. |
| [`topics/predicates/`](topics/predicates/) | Exact geometric predicates (`bunny-predicates/1`): `orientation2` and `orientation3`, the exact sign of the 2D and 3D orientation determinants for every representable input with the canonical winding and right-hand conventions; the exact doubled area and sextupled volume with their widths, and the forms rounded once to Q32.32 that round a value of at most half a raw unit to zero; exact barycentric numerators, the exact location classification with its precedence, closed and open containment, and the rounded weights with their one-raw-unit sum bound; the relation to the rounded plane classification. |
| [`topics/quaternion/`](topics/quaternion/) | `FixedUnitQuat` (`bunny-quaternion/1`): the `(x, y, z, w)` storage order, the length band within one raw unit of one, the one exact normalisation and its only refusal, why no rounding meets an exact half, the Hamilton product normalised in one rounding and composed right to left, drift over chains, `q v q^-1` rounded once per component, the rotation matrix and its bounds against `bunny-rotation/1`, axis-angle input, and the sign operations. |
| [`topics/ray-tracing/`](topics/ray-tracing/) | Ray tracing: the kernels (trace domain, conservative division-free ray-box bound, exact integer ray-triangle test, edge rule, facing, rounding of distances and barycentrics) and the closest-hit and any-hit BVH traversals, with the argument that their answer does not depend on the tree. |
| [`topics/release-packaging/`](topics/release-packaging/) | The `scripts/publish-crates.sh` archive checks, including building every crate's tests from its own unpacked package, and the rule that a test and the files it reads ship together. |
| [`topics/repo-respect/`](topics/repo-respect/) | Receipt files, commit trailers, staged and branch gate behavior, and Git index isolation for auditability. |
| [`topics/rotation/`](topics/rotation/) | `FixedMat3::rotation_x`, `rotation_y`, `rotation_z` and `rotation_axis_angle` (`bunny-rotation/1`): the right-hand sign convention, the exact cardinal and unit-axis cases, the single ties-to-even rounding of Rodrigues' formula, and the entry, orthonormality and determinant bounds in raw units. |
| [`topics/shape-frame/`](topics/shape-frame/) | `ShapeFrame` (`bunny-shape-frame/1`): the rigid frame an analytic shape is traced in, its axis and domain refusals, rays mapped into object space and normals back with one ties-to-even rounding per component. |
| [`topics/shape-library/`](topics/shape-library/) | The geometry-level shape types of `bunny-geom`: line, segment, plane, triangle, capsule and oriented box, with each one's representation, invariants, accepted and refused degenerate cases, the exact triangle degeneracy test, the exact oriented-box axis tolerances taken from the rotation contracts, the float boundary constructors and their refusal order, and the relation to the trace-domain types of `bunny-query`. |
| [`topics/transform/`](topics/transform/) | `FixedAffine3::from_trs`, `rotate_about_point` and `scale_about_point` (`bunny-transform/1`): the scale, rotate, translate order with a worked example, the single ties-to-even rounding of each product, the exactly fixed pivot, the overflow thresholds in raw units, and the bound on both groupings of a composed product; ray, axis-aligned box and inverse-transpose normal transforms, and what a non-uniform scale does to normals and bounds. |
| [`topics/uniform-grid/`](topics/uniform-grid/) | The uniform grid broadphase `uniform_grid_pairs` (`bunny-uniform-grid/1`): positive cell sizes from one raw unit, cells as exact floor divisions of raw coordinates (half-open, boundaries in the upper cell, the whole raw range in `i64`), closed boxes occupying the cells of their min to their max, the exact key count with its overflow refusal, the refusals and their order, pairs exactly the overlapping ones in `sweep_and_prune`'s order, and the caller-owned storage contract with its short-buffer report. |
| [`NUMERIC_CONSTITUTION.md`](NUMERIC_CONSTITUTION.md) | Q32.32 arithmetic law, construction policy, rounding, overflow, and numeric golden-vector expectations. |
| [`MATH_GEOMETRY_CAPABILITY_MAP.md`](MATH_GEOMETRY_CAPABILITY_MAP.md) | Bunny's durable math/geometry ownership boundary, non-goals, missing surface, and build order. |
| [`CODE_DOJO.md`](CODE_DOJO.md) | Local and CI quality gate behavior. |
| [`TESTING.md`](TESTING.md) | Repository-level test and validation commands. |
| [`PROCESS.md`](PROCESS.md) | Goalpost delivery loop, review discipline, documentation source-of-truth rules, and release publication flow. |

## Topic Folder Standard

Durable concepts that will be changed by more than one pull request should live
under `docs/topics/<topic>/`.

Recommended files:

| File | Required | Role |
| --- | --- | --- |
| `README.md` | Yes | Current behavior and invariants for the topic. |
| `test-plan.md` | Yes | Verification design: requirements, fixtures, cases, oracles, determinism obligations and evidence, and gaps. |
| `architecture.md` | Optional | Structure, data flow, and module boundaries when the topic is large enough to need them. |
| `rationale.md` | Optional | Durable tradeoffs and rejected approaches that still help maintainers. |

The topic `README.md` is the chapter. It answers "what is true now?" Historical
design documents, issues, pull requests, and goalpost notes may explain why that
truth changed, but they are not competing current references.

## Historical Records

| Location | Role |
| --- | --- |
| [`design/`](design/) | Historical design decisions and proposal-era notes. |
| [`goalposts/`](goalposts/) | Delivery evidence for completed goalposts. |
| [`../ROADMAP.md`](../ROADMAP.md) | Versioned delivery plan and GitHub issue anchors. |
| [`../CHANGELOG.md`](../CHANGELOG.md) | Release chronology. |

Historical records may be corrected when factually wrong. When a historical
document could mislead readers about current behavior, add a short note pointing
to the relevant living topic chapter instead of rewriting the whole record.

## Design And Test Gate

Before implementing a nontrivial behavior change:

1. Write or update a proposal, RFC, or rationale note when the change needs
   real design discussion.
2. Add or update planned cases in `test-plan.md` before implementation.
3. Write the smallest failing executable evidence for the planned case.
4. Implement the behavior.
5. Update the living topic chapter so it describes the behavior now present in
   `HEAD`.
6. Mark the planned cases as implemented evidence and record the actual test
   names, fixtures, or artifact anchors.
7. Record release-visible changes in `CHANGELOG.md`.

For small bug fixes, the same discipline can be scaled down, but every behavior
change still needs a test or a written reason why the existing tests already
cover it.

## Test Plan Standard

Topic test plans are both prose and a small contract graph. The prose explains
intent for humans. Fenced `toml` metadata blocks define stable requirement IDs,
case IDs, explicit oracles, evidence types, status, and test or artifact
anchors for `cargo run --locked -p xtask -- topic-docs`.

Topic test plans should cover:

- golden paths
- known failures
- edge cases
- unusual inputs
- stable error kinds
- determinism obligations and evidence
- invariants and property tests
- metamorphic or differential checks when no simple oracle exists
- stress, fuzz, and replay strategy
- fixture provenance and regeneration
- public-surface test goals
- open verification gaps

Tests should assert public behavior and stable contract artifacts. They should
not assert private implementation details, scrape human-oriented output, or use
documentation prose as the oracle.

Valid assertions include structured return values, state transitions, error
kinds, raw fixed-point values, canonical bytes, stable hashes, generated DTOs,
and explicitly documented artifacts. Incidental stdout, stderr, log text, and
private helper behavior are not stable test contracts unless an API explicitly
makes them so.
