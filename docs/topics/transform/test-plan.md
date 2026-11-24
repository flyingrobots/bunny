# Transform Composition Test Plan

This document defines how Bunny proves [`bunny-transform/1`](README.md):
`FixedAffine3::from_trs`, `FixedAffine3::rotate_about_point` and
`FixedAffine3::scale_about_point`, and the composition bound of `checked_mul_affine`. It
answers flyingrobots/bunny#312. It also proves the ray, box and normal transforms of
flyingrobots/bunny#108: `FixedRay3::checked_transform` and `FixedAabb3::checked_transform` in
`bunny-geom`, and `FixedAffine3::checked_transform_normal` in `bunny-linalg`. It proves the
transform-aware bounds of flyingrobots/bunny#119: `FixedAabb3::transformed_bounds`,
`from_transformed_points`, `from_transformed_triangle` and `from_transformed_sphere` in
`bunny-geom`.

## Scope

- The three constructors in `bunny-linalg`: the scale, rotate, translate order, the pivot
  formulas, the single rounding of each product, the bounds in raw units, the overflow
  thresholds, and the acceptance of zero and negative scales.
- The bound on the two groupings of a product of three affine transforms.
- The ray image (origin and direction, not renormalised) and its parameter bound; the box hull
  of the eight rounded corners; the inverse-transpose normal; each refusal and threshold.
- The transform-aware bounds: the smallest raw box containing the exact image of a box, a point
  set, a triangle or a sphere, its outward rounding, and its refusals and thresholds.

Out of scope: decomposing a transform, a bound on `try_inverse` against the exact inverse, a
box that contains the rounded point transform of every point of the input, oriented boxes and
capsules, and a bounding sphere of a transformed sphere.

## Requirements

The table is the human-readable view. The fenced TOML block immediately after it is the
contract graph consumed by `cargo run --locked -p xtask -- topic-docs`.

| ID | Requirement | Current Source |
| --- | --- | --- |
| TRS-REQ-001 | `from_trs(t, R, s)` is `T * R * S` with column vectors and right-to-left composition: a point is scaled, then rotated, then translated; the linear part is `R * diag(s)` and the translation is `t`, copied. | `README.md#convention` |
| TRS-REQ-002 | Each `from_trs` linear entry is `R_ij * s_j` rounded once to nearest, ties to even, within 1/2 raw of the exact product; the result is `None` exactly when a rounded entry is outside the `i64` raw range; zero and negative scales are accepted. | `README.md#rounding-and-bounds` |
| TRS-REQ-003 | `rotate_about_point(p, R)` is `T(p) * R * T(-p)`: linear part `R`, translation `p - R p` with `R p` from `checked_mul_vec3`, within 3/2 raw of the exact value; `checked_transform_point(p)` is exactly `p`; `None` exactly when a rounded product, a partial sum or the subtraction leaves the range. | `README.md#rounding-and-bounds` |
| TRS-REQ-004 | `scale_about_point(p, s)` is `T(p) * S * T(-p)`: linear part `diag(s)`, translation `p_i - s_i p_i` rounded once, within 1/2 raw; the pivot maps onto itself exactly; `None` exactly on overflow; zero and negative scales are accepted. | `README.md#refusals` |
| TRS-REQ-005 | For three affine transforms, each entry of `(A * B) * C` and of `A * (B * C)` by `checked_mul_affine` is within the documented bound of the exact product `A B C`. | `README.md#composition` |
| TRS-REQ-010 | `FixedRay3::checked_transform(&a)` gives a `TransformedRay3` whose origin is `a.checked_transform_point(origin)` and whose direction is `a.checked_transform_vector` of the stored unit direction, not renormalised: each component within 3/2 raw of the exact value, and `origin' + t direction'` within `3/2 (1 + abs(t))` raw of the exact image of `origin + t direction` for every real `t`. | `README.md#rays` |
| TRS-REQ-011 | The ray transform is `Err(OutOfRange)` exactly when the point or the vector transform is `None` (the origin is checked first) and `Err(DegenerateDirection)` exactly when the rounded direction is the zero vector; `TransformedRay3::try_into_ray` renormalises with `FixedRay3::try_new`, after which `t` no longer carries over. | `README.md#rays` |
| TRS-REQ-012 | `FixedAabb3::checked_transform(&a)` is `Err(InvertedBox)` exactly when a minimum is above its maximum, before any arithmetic; otherwise the per-axis minimum and maximum of the eight corners mapped by `checked_transform_point`, so it contains every rounded corner and each face is within 3/2 raw of the exact hull's; a point box gives the point transform; `Err(OutOfRange)` exactly when a corner's transform is `None`. | `README.md#boxes` |
| TRS-REQ-013 | `FixedAffine3::checked_transform_normal(n)` is `linear.try_inverse()` transposed times `n` by `checked_mul_vec3`: within 3/2 raw of the exact product of the rounded inverse and `n`, the translation ignored, `None` exactly when `try_inverse` is `None` or a rounded product or partial sum leaves the range; where the inverse and every product are exact, `n' . (L t)` equals `n . t` exactly. | `README.md#normals` |
| TRS-REQ-014 | The chapter states what a non-uniform scale does to normals (the inverse transpose, then renormalisation) and to bounds (the axis-aligned box of a transformed box is a hull larger than the image; a bounding sphere does not stay a sphere), with worked examples. | `README.md#non-uniform-scale` |
| TRS-REQ-020 | `FixedAabb3::transformed_bounds(&a)` is `Err(InvertedBox)` exactly when a minimum is above its maximum, before any arithmetic; otherwise on each axis the floor of the exact least and the ceiling of the exact greatest image coordinate of the box (the smallest raw box containing the exact image), computed with no intermediate range check, and `Err(OutOfRange)` exactly when such a face is outside `i64`. | `README.md#transform-aware-bounds` |
| TRS-REQ-021 | `FixedAabb3::from_transformed_points(points, &a)` is `Err(NoPoints)` for an empty slice and otherwise the floor of the least and the ceiling of the greatest exact image coordinate over the points on each axis, `Err(OutOfRange)` exactly when such a face is outside `i64`; `from_transformed_triangle` is that of its three vertices, degenerate triangles included. | `README.md#transform-aware-bounds` |
| TRS-REQ-022 | `FixedAabb3::from_transformed_sphere(&s, &a)` is `Err(NegativeRadius)` for a negative radius, first; otherwise on each axis the floor and the ceiling of the exact extremes `c'_i -+ r abs(row_i)` of the image ellipsoid, decided exactly, and `Err(OutOfRange)` exactly when such a face is outside `i64`. | `README.md#transform-aware-bounds` |
| TRS-REQ-023 | The chapter states the transform-aware bounds' rounding (outward, under 1 raw per face, none when the face is a whole raw value), their overflow rule, the widths of the intermediates, and how they relate to the eight-corner hull and to the rounded point transform. | `README.md#transform-aware-bounds` |

```toml
[[requirement]]
id = "TRS-REQ-001"
summary = "from_trs is T * R * S: scale, then rotate, then translate; linear part R * diag(s), translation copied."
status = "active"

[[requirement]]
id = "TRS-REQ-002"
summary = "Each from_trs entry is R_ij * s_j rounded once, ties to even, within 1/2 raw; None exactly when an entry leaves the i64 raw range; zero and negative scales accepted."
status = "active"

[[requirement]]
id = "TRS-REQ-003"
summary = "rotate_about_point is T(p) R T(-p): translation p - R p within 3/2 raw, the pivot fixed exactly, None exactly on overflow."
status = "active"

[[requirement]]
id = "TRS-REQ-004"
summary = "scale_about_point is T(p) S T(-p): translation p_i - s_i p_i rounded once within 1/2 raw, the pivot fixed exactly, None exactly on overflow; zero and negative scales accepted."
status = "active"

[[requirement]]
id = "TRS-REQ-005"
summary = "Both groupings of a product of three affine transforms are within the documented bound of the exact product."
status = "active"

[[requirement]]
id = "TRS-REQ-010"
summary = "The ray image is the point transform of the origin and the vector transform of the direction, not renormalised; origin' + t direction' is within 3/2 (1 + |t|) raw of the exact image."
status = "active"

[[requirement]]
id = "TRS-REQ-011"
summary = "Ray transform refusals: OutOfRange exactly when a point or vector transform is None, origin first; DegenerateDirection exactly when the rounded direction is zero; try_into_ray renormalises."
status = "active"

[[requirement]]
id = "TRS-REQ-012"
summary = "The box transform refuses an inverted box first, is the per-axis hull of the eight rounded corners, each face within 3/2 raw of the exact hull, and OutOfRange exactly when a corner overflows."
status = "active"

[[requirement]]
id = "TRS-REQ-013"
summary = "checked_transform_normal is the rounded inverse transposed times n, within 3/2 raw of that exact product, None exactly when try_inverse is None or the product overflows."
status = "active"

[[requirement]]
id = "TRS-REQ-014"
summary = "The chapter documents how non-uniform scale affects normals and bounds, with worked examples."
status = "active"

[[requirement]]
id = "TRS-REQ-020"
summary = "transformed_bounds refuses an inverted box first, is the floor and ceiling of the exact image hull of the box with no intermediate range check, OutOfRange exactly when a face leaves i64."
status = "active"

[[requirement]]
id = "TRS-REQ-021"
summary = "from_transformed_points refuses an empty slice and is the floor and ceiling of the exact image hull of the points; from_transformed_triangle is that of its vertices."
status = "active"

[[requirement]]
id = "TRS-REQ-022"
summary = "from_transformed_sphere refuses a negative radius first and is the floor and ceiling of the exact image ellipsoid's extremes, decided exactly; OutOfRange exactly when a face leaves i64."
status = "active"

[[requirement]]
id = "TRS-REQ-023"
summary = "The chapter documents the transform-aware bounds' outward rounding, overflow rule and widths, and how they relate to the eight-corner hull."
status = "active"
```

## Oracles

The test file writes its own exact reference and shares no code with the kernel.
`oracle_trs` and `oracle_about` multiply raw values in `i128`, round by `div_euclid`,
`rem_euclid` and a comparison of twice the remainder with `2^32`, and check every value, partial
sum and difference against the `i64` range with `i64::try_from`. The kernel uses
`FixedQ32_32::checked_mul`, which rounds the magnitude by a shift and a mask. Bounds are
decided exactly on the unrounded `i128` values. `exact_product` forms the exact `A B C` over
`2^64` raw with checked `i128` arithmetic, which panics if a seeded input ever left the widths
it was chosen for.

A separate Python model, written from the [chapter](README.md) with unbounded integers,
applies the documented rules to the golden inputs of TRS-TP-004 and checks in `Fraction` that
each `from_trs` entry is within 1/2 raw and each pivot translation within 3/2 raw of the exact
value, and that each pivot maps onto itself. The goldens come from it; it also found the
`from_trs` thresholds of TRS-TP-006 by bisection. The model is posted on the pull request for
flyingrobots/bunny#312, not kept in the tree.

The ray, box and normal tests have their own oracles, written in their files and sharing no
code with the kernels. `oracle_apply` (in `ray_box_transform_tests.rs`) rounds each product by
`div_euclid` and twice the remainder, range-checks every product, partial sum and translation
sum with `i64::try_from`, and gives the expected bits and refusal of every point, vector and
corner. `exact` carries the exact `row . p + t` as `q * 2^32 + r` with `0 <= r < 2^32`, so the
exact hull faces of full-range boxes are compared without leaving `i128`. The ray parameter
bound is decided in units of `2^-96` in a domain where every term fits `i128`: linear entries
below `2^39` raw, origins and translations below `2^46`, `t` below `2^46` raw. The normal
oracle (in `normal_transform_tests.rs`) takes the rounded inverse from `FixedMat3::try_inverse`
as given, because the rule is stated against it, and multiplies, rounds and range-checks on its
own; the rounded inverse itself is pinned by TRS-TP-030.

A second Python model, written from the [chapter](README.md) with unbounded integers, applies
the documented rules (including `try_inverse`, rounded minors, determinant and division) to the
golden inputs of TRS-TP-029 and TRS-TP-030, and checks in `Fraction` that every ray, box and
normal component is within 3/2 raw of its exact value. The goldens come from it. It is posted on
the pull request for flyingrobots/bunny#108, not kept in the tree.

The transform-aware bounds tests (`transform_bounds_tests.rs`) use exact big-integer arithmetic,
`transform_bounds_oracle::Big` (sign and magnitude, base `2^32` limbs, schoolbook operations),
written for them and sharing nothing with the kernel. Each exact image coordinate is
`N = 2^32 t + sum_j m_j p_j`, its value times `2^32` in raw units. The box oracle takes the least
and greatest `N` over all **eight corners**, where the kernel picks each product's extreme per
axis; the point oracle over every point. A face `F` is accepted exactly when it contains
(`2^32 F <= N_min`) and is tight (`2^32 (F + 1) > N_min`), and the mirror for a maximum; a refusal
exactly when `N_min < 2^32 i64::MIN` or `N_max > 2^32 i64::MAX`. The sphere oracle takes
`X = R^2 (m_i0^2 + m_i1^2 + m_i2^2)` and accepts a low face `F` when `D = N_c - 2^32 F` is at
least zero and `D^2 >= X` and `F + 1` fails that, the mirror for a high face; it squares, where
the kernel bisects. A third Python model with unbounded integers and `Fraction` (posted on the
pull request for flyingrobots/bunny#119, not kept in the tree) gives the goldens of TRS-TP-058:
it forms the eight corners in `Fraction`, takes `floor` and `ceil`, and bounds a sphere with
`ceil(sqrt(X))` from `math.isqrt`.

## Fixtures

None. TRS-TP-005 draws 10 000 cases from seed `0xB002_0312_0000_0001` with a 64-bit LCG: a
matrix, a translation, a scale and a pivot, each raw component a uniform 64-bit pattern shifted
right by 0 to 63 bits, so every magnitude up to the range limit occurs. Of the cases, 4 682
`from_trs` results are in range; of the 20 000 pivot constructions, 11 717 are in range and
8 283 are `None`. The test pins those counts. TRS-TP-008 draws 10 000 triples from seed
`0xB002_0312_0000_0001 ^ 8`, each transform `from_trs` of a translation within 4096, the
product `rotation_x(a) * rotation_z(b)` of two seeded angles and a scale within 4 in each
component (zero and negative included). Every failure message names the seed and the case.

TRS-TP-022 draws 10 000 full-range rays from seed `0xB002_0108_0000_0001` (each raw component a
uniform 64-bit pattern shifted right by 0 to 63 bits; the direction is `FixedUnitVec3::new` of
such a vector, and the 1 095 draws it refuses are skipped): 4 115 are in range, 4 790
`OutOfRange` and none degenerate (degenerate directions are covered by TRS-TP-021). It then
draws 10 000 rays in the moderate domain of the parameter bound from seed `0xB002_0108_0000_0001
^ 2`. TRS-TP-025 draws 10 000 full-range boxes from seed `0xB002_0108_0000_0001 ^ 1`, each box
the per-axis minimum and maximum of two seeded points: 2 370 are in range. TRS-TP-028 draws
10 000 matrices and normals from seed `0xB002_0108_0000_0002`, each raw component below `2^36`
and shifted right by 28 to 63 bits: 6 662 are in range, 3 335 are `None` from `try_inverse` and 3
from the product. It then draws 10 000 exactly invertible transforms from seed
`0xB002_0108_0000_0002 ^ 1`: scales `+-2^k` units for `k` in `-8..=8`, rows permuted, and
vectors whose raw components are multiples of `2^12` below `2^39`. The tests pin these counts.

The transform-aware bounds tests draw from seed `0xB002_0119_0000_0001` with the same LCG and
full-range components. TRS-TP-052 draws 10 000 boxes (each the per-axis minimum and maximum of
two seeded points): 2 408 are in range (the eight-corner hull of TRS-TP-025, whose partial sums
are range-checked, keeps 2 370 of its own draws); it then draws 10 000 boxes from seed
`... ^ 1` with linear entries and coordinates shifted right by at least 24 bits and
translations by at least 2, all in range. TRS-TP-054 draws 10 000 sets of 1 to 6 points from
seed `... ^ 2`: 1 753 are in range. TRS-TP-057 draws 10 000 spheres (centre and non-negative
radius full range) from seed `... ^ 3`: 3 137 are in range; then 10 000 in the moderate domain
from seed `... ^ 4`, each also checked to lie inside `transformed_bounds` of `from_sphere`. The
tests pin these counts.

## Test Cases

| ID | Category | Requirements | Oracle | Test |
| --- | --- | --- | --- | --- |
| TRS-TP-001 | Hand-derived | TRS-REQ-001 | Derived on paper in the test: `from_trs((1, 0, 0), rotation_y(quarter), (2, 3, 4))` maps `(1, 0, 0)` to `(1, 0, -2)`; rotating before scaling gives `(1, 0, -4)`; the vector `(1, 0, 0)` maps to `(0, 0, -2)`; the result equals `T * (R * S)` by `checked_mul_affine`. | `crates/bunny-linalg/tests/transform_tests.rs::trs_tp_001_scale_then_rotate_then_translate` |
| TRS-TP-002 | Hand-derived | TRS-REQ-003, TRS-REQ-004 | `rotate_about_point((1, 0, 0), rotation_y(half))` maps `(2, 0, 0)` to the origin and the pivot to itself, and equals `T(p) * (R * T(-p))`; `scale_about_point((1, 2, 3), (2, -1, 0))` maps `(2, 2, 2)` to `p + s (q - p) = (3, 2, 3)` and the pivot to itself. | `crates/bunny-linalg/tests/transform_tests.rs::trs_tp_002_rotation_and_scale_about_a_point_fix_the_pivot` |
| TRS-TP-003 | Ties | TRS-REQ-002, TRS-REQ-003, TRS-REQ-004 | At 60 degrees `rotation_z` has `m00` exactly one half, so `m00 s` is `s / 2` raw: `s = 1, -1, 3, -3, 5` give `0, 0, 2, -2, 2`, and the pivot translations `s - round(s / 2)` are `1, -1, 1, -1, 3`. | `crates/bunny-linalg/tests/transform_tests.rs::trs_tp_003_products_on_a_half_round_to_even` |
| TRS-TP-004 | Pinned golden | TRS-REQ-002, TRS-REQ-003, TRS-REQ-004 | Raw linear parts, translations and mapped points for `from_trs` with `rotation_z(30 degrees)` and the 60 degree axis-angle rotation of ROT-TP-005, two `rotate_about_point` and one `scale_about_point`, from the Python model. Runs in the WebAssembly gate. | `crates/bunny-linalg/tests/transform_tests.rs::trs_tp_004_pinned_goldens` |
| TRS-TP-005 | Seeded oracle | TRS-REQ-002, TRS-REQ-003, TRS-REQ-004 | 10 000 seeded full-range cases: each constructor's `Option` and bits equal the oracle's; in-range results are within 1/2 raw (`from_trs`) and 3/2 raw (pivot translations) of the exact value, and every accepted pivot maps onto itself exactly; the in-range counts are pinned. | `crates/bunny-linalg/tests/transform_tests.rs::trs_tp_005_seeded_constructors_match_the_oracle_and_fix_the_pivot` |
| TRS-TP-006 | Boundary | TRS-REQ-002, TRS-REQ-003, TRS-REQ-004 | One raw unit either side of each threshold: `-1 * s` at `s = i64::MIN + 1` and `i64::MIN`; the longest-axis half turn (`m00 = 1 + 4` raw) at `0x7FFF_FFFE_0000_0007` and `-0x7FFF_FFFE_0000_0008`; translations at `i64::MIN` and `i64::MAX` copied; pivots `2^62 - 1`, `2^62`, `-2^62`, `-2^62 - 1` for the half turn, the scale `-1` and the scale 2. | `crates/bunny-linalg/tests/transform_tests.rs::trs_tp_006_overflow_is_none_one_raw_unit_past_each_threshold` |
| TRS-TP-007 | Exact cases | TRS-REQ-002, TRS-REQ-004 | A zero scale gives determinant exactly zero and no inverse; the scale `(-1, 1, 1)` gives determinant exactly `-1` and the mirror image; a zero scale about a pivot maps every point onto the pivot. | `crates/bunny-linalg/tests/transform_tests.rs::trs_tp_007_zero_and_negative_scales_are_accepted` |
| TRS-TP-008 | Seeded bound | TRS-REQ-005 | 10 000 seeded triples: every entry of `(A * B) * C` and `A * (B * C)` is within its documented bound of the exact `A B C`, decided exactly in `i128`. | `crates/bunny-linalg/tests/transform_tests.rs::trs_tp_008_composition_is_associative_within_the_rounding_bound` |
| TRS-TP-009 | Mutation | TRS-REQ-001, TRS-REQ-002, TRS-REQ-003, TRS-REQ-004 | Each rule broken on purpose fails at least one case above. | [Mutations](#mutations) |
| TRS-TP-020 | Hand-derived | TRS-REQ-010, TRS-REQ-011 | Scale `(2, 1, 1)` then translate `(1, 0, 0)`: the ray from `(1, 0, 0)` along `+X` maps to origin `(3, 0, 0)` and direction `(2, 0, 0)`; the original point at `t = 3`, `(4, 0, 0)`, maps to `(9, 0, 0)`, the transformed ray at the same `t`; renormalised, the direction is `+X` and `t = 3` reaches `(6, 0, 0)`. The identity and a translation keep the direction. | `crates/bunny-geom/tests/ray_box_transform_tests.rs::trs_tp_020_ray_image_keeps_its_parameter` |
| TRS-TP-021 | Boundary | TRS-REQ-011 | `m00 = i64::MAX` raw times the unit `+X` (`2^32`) is `i64::MAX`, `2^32 + 1` is `OutOfRange`, `2^32 - 1` fits; the same at `i64::MIN`; an origin reaching `i64::MAX` through the translation, one raw unit past; a zero scale collapsing `+X` is `DegenerateDirection`; `m00 = 1` raw rounds `d_x = 2^31 - 1` and the tie `2^31` to zero (degenerate) and `2^31 + 1` to 1 raw; an out-of-range origin with a zero direction is `OutOfRange`. | `crates/bunny-geom/tests/ray_box_transform_tests.rs::trs_tp_021_ray_refusals_one_raw_unit_either_side` |
| TRS-TP-022 | Seeded oracle | TRS-REQ-010, TRS-REQ-011 | 10 000 seeded full-range rays: the result and bits equal the oracle's; the in-range, out-of-range and degenerate counts are pinned. 10 000 seeded rays in a moderate domain: `origin' + t direction'` is within `3/2 (1 + abs(t))` raw of the exact image of `origin + t direction`, decided exactly in `i128`. | `crates/bunny-geom/tests/ray_box_transform_tests.rs::trs_tp_022_seeded_rays_match_the_oracle_and_the_parameter_bound` |
| TRS-TP-023 | Hand-derived | TRS-REQ-012 | An eighth turn about `+Z` (entries `c = round(2^32 / sqrt 2)` raw) maps the unit cube to the hull `(-c, 0, 0)` to `(c, 2c, 1)`, about twice the cube's volume; a negative scale with a translation swaps which corner is the minimum; the identity gives the box back and a translation moves both corners by it; a point box gives the point transform as a point box. | `crates/bunny-geom/tests/ray_box_transform_tests.rs::trs_tp_023_box_hull_of_a_rotated_cube_and_a_point` |
| TRS-TP-024 | Boundary | TRS-REQ-012 | A minimum above its maximum on each axis is `InvertedBox` (as `FixedAabb3::try_new` refuses it), an equal pair is accepted, and an inverted box is refused even under an overflowing transform; a maximum reaching `i64::MAX` and a minimum reaching `i64::MIN` through the translation, one raw unit past each; the product `2 x` at `x = 2^62 - 1` and `2^62` raw. | `crates/bunny-geom/tests/ray_box_transform_tests.rs::trs_tp_024_box_refusals_one_raw_unit_either_side` |
| TRS-TP-025 | Seeded oracle | TRS-REQ-012 | 10 000 seeded full-range boxes: `OutOfRange` exactly when an oracle corner is out of range; otherwise each face equals the minimum or maximum of the eight oracle corners and is within 3/2 raw of the exact hull face; the in-range count is pinned. | `crates/bunny-geom/tests/ray_box_transform_tests.rs::trs_tp_025_seeded_boxes_contain_every_rounded_corner` |
| TRS-TP-026 | Hand-derived | TRS-REQ-013 | Scale `(2, 1, 1)`: the normal `(1, 1, 0)` maps to `(1/2, 1, 0)` and the tangent `(1, -1, 0)` to `(2, -1, 0)`, dot product exactly 0, while the linear image `(2, 1, 0)` gives 3; the translation is ignored; a quarter turn gives the vector transform; a uniform scale 2 gives `n / 2`; the mirror `(-1, 1, 1)` maps `+X` to `-X`. | `crates/bunny-linalg/tests/normal_transform_tests.rs::trs_tp_026_non_uniform_scale_keeps_normals_perpendicular` |
| TRS-TP-027 | Boundary | TRS-REQ-013 | A zero scale is `None`; `diag(1, 1, 1)` raw is nonsingular but its rounded determinant is zero, `None`; the inverse entry of `2` raw is `2^63` raw (`None`), of `3` raw `0x5555_5555_5555_5555`; that entry times `n_x = 1.5 - 1` raw fits, `1.5` and `1.5 + 1` raw are `None`, `-1.5` gives `i64::MIN` and `-1.5 - 1` raw is `None`. | `crates/bunny-linalg/tests/normal_transform_tests.rs::trs_tp_027_normal_refusals_one_raw_unit_either_side` |
| TRS-TP-028 | Seeded oracle | TRS-REQ-013 | 10 000 seeded matrices and normals: the result equals the oracle applied to the rounded inverse, within 3/2 raw of the exact product; the in-range and `None` counts are pinned. 10 000 seeded exactly invertible transforms (power-of-two scales, signed permutations): `n' . (M t)` equals `n . t` exactly. | `crates/bunny-linalg/tests/normal_transform_tests.rs::trs_tp_028_seeded_normals_match_the_oracle` |
| TRS-TP-029 | Pinned golden | TRS-REQ-010, TRS-REQ-012 | Raw ray image and box hull for one sheared, non-uniformly scaled transform with rounding in every product, from the Python model. Runs in the WebAssembly gate. | `crates/bunny-geom/tests/ray_box_transform_tests.rs::trs_tp_029_pinned_goldens` |
| TRS-TP-030 | Pinned golden | TRS-REQ-013 | The rounded inverse and normal image for the same matrix, and the exact dot products of the normal image and of the linear image with a transformed tangent, from the Python model. Runs in the WebAssembly gate. | `crates/bunny-linalg/tests/normal_transform_tests.rs::trs_tp_030_pinned_normal_golden` |
| TRS-TP-031 | Mutation | TRS-REQ-010, TRS-REQ-011, TRS-REQ-012, TRS-REQ-013 | Each rule of the ray, box and normal transforms broken on purpose fails at least one case above. | [Mutations](#mutations) |
| TRS-TP-032 | Documentation audit | TRS-REQ-014 | The chapter's section on non-uniform scale states the normal rule, the hull rule and the sphere rule, with the worked examples of TRS-TP-023 and TRS-TP-026. | [Non-uniform scale](README.md#non-uniform-scale) |
| TRS-TP-050 | Hand-derived | TRS-REQ-020 | Identity gives the box and a translation moves it exactly; the eighth turn maps the unit cube to `(-c, 0, 0)` to `(c, 2c, 1)`; a negative non-uniform scale swaps the faces; `m00 = 1` raw rounds `1 + 2^-32` and `3 - 2^-32` out to `[1, 3]`, and the image `1/2` raw (whose rounded point transform is 0) to `[0, 1]`; a point the rounded transform refuses (`2 * 2^62 - 2 * 2^62`) is bounded at its exact image 0; a `from_trs` rotation, scale and translation against the oracle. | `crates/bunny-geom/tests/transform_bounds_tests.rs::trs_tp_050_box_bounds_under_identity_translation_rotation_scale_and_mixed` |
| TRS-TP-051 | Boundary | TRS-REQ-020 | Inverted on each axis (equal accepted) and inverted under an overflowing transform: `InvertedBox`; translation `i64::MAX - 5` with `x = 5` and 6, `i64::MIN + 5` with -5 and -6; `m00 = 1 + 2^-32` with `x = 0x7FFF_FFFF_7FFF_FFFF` (ceiling `i64::MAX`) and one raw unit more, and `-0x7FFF_FFFF_8000_0000` and one less. | `crates/bunny-geom/tests/transform_bounds_tests.rs::trs_tp_051_box_refusals_one_raw_unit_either_side` |
| TRS-TP-052 | Seeded oracle | TRS-REQ-020 | 10 000 seeded full-range boxes: each face contains and is tight against the big-integer eight-corner extremes, and `OutOfRange` exactly when they leave the range; the in-range count is pinned. 10 000 moderate boxes, all in range. | `crates/bunny-geom/tests/transform_bounds_tests.rs::trs_tp_052_seeded_boxes_match_the_big_integer_oracle` |
| TRS-TP-053 | Hand-derived | TRS-REQ-021 | An empty slice is `NoPoints`; one point gives the floor and ceiling of its image (`1/2` and `-3/2` raw); identity equals `from_points`; translation moves exactly; the triangle equals the points of its vertices, a degenerate one its single vertex; the eighth turn; `m00 = 1 + 2^-32` one raw unit either side of the top of the range. | `crates/bunny-geom/tests/transform_bounds_tests.rs::trs_tp_053_points_and_triangles` |
| TRS-TP-054 | Seeded oracle | TRS-REQ-021 | 10 000 seeded full-range sets of 1 to 6 points against the big-integer oracle; the in-range count is pinned. | `crates/bunny-geom/tests/transform_bounds_tests.rs::trs_tp_054_seeded_points_match_the_big_integer_oracle` |
| TRS-TP-055 | Hand-derived | TRS-REQ-022 | Identity and translation give centre plus or minus the radius exactly; a zero radius the bounds of the centre's image (`1/2` raw to `[0, 1]`); the scale `(2, 3, 1/4)` scales each extent exactly; the eighth turn reaches `2^32 + 1` raw on `x` and `y` (the row length `c sqrt 2` is `2^32 + 0.39` raw) and `2^32` on `z`; a singular scale flattens one axis; a `from_trs` transform against the oracle. | `crates/bunny-geom/tests/transform_bounds_tests.rs::trs_tp_055_sphere_bounds_under_identity_translation_rotation_scale_and_mixed` |
| TRS-TP-056 | Boundary | TRS-REQ-022 | Radius -1 raw and `i64::MIN` are `NegativeRadius`, also under an overflowing transform; 0 is accepted; radius 7 at `x = i64::MAX - 7` reaches `i64::MAX`, `i64::MAX - 6` is `OutOfRange`, and the same at `i64::MIN`; radius `i64::MAX` at the origin gives `[-i64::MAX, i64::MAX]`; full-range rows are `OutOfRange`. | `crates/bunny-geom/tests/transform_bounds_tests.rs::trs_tp_056_sphere_refusals_one_raw_unit_either_side` |
| TRS-TP-057 | Seeded oracle | TRS-REQ-022 | 10 000 seeded full-range spheres against the big-integer oracle, the in-range count pinned; 10 000 moderate spheres, all in range and each inside `transformed_bounds` of its `from_sphere` box. | `crates/bunny-geom/tests/transform_bounds_tests.rs::trs_tp_057_seeded_spheres_match_the_big_integer_oracle` |
| TRS-TP-058 | Pinned golden | TRS-REQ-020, TRS-REQ-021, TRS-REQ-022 | Raw box, triangle and sphere bounds for the sheared transform of TRS-TP-029, from the Python model; the box's `y` minimum and `z` maximum lie one raw unit outside TRS-TP-029's rounded hull. Runs in the WebAssembly gate. | `crates/bunny-geom/tests/transform_bounds_tests.rs::trs_tp_058_pinned_goldens` |
| TRS-TP-059 | Mutation | TRS-REQ-020, TRS-REQ-021, TRS-REQ-022 | Each rule of the transform-aware bounds broken on purpose fails at least one case above. | [Mutations](#mutations) |
| TRS-TP-060 | Documentation audit | TRS-REQ-023 | The chapter's section on transform-aware bounds states the outward rounding and its slack, the overflow rule, the widths, and the relation to the eight-corner hull and the rounded point transform. | [Transform-aware bounds](README.md#transform-aware-bounds) |

```toml
[[case]]
id = "TRS-TP-001"
requirements = ["TRS-REQ-001"]
evidence = "test"
test = "crates/bunny-linalg/tests/transform_tests.rs::trs_tp_001_scale_then_rotate_then_translate"
oracle = "Hand-derived scale, rotate, translate of one point; the swapped order; the literal T * R * S composition."
tier = "fast"
status = "implemented"

[[case]]
id = "TRS-TP-002"
requirements = ["TRS-REQ-003", "TRS-REQ-004"]
evidence = "test"
test = "crates/bunny-linalg/tests/transform_tests.rs::trs_tp_002_rotation_and_scale_about_a_point_fix_the_pivot"
oracle = "Hand-derived half turn and scale about a pivot; the literal T(p) R T(-p) composition."
tier = "fast"
status = "implemented"

[[case]]
id = "TRS-TP-003"
requirements = ["TRS-REQ-002", "TRS-REQ-003", "TRS-REQ-004"]
evidence = "test"
test = "crates/bunny-linalg/tests/transform_tests.rs::trs_tp_003_products_on_a_half_round_to_even"
oracle = "Hand-derived exact halves at 60 degrees rounded to even."
tier = "fast"
status = "implemented"

[[case]]
id = "TRS-TP-004"
requirements = ["TRS-REQ-002", "TRS-REQ-003", "TRS-REQ-004"]
evidence = "test"
test = "crates/bunny-linalg/tests/transform_tests.rs::trs_tp_004_pinned_goldens"
oracle = "Raw goldens from the Python model, each checked in Fraction against the exact value."
tier = "fast"
status = "implemented"

[[case]]
id = "TRS-TP-005"
requirements = ["TRS-REQ-002", "TRS-REQ-003", "TRS-REQ-004"]
evidence = "test"
test = "crates/bunny-linalg/tests/transform_tests.rs::trs_tp_005_seeded_constructors_match_the_oracle_and_fix_the_pivot"
oracle = "Independent i128 oracle and exact bounds over 10 000 seeded full-range cases."
tier = "fast"
status = "implemented"

[[case]]
id = "TRS-TP-006"
requirements = ["TRS-REQ-002", "TRS-REQ-003", "TRS-REQ-004"]
evidence = "test"
test = "crates/bunny-linalg/tests/transform_tests.rs::trs_tp_006_overflow_is_none_one_raw_unit_past_each_threshold"
oracle = "Hand-derived and bisected thresholds, one raw unit either side."
tier = "fast"
status = "implemented"

[[case]]
id = "TRS-TP-007"
requirements = ["TRS-REQ-002", "TRS-REQ-004"]
evidence = "test"
test = "crates/bunny-linalg/tests/transform_tests.rs::trs_tp_007_zero_and_negative_scales_are_accepted"
oracle = "Exact determinants and images of zero and negative scales."
tier = "fast"
status = "implemented"

[[case]]
id = "TRS-TP-008"
requirements = ["TRS-REQ-005"]
evidence = "test"
test = "crates/bunny-linalg/tests/transform_tests.rs::trs_tp_008_composition_is_associative_within_the_rounding_bound"
oracle = "Exact i128 product of three transforms over 10 000 seeded triples."
tier = "fast"
status = "implemented"

[[case]]
id = "TRS-TP-009"
requirements = ["TRS-REQ-001", "TRS-REQ-002", "TRS-REQ-003", "TRS-REQ-004"]
evidence = "manual-audit"
artifact = "docs/topics/transform/test-plan.md#mutations"
oracle = "Each rule broken on purpose fails at least one case."
tier = "manual"
status = "implemented"

[[case]]
id = "TRS-TP-020"
requirements = ["TRS-REQ-010", "TRS-REQ-011"]
evidence = "test"
test = "crates/bunny-geom/tests/ray_box_transform_tests.rs::trs_tp_020_ray_image_keeps_its_parameter"
oracle = "Hand-derived ray image at a fixed parameter; renormalisation."
tier = "fast"
status = "implemented"

[[case]]
id = "TRS-TP-021"
requirements = ["TRS-REQ-011"]
evidence = "test"
test = "crates/bunny-geom/tests/ray_box_transform_tests.rs::trs_tp_021_ray_refusals_one_raw_unit_either_side"
oracle = "Hand-derived thresholds, one raw unit either side."
tier = "fast"
status = "implemented"

[[case]]
id = "TRS-TP-022"
requirements = ["TRS-REQ-010", "TRS-REQ-011"]
evidence = "test"
test = "crates/bunny-geom/tests/ray_box_transform_tests.rs::trs_tp_022_seeded_rays_match_the_oracle_and_the_parameter_bound"
oracle = "Independent i128 oracle; exact parameter bound over seeded rays."
tier = "fast"
status = "implemented"

[[case]]
id = "TRS-TP-023"
requirements = ["TRS-REQ-012"]
evidence = "test"
test = "crates/bunny-geom/tests/ray_box_transform_tests.rs::trs_tp_023_box_hull_of_a_rotated_cube_and_a_point"
oracle = "Hand-derived corner images of a rotated cube."
tier = "fast"
status = "implemented"

[[case]]
id = "TRS-TP-024"
requirements = ["TRS-REQ-012"]
evidence = "test"
test = "crates/bunny-geom/tests/ray_box_transform_tests.rs::trs_tp_024_box_refusals_one_raw_unit_either_side"
oracle = "Hand-derived thresholds, one raw unit either side."
tier = "fast"
status = "implemented"

[[case]]
id = "TRS-TP-025"
requirements = ["TRS-REQ-012"]
evidence = "test"
test = "crates/bunny-geom/tests/ray_box_transform_tests.rs::trs_tp_025_seeded_boxes_contain_every_rounded_corner"
oracle = "Independent i128 corner oracle and exact hull faces over seeded boxes."
tier = "fast"
status = "implemented"

[[case]]
id = "TRS-TP-026"
requirements = ["TRS-REQ-013"]
evidence = "test"
test = "crates/bunny-linalg/tests/normal_transform_tests.rs::trs_tp_026_non_uniform_scale_keeps_normals_perpendicular"
oracle = "Hand-derived inverse transposes and dot products."
tier = "fast"
status = "implemented"

[[case]]
id = "TRS-TP-027"
requirements = ["TRS-REQ-013"]
evidence = "test"
test = "crates/bunny-linalg/tests/normal_transform_tests.rs::trs_tp_027_normal_refusals_one_raw_unit_either_side"
oracle = "Hand-derived thresholds, one raw unit either side."
tier = "fast"
status = "implemented"

[[case]]
id = "TRS-TP-028"
requirements = ["TRS-REQ-013"]
evidence = "test"
test = "crates/bunny-linalg/tests/normal_transform_tests.rs::trs_tp_028_seeded_normals_match_the_oracle"
oracle = "Independent i128 product oracle; exact dot products."
tier = "fast"
status = "implemented"

[[case]]
id = "TRS-TP-029"
requirements = ["TRS-REQ-010", "TRS-REQ-012"]
evidence = "test"
test = "crates/bunny-geom/tests/ray_box_transform_tests.rs::trs_tp_029_pinned_goldens"
oracle = "Raw goldens from the Python model, each checked in Fraction against the exact value."
tier = "fast"
status = "implemented"

[[case]]
id = "TRS-TP-030"
requirements = ["TRS-REQ-013"]
evidence = "test"
test = "crates/bunny-linalg/tests/normal_transform_tests.rs::trs_tp_030_pinned_normal_golden"
oracle = "Raw goldens from the Python model, each checked in Fraction against the exact value."
tier = "fast"
status = "implemented"

[[case]]
id = "TRS-TP-031"
requirements = ["TRS-REQ-010", "TRS-REQ-011", "TRS-REQ-012", "TRS-REQ-013"]
evidence = "manual-audit"
artifact = "docs/topics/transform/test-plan.md#mutations"
oracle = "Each rule of the ray, box and normal transforms broken on purpose fails at least one case."
tier = "manual"
status = "implemented"

[[case]]
id = "TRS-TP-032"
requirements = ["TRS-REQ-014"]
evidence = "manual-audit"
artifact = "docs/topics/transform/README.md#non-uniform-scale"
oracle = "The chapter states the normal, hull and sphere rules with worked examples."
tier = "manual"
status = "implemented"

[[case]]
id = "TRS-TP-050"
requirements = ["TRS-REQ-020"]
evidence = "test"
test = "crates/bunny-geom/tests/transform_bounds_tests.rs::trs_tp_050_box_bounds_under_identity_translation_rotation_scale_and_mixed"
oracle = "Hand-derived exact hulls; the big-integer oracle for the mixed transform."
tier = "fast"
status = "implemented"

[[case]]
id = "TRS-TP-051"
requirements = ["TRS-REQ-020"]
evidence = "test"
test = "crates/bunny-geom/tests/transform_bounds_tests.rs::trs_tp_051_box_refusals_one_raw_unit_either_side"
oracle = "Hand-derived thresholds, one raw unit either side."
tier = "fast"
status = "implemented"

[[case]]
id = "TRS-TP-052"
requirements = ["TRS-REQ-020"]
evidence = "test"
test = "crates/bunny-geom/tests/transform_bounds_tests.rs::trs_tp_052_seeded_boxes_match_the_big_integer_oracle"
oracle = "Big-integer eight-corner extremes over seeded boxes."
tier = "fast"
status = "implemented"

[[case]]
id = "TRS-TP-053"
requirements = ["TRS-REQ-021"]
evidence = "test"
test = "crates/bunny-geom/tests/transform_bounds_tests.rs::trs_tp_053_points_and_triangles"
oracle = "Hand-derived point and triangle images; from_points under the identity."
tier = "fast"
status = "implemented"

[[case]]
id = "TRS-TP-054"
requirements = ["TRS-REQ-021"]
evidence = "test"
test = "crates/bunny-geom/tests/transform_bounds_tests.rs::trs_tp_054_seeded_points_match_the_big_integer_oracle"
oracle = "Big-integer point extremes over seeded point sets."
tier = "fast"
status = "implemented"

[[case]]
id = "TRS-TP-055"
requirements = ["TRS-REQ-022"]
evidence = "test"
test = "crates/bunny-geom/tests/transform_bounds_tests.rs::trs_tp_055_sphere_bounds_under_identity_translation_rotation_scale_and_mixed"
oracle = "Hand-derived ellipsoid extents; the big-integer oracle for the mixed transform."
tier = "fast"
status = "implemented"

[[case]]
id = "TRS-TP-056"
requirements = ["TRS-REQ-022"]
evidence = "test"
test = "crates/bunny-geom/tests/transform_bounds_tests.rs::trs_tp_056_sphere_refusals_one_raw_unit_either_side"
oracle = "Hand-derived thresholds, one raw unit either side."
tier = "fast"
status = "implemented"

[[case]]
id = "TRS-TP-057"
requirements = ["TRS-REQ-022"]
evidence = "test"
test = "crates/bunny-geom/tests/transform_bounds_tests.rs::trs_tp_057_seeded_spheres_match_the_big_integer_oracle"
oracle = "Big-integer squared comparisons over seeded spheres; containment in the box bounds."
tier = "fast"
status = "implemented"

[[case]]
id = "TRS-TP-058"
requirements = ["TRS-REQ-020", "TRS-REQ-021", "TRS-REQ-022"]
evidence = "test"
test = "crates/bunny-geom/tests/transform_bounds_tests.rs::trs_tp_058_pinned_goldens"
oracle = "Raw goldens from the Python model, floor and ceiling of Fraction extremes."
tier = "fast"
status = "implemented"

[[case]]
id = "TRS-TP-059"
requirements = ["TRS-REQ-020", "TRS-REQ-021", "TRS-REQ-022"]
evidence = "manual-audit"
artifact = "docs/topics/transform/test-plan.md#mutations"
oracle = "Each rule of the transform-aware bounds broken on purpose fails at least one case."
tier = "manual"
status = "implemented"

[[case]]
id = "TRS-TP-060"
requirements = ["TRS-REQ-023"]
evidence = "manual-audit"
artifact = "docs/topics/transform/README.md#transform-aware-bounds"
oracle = "The chapter states the rounding, overflow rule, widths and relation to the eight-corner hull."
tier = "manual"
status = "implemented"
```

## Mutations

Each mutation was applied to the kernel, `transform_tests` and `affine_transform_tests` run, and
the kernel restored; each fails the named cases.

| Mutation | Fails |
| --- | --- |
| `from_trs` applies `S` after `R` (`diag(s) * R`) | TRS-TP-001 (maps `(1, 0, 0)` to `(1, 0, -4)`), TRS-TP-004, TRS-TP-005 |
| `from_trs` drops the translation | TRS-TP-001, TRS-TP-004, TRS-TP-005, TRS-TP-006 |
| `from_trs` floors the product instead of rounding to nearest | TRS-TP-003, TRS-TP-005, TRS-TP-006 |
| `from_trs` saturates instead of refusing on overflow | TRS-TP-005, TRS-TP-006 |
| Pivot translation `p + R p` | TRS-TP-002, TRS-TP-003, TRS-TP-004, TRS-TP-005, TRS-TP-006 |
| Pivot translation `R p - p` | TRS-TP-002, TRS-TP-003, TRS-TP-004, TRS-TP-005, TRS-TP-006, TRS-TP-007 |
| `rotate_about_point` stores the transposed rotation | TRS-TP-003, TRS-TP-005 |
| `scale_about_point` permutes the scale components | TRS-TP-002, TRS-TP-003, TRS-TP-004, TRS-TP-005, TRS-TP-006 |

TRS-TP-008 tests the existing `checked_mul_affine` and no kernel line of this contract.

The ray, box and normal mutations (TRS-TP-031) were applied to `crates/bunny-geom/src/transform.rs`
or `crates/bunny-linalg/src/fixed_affine3_normal.rs`, `ray_box_transform_tests` and
`normal_transform_tests` run, and the kernel restored:

| Mutation | Fails |
| --- | --- |
| The ray direction mapped as a point (translated) | TRS-TP-020, TRS-TP-022, TRS-TP-029 |
| The ray origin mapped as a vector (not translated) | TRS-TP-020, TRS-TP-021, TRS-TP-022, TRS-TP-029 |
| The ray direction renormalised | TRS-TP-020, TRS-TP-021, TRS-TP-022, TRS-TP-029 |
| A zero ray direction accepted | TRS-TP-021 |
| An inverted box accepted | TRS-TP-024 |
| The box hull of the minimum and maximum corners only | TRS-TP-023, TRS-TP-025, TRS-TP-029 |
| An out-of-range corner skipped instead of refused | TRS-TP-024, TRS-TP-025 |
| The corner's `y` chosen by the `z` bit | TRS-TP-025, TRS-TP-029 |
| The normal by the inverse, not its transpose | TRS-TP-026, TRS-TP-028, TRS-TP-030 |
| The normal by the linear part | TRS-TP-026, TRS-TP-027, TRS-TP-028, TRS-TP-030 |
| The normal by the transpose, without the inverse | TRS-TP-026, TRS-TP-027, TRS-TP-028, TRS-TP-030 |

The transform-aware bounds mutations (TRS-TP-059) were applied to
`crates/bunny-geom/src/transform_bounds.rs` or `transform_bounds/wide.rs`, `transform_bounds_tests`
run, and the kernel restored:

| Mutation | Fails |
| --- | --- |
| The box minimum face rounded up (ceiling, not floor) | TRS-TP-050, TRS-TP-051, TRS-TP-052, TRS-TP-057, TRS-TP-058 |
| The box extremes taken from the minimum corner only | TRS-TP-050, TRS-TP-051, TRS-TP-052, TRS-TP-057, TRS-TP-058 |
| An inverted box accepted | TRS-TP-051 |
| The remainders' carry dropped from the exact sum | TRS-TP-052, TRS-TP-054, TRS-TP-057, TRS-TP-058 |
| The ceiling always one above the floor | TRS-TP-050, TRS-TP-051, TRS-TP-052, TRS-TP-053, TRS-TP-054, TRS-TP-058 |
| The point fold keeps the greater low face | TRS-TP-053, TRS-TP-054, TRS-TP-058 |
| The translation ignored | TRS-TP-050, TRS-TP-051, TRS-TP-052, TRS-TP-053, TRS-TP-054, TRS-TP-055, TRS-TP-057, TRS-TP-058 |
| An out-of-range face saturated instead of refused | TRS-TP-051, TRS-TP-052, TRS-TP-053, TRS-TP-054, TRS-TP-056, TRS-TP-057 |
| A negative radius taken as its magnitude | TRS-TP-056 |
| The sphere reach along a column, not a row | TRS-TP-055, TRS-TP-057, TRS-TP-058 |
| The sphere's low face from the high-side search | TRS-TP-055, TRS-TP-057, TRS-TP-058 |
| The reach search returns one past the least | TRS-TP-055, TRS-TP-056, TRS-TP-057, TRS-TP-058 |
| The reach compared with `>` instead of `>=` | TRS-TP-055, TRS-TP-056, TRS-TP-057 |

## Determinism Obligations And Evidence

The kernel uses only integer arithmetic; the tests use fixed seeds and no clock. They run
natively and in the WebAssembly gate.
