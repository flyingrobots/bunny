# Plane Classification And Segment Clipping

This document defines Bunny's fixed-point plane and segment clipping contract.

The surface lives in `bunny-geom`: `FixedPlane3`, `PlaneSide`, `FixedSegment3`
and `SegmentClip`. Polygon clipping is out of scope (flyingrobots/bunny#204).

## Status

This is a living topic chapter, not a proposal. The verification design is
[`test-plan.md`](test-plan.md).

## Plane

`FixedPlane3 { normal, offset }` is the set `normal . p = offset`. The signed
distance of `p` is `normal . p - offset`. The normal need not be unit length;
distances scale by its length. A zero normal is accepted and gives every point
the distance `-offset`.

`signed_distance` multiplies each coordinate with `checked_mul`, adds with
`checked_add` and subtracts the offset with `checked_sub`. It returns `None` on
overflow.

## Classification

`classify` returns `Some(PlaneSide::Front)` for positive signed distance,
`Behind` for negative and `On` for exactly zero (raw value 0), or `None` on
overflow. Classification is exact in the rounded distance: two points with the
same computed distance get the same side.

## On-Plane Rule

Clipping keeps the closed front half-space. A point whose signed distance is
exactly zero belongs to the front: `On` and `Front` are both kept, only
`Behind` is clipped. Classification and clipping share one predicate, so they
cannot disagree.

## Clipping

`clip_segment(FixedSegment3) -> Option<SegmentClip>`:

| Endpoint sides | Result |
| --- | --- |
| both kept (Front or On, including a coplanar segment) | `Kept` with the segment unchanged |
| both Behind | `Outside` |
| one kept, one Behind | `Kept` with the front part, original endpoint order preserved |

A segment touching the plane at one end and otherwise behind returns a
degenerate segment whose two endpoints are the touching point. `None` means
overflow, never "outside".

## Rounding Of The Clipped Endpoint

For a crossing, with signed distances `da` and `db`:

1. `t = da / (da - db)` is one `checked_div`, rounded ties-to-even in Q32.32.
2. Each coordinate is `a + (b - a) * t`, where `(b - a) * t` is a `checked_mul`
   rounded ties-to-even.

The kept original endpoint is returned bit for bit. When `da` is zero or `db`
is zero the intersection is that exact endpoint.

Bound: per coordinate the result differs from the exact intersection by at
most `|b - a| / 2^33 + 1` raw units (the half unit in `t` times the span, plus
the half unit of the product), plus the half unit of rounding when the exact
value is itself rounded. The sweep test checks `|b - a| / 2^33 + 2`.

Consequence: the clipped endpoint can lie behind the plane by rounding error.
For the plane `z = 1` and `z` running 0 to 3, `t` is raw 1431655765 and the
endpoint is raw 4294967295, one raw unit behind the plane. Clipping does not
repair it.

## Overflow

Any overflow in a distance, in `da - db`, in `t`, or in the interpolation
returns `None`. Nothing saturates and nothing panics.

## Required Tests

The repository keeps tests for classification, exact crossings in both
directions, fully front and fully behind segments, touching ends, coplanar
segments, golden raw outputs, tie rounding, overflow, and a seeded sweep against
an exact `i128` reference. They run natively and under WASM through
`wasm_bindgen_test`.
