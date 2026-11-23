# Exact Geometric Predicates (`bunny-predicates/1`)

This chapter states Bunny's exact orientation predicates, signed areas and volumes, and
barycentric coordinates. A Q32.32 point is an integer on the raw grid, so each predicate's
determinant is an integer, and its sign is computed exactly, with no rounding, for every
representable input, `i64::MIN` and `i64::MAX` coordinates included. The identity string is
`bunny-predicates/1` (`bunny_geom::PREDICATES_CONTRACT_ID`). The [test plan](test-plan.md)
holds the oracles, golden vectors and mutation checks. It answers flyingrobots/bunny#145.

A predicate computed with rounded arithmetic gives inconsistent answers near zero (three
points that are collinear in one order and not in another), which is what breaks geometric
algorithms. Here the sign is the sign of the exact integer. Every function that rounds is
named `_rounded` and returns a quantity; none of them decides a sign.

| Function | Exact value | Rounded? | Refused (`None`) |
| --- | --- | --- | --- |
| `orientation2(a, b, c)` | sign of `D2 = (b - a) x (c - a)` | never | never |
| `doubled_signed_area2(a, b, c)` | `D2`, raw squared units | never | never |
| `signed_area2_rounded(a, b, c)` | `D2 / 2^33` (the area) | once, ties to even | outside `i64` |
| `orientation3(a, b, c, d)` | sign of `D3 = (d - a) . ((b - a) x (c - a))` | never | never |
| `sextupled_signed_volume3(a, b, c, d)` | `D3`, raw cubed units | never | never |
| `signed_volume3_rounded(a, b, c, d)` | `D3 / (6 2^64)` (the volume) | once, ties to even | outside `i64` |
| `barycentric2(p, a, b, c)` | numerators and denominator | never | `D2(a, b, c) = 0` |
| `barycentric3_coplanar(p, a, b, c)` | numerators and denominator | never | zero area, or `p` off the plane |
| `ExactBarycentric::location()` | inside, on edge, on vertex, outside | never | never |
| `ExactBarycentric::weights_rounded()` | each numerator `2^32` over the denominator | once each, ties to even | any outside `i64` |

All of it is in `bunny-geom`, `no_std` with no allocation, and depends only on
`FixedVec2` and `FixedVec3`.

## Exact Values

`ExactDeterminant` holds an exact integer: a sign and a magnitude of four little-endian
64-bit limbs (`magnitude()`), with `sign()`, `is_zero()`, `is_negative()` and `to_i128()`
(`None` when it does not fit). Zero is never negative, so equal values compare equal.

Widths, with `L = 2^64 - 1`:

- A difference of two raw `i64` values has magnitude at most `L`, so it needs 65 signed bits.
- A product of two differences is below `2^128`; a product of three below `2^192`.
- `|D2| <= L^2 < 2^128`. The two products of the cross product are each below `2^128`, and
  a triangle inside the raw square covers at most half of it, so `|D2|` is at most `L^2`,
  reached by a corner triangle of the square.
- `|D3| <= 2 L^3 < 2^193`. Each of the six terms is below `2^192`, so the sum is below
  `6 2^192 < 2^195` while it is formed; a tetrahedron inside the raw cube has at most a third
  of its volume, so the result is at most `2 L^3`, reached by the regular tetrahedron on
  alternate corners of the cube.

The kernel holds every intermediate in a private 256-bit sign-and-magnitude integer
(`crates/bunny-geom/src/predicates/wide.rs`), so no input overflows and nothing is
discarded. The workspace has other private wide integers of the same kind;
flyingrobots/bunny#424 tracks sharing one.

## 2D Orientation

`orientation2(a, b, c)` returns the sign of the exact `D2 = (b - a) x (c - a)` on raw
values, `(bx - ax)(cy - ay) - (by - ay)(cx - ax)`:

| `D2` | `Orientation2` | In the canonical plane |
| --- | --- | --- |
| positive | `CounterClockwise` | `c` is left of the line from `a` to `b` |
| negative | `Clockwise` | `c` is right of it |
| exactly zero | `Collinear` | the three points are on one line, or repeated |

The names follow Bunny's canonical 2D plane ([coordinate law](../coordinate-law/README.md#canonical-2d-plane)):
`+X` right, `+Y` up, viewed from `+Z`, where positive winding is counter-clockwise and
`(b - a) x (c - a)` points toward `+Z`. In a `y`-down frame, such as the viewport's pixel
coordinates, the same positive sign looks clockwise on screen; the enum keeps its canonical
name.

Exact zero and touching, in raw units: `(0, 0)`, `(1, 1)`, `(2, 2)` are `Collinear`;
moving `c` to `(2, 3)` gives `D2 = 1` (`CounterClockwise`) and to `(2, 1)` gives `D2 = -1`
(`Clockwise`). At the ends of the range, `(MIN, MIN)`, `(0, 0)`, `(MAX, MAX)` are
`Collinear`, and `c = (MAX, MAX - 1)` gives `D2 = -2^63` against products near `2^127`, a
change a rounded or floating-point product loses.

## Signed Area

`doubled_signed_area2(a, b, c)` returns `D2` itself: twice the signed area, in raw squared
units (`2^-64` square units). It is generally not on the Q32.32 grid and does not fit it.

`signed_area2_rounded(a, b, c)` is the area in Q32.32, rounded once from the exact value:

```text
raw = round_half_even(D2 / 2^33)
```

It is `None` when that is outside `i64` (an area of about `2^31` square units or more). Its
sign agrees with `orientation2` except that a nonzero area of at most half a raw unit rounds
to zero: the result is zero exactly when `|D2| <= 2^32`. `D2 = 1` is `CounterClockwise`
with a rounded area of zero; `D2 = 2^32` is a tie that goes to 0; `2^32 + 1` gives 1 raw;
`3 2^32` is a tie that goes to 2. Use `orientation2` for the sign.

## 3D Orientation

`orientation3(a, b, c, d)` returns the sign of the exact
`D3 = (d - a) . ((b - a) x (c - a))`, the determinant of the three edge vectors from `a`:

| `D3` | `Orientation3` | Meaning |
| --- | --- | --- |
| positive | `Front` | `d` is on the side the normal `(b - a) x (c - a)` points to |
| negative | `Behind` | `d` is on the other side |
| exactly zero | `Coplanar` | the four points are on one plane, or `a`, `b`, `c` are collinear or repeated |

Bunny's frame is right-handed ([coordinate law](../coordinate-law/README.md#canonical-3d-frame)),
so the normal is given by the right-hand rule: `d` is `Front` when `a`, `b`, `c` appear
counter-clockwise from `d`'s side. For `a = 0`, `b = (1, 0, 0)`, `c = (0, 1, 0)` the normal
is `+Z` and `d = (5, 7, 1)` raw is `Front` with `D3 = 1`.

Exact zero and touching: the face `z = MIN` of the raw cube is `Coplanar` for its four
corners, and moving `d` by one raw unit of `z` is `Front`; on the tilted plane `x = y`
spanning the whole range, `(0, 0, 5)` is `Coplanar`, `(1, 0, 5)` is `Front` and `(0, 1, 5)`
is `Behind`.

## Signed Volume

`sextupled_signed_volume3(a, b, c, d)` returns `D3`: six times the signed volume of the
tetrahedron, in raw cubed units (`2^-96` cubic units).

`signed_volume3_rounded(a, b, c, d)` is the volume in Q32.32, rounded once:

```text
raw = round_half_even(D3 / (6 2^64))
```

`None` outside `i64`. Its sign agrees with `orientation3` except that a nonzero volume of
at most half a raw unit rounds to zero: the result is zero exactly when `|D3| <= 3 2^64`.
Use `orientation3` for the side.

## Barycentric Numerators

`barycentric2(p, a, b, c)` returns an `ExactBarycentric`: the exact numerators of the
weights of `a`, `b` and `c` over one common denominator, all in raw squared units:

```text
na = D2(p, b, c)    nb = D2(a, p, c)    nc = D2(a, b, p)    d = D2(a, b, c)
```

When `d < 0` (a clockwise triangle) all four are negated, so `denominator()` is positive and
the weights `na / d`, `nb / d`, `nc / d` are those of the same point whatever the vertex
order. `na + nb + nc = d` exactly. Each magnitude is at most `L^2`. A triangle with
`d = 0` exactly (collinear or repeated vertices) is refused with `None`.

The weights are ratios of exact areas and are generally not on the Q32.32 grid, which is why
the type keeps the integers. `nb`, `nc` and `d` correspond to the `u`, `v` and `det` of
`bunny_query::BarycentricNumerators`, which the
[ray-triangle test](../ray-tracing/README.md#exact-triangle-test) returns in `i128` for hits
inside the trace domain only. That type is unchanged; this one covers every point and every
representable triangle.

## Location

`location()` classifies the point exactly from the signs of the normalized numerators, in
this precedence:

1. `Outside` when any numerator is negative. A point on the line of an edge but past a
   vertex has one zero numerator and one negative, so it is outside.
2. `OnVertex(TriangleVertex::A | B | C)` when two numerators are zero; the vertex is the
   one whose numerator is the denominator.
3. `OnEdge(TriangleEdge::BC | CA | AB)` when one numerator is zero; `BC` is the edge
   opposite `a` (`na = 0`), and so on.
4. `Inside` when none is zero.

The four outcomes partition the plane. Two convenience forms name the touching rule:

- `contains_closed()`: the closed triangle, boundary included (not `Outside`). This matches
  the closed-front rule of [plane clipping](../plane-clipping/README.md#on-plane-rule) and
  the inclusive edge rule of the ray-triangle test.
- `contains_open()`: the open triangle, boundary excluded (`Inside`).

For `a = (0, 0)`, `b = (4, 0)`, `c = (0, 4)` raw: `(2, 0)` is `OnEdge(AB)` with numerators
`(8, 8, 0)` over 16, `(2, 1)` is `Inside`, `(2, -1)` is `Outside`, `(4, 0)` is
`OnVertex(B)`, and `(5, 0)` is `Outside` with numerators `(-4, 20, 0)`.

## Coplanar 3D Points

`barycentric3_coplanar(p, a, b, c)` returns `None` when the triangle has zero area or `p` is
not exactly in its plane (`orientation3(a, b, c, p)` is not `Coplanar`). Otherwise it is
`barycentric2` of the projection that drops the first axis, in the order `x`, `y`, `z`, in
which the normal `(b - a) x (c - a)` is nonzero, keeping `(y, z)`, `(z, x)` or `(x, y)`.
That projection's doubled area is the dropped axis's normal component, so it is nonzero,
and for a point in the plane the ratios equal those of the 3D areas exactly. The numerators
are in the units of that projection: for the plane `z = 2x + 3y` with `a = 0`,
`b = (1, 0, 2)`, `c = (0, 1, 3)` the denominator is 2, where the `(x, y)` projection would
give 1.

A point exactly in the plane of a general triangle is rare on the raw grid; a point that is
not is refused rather than projected.

## Rounded Weights

`weights_rounded()` rounds each weight once, from the exact ratio:

```text
raw_i = round_half_even(n_i 2^32 / d)
```

`None` when any of the three is outside `i64`. Each is within half a raw unit of the exact
weight, and the exact weights sum to 1, so the three raw weights sum to `2^32` within one raw
unit. They are not adjusted to sum exactly: for the triangle `(0, 0)`, `(3, 0)`, `(0, 3)`
and `p = (1, 1)` each weight is `1431655765` and the sum is `2^32 - 1`; for `(0, 0)`,
`(6, 0)`, `(0, 6)` the weights `(2/3, 1/6, 1/6)` sum to `2^32 + 1`. Decide location with
`location()`, never from rounded weights. The trace kernel's `TriangleHit::barycentrics`
floors instead; the two are different functions with different rules.

## Consistency

Because every value is exact, the identities of the determinant hold exactly:

- swapping any two points negates `D2` and `D3`;
- a cyclic rotation of the three 2D points, or an even permutation of the four 3D points,
  leaves the value unchanged;
- each normalized numerator has the sign of its sub-triangle's orientation times the
  triangle's orientation, so `location()` agrees with `orientation2` on every sub-triangle.

## Relation To Plane Classification

`FixedPlane3::classify` ([plane clipping](../plane-clipping/README.md#classification)) is a
different contract. It computes the signed distance `normal . p - offset` with three
`checked_mul` products, each rounded ties-to-even, and reads the sign of that rounded
distance. It is exact in the rounded distance (two points with the same computed distance
get the same side), not against the real plane: a point a fraction of a raw unit from the
plane can be classified `On`, and it returns `None` on overflow. `orientation3` takes the
plane as three points and its sign is exact against the real plane for every input, with no
refusal. Use `orientation3` when the plane is given by points and the answer must be robust;
use `classify` with a `FixedPlane3` and its stated rounding.
