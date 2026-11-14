# ASCII PLY Triangle Meshes

This chapter defines the contract of `parse_ascii_ply` in `bunny-codec`: the
ASCII PLY profile `bunny-ascii-ply-triangle-mesh/1`, and the conversion rule
`bunny-decimal-q32-32/1` that turns the decimal text of a coordinate into a
Q32.32 raw value exactly, with integer arithmetic only.

## Status

This is a living topic chapter, not a proposal. It is implemented in
`crates/bunny-codec/src/ply_ascii.rs` and the modules beside it, and exported as:

- `parse_ascii_ply`, `AsciiPlyMesh` (`vertex_count`, `face_count`, `vertices`,
  `triangles`), `AsciiPlyVertices`, `AsciiPlyTriangles`, `AsciiPlyVertex`;
- `AsciiPlyError` (`kind`, `line`) and `AsciiPlyErrorKind`;
- `parse_decimal_q32_32`, `DecimalError`, `DECIMAL_Q32_32_ID`
  (`"bunny-decimal-q32-32/1"`) and `MAX_FRACTIONAL_SIGNIFICANT_DIGITS` (28);
- `ASCII_PLY_PROFILE_ID` (`"bunny-ascii-ply-triangle-mesh/1"`).

The test plan is [`test-plan.md`](test-plan.md).

## Relation To The Binary Reader

`parse_binary_ply` is unchanged and keeps every output bit. The ASCII reader is a
new function beside it, not a format dispatch in front of both, because the two
differ in what they return and how they are read:

| | `parse_binary_ply` | `parse_ascii_ply` |
| --- | --- | --- |
| Input | `&[u8]`, `binary_little_endian 1.0` | `&[u8]`, `ascii 1.0` |
| Vertex type | `f32` x, y, z | `FixedQ32_32` x, y, z |
| Extra vertex properties | refused | skipped by their declaration |
| Access | O(1) `vertex(i)`, `triangle(i)` | forward iterators over a re-read of the text |
| Errors | `PlyError`, no position | `AsciiPlyError`, kind and 1-based line |
| Allocation | none after warm-up | none after warm-up |

A caller that wants one entry point can look at the `format` line and call one
of the two; the readers share the borrowed-view shape, the validate-everything-up-front
rule (a returned mesh never fails later) and the face rules.

## The Profile

`parse_ascii_ply` accepts, and nothing else:

1. A header whose first line is `ply`, then `format ascii 1.0`, then any of the
   lines below, then `end_header`.
2. `element vertex N` with scalar properties `property <type> <name>`. The names
   `x`, `y` and `z` must each appear exactly once, in any order, with type
   `float`, `float32`, `double` or `float64`. Every other property is skipped by
   its position in the declared list. Its type must be one of `char`, `uchar`,
   `short`, `ushort`, `int`, `uint`, `float`, `double`, `int8`, `uint8`, `int16`,
   `uint16`, `int32`, `uint32`, `float32`, `float64`, but its text is never
   interpreted, only counted: a vertex line holds exactly one whitespace-separated
   value per declared property. List properties in the vertex element are refused.
3. `element face M` after the vertex element, with exactly one property
   `property list <count> <index> vertex_indices`, where `<count>` is `uchar` or
   `uint8` and `<index>` is `int`, `uint`, `int32` or `uint32`.
4. Any other element is accepted only with count 0, together with its properties,
   and is ignored. A vertex or face element declared twice, or a face element before
   the vertex element, is refused.
5. A body of exactly `N` vertex records, then `M` face records, one record per line.

### Faces

A face line is `3 a b c`: the count must be the decimal integer `3`, then three
indices. An index is a decimal integer with no sign; `a`, `b`, `c` must each be less
than `N` and fit `u32`. Any other count is `NonTriangularFace`; a wrong number of
values is `FaceFieldCount`; a leading `-` is `NegativeIndex`; any other non-digit
text (`+1`, `1.0`, `0x1`) is `InvalidIndex`; an index at or above `N` or above
`u32::MAX` is `IndexOutOfBounds`.

### Line Endings

A line ends at `\n`. One `\r` directly before the `\n` belongs to the terminator, so
LF, CRLF and a mix of the two within one file are all accepted, in the header and the
body. A `\r` anywhere else is an ordinary byte: it is not whitespace, so it ends up
inside a word or a number and is refused there (`InvalidNumber`, `InvalidIndex`,
`UnsupportedFormat`, ...). The `end_header` line must end with `\n`; the last body line
may end at the end of the input without a terminator.

### Whitespace, Blank Lines And Comments

- Words are separated by runs of spaces and tabs. Leading and trailing spaces and tabs
  on any line are ignored, including on the `ply` line and the `end_header` line.
  No other byte (form feed, vertical tab, `\r`) counts as whitespace.
- Header: blank lines are ignored. A line whose first word is `comment` or `obj_info`
  is ignored whatever follows. Any other first word that is not `format`, `element` or
  `property` is `InvalidHeaderLine`. The header must be valid UTF-8 (`HeaderUtf8`),
  so comments may hold UTF-8 text.
- Body: lines that are empty or only spaces and tabs are skipped before, between and
  after records. There are no comments in the body: a comment line is a record that
  fails to parse. Body bytes are never decoded as text; a non-ASCII byte in a number is
  `InvalidNumber`.
- After the last face record only blank lines may follow, else `TrailingData`.

### Counts And Allocation

The reader allocates nothing: it validates the whole text in one pass, then returns a
borrowed view that re-reads the text when iterated (`vertices`, `triangles`; every
item of a validated mesh is `Ok`). Declared counts are never used to reserve memory.
They are only upper bounds on how many records are read, so a header that declares
`element vertex 4294967295` over a short body fails with a typed error after reading
that body. A count that does not fit `usize` is `IntegerOverflow`; a count that is not
a run of ASCII digits is `InvalidCount`. Neither reader limits the counts further.

### Errors And Line Numbers

`AsciiPlyError` carries an `AsciiPlyErrorKind` and a 1-based line number counted from
the start of the input by `\n` characters. The line is where the problem was found:
the offending header or body line; the `end_header` line for a missing element or
property; and, for something that is absent (no `end_header`, too few records), the
line after the last line of the input. Its `Display` is `line <n>: <message>`.

| Kind | Raised for |
| --- | --- |
| `MissingHeaderEnd` | no `end_header` line, or one without a terminating `\n` |
| `HeaderUtf8` | header bytes are not UTF-8 |
| `InvalidMagic` | the first line is not `ply` |
| `UnsupportedFormat` | `format` absent, late, repeated, or not `ascii 1.0` |
| `InvalidHeaderLine` | unknown first word, or extra words on an `element` line |
| `InvalidCount` | an element count that is missing or not digits |
| `IntegerOverflow` | an element count above `usize::MAX` |
| `MissingVertexElement` | no vertex element, or `x`, `y`, `z` not all declared |
| `MissingFaceElement` | no face element or no `vertex_indices` property |
| `UnsupportedElement` | a non-empty extra element, or a repeated vertex or face element |
| `UnsupportedProperty` | wrong type, duplicate coordinate, list property in a vertex, wrong face property |
| `PayloadTooShort` | fewer vertex or face records than declared |
| `TrailingData` | a record after the last declared one |
| `VertexFieldCount` | a vertex line with other than one value per declared property |
| `InvalidNumber`, `NumberOverflow`, `NumberTooLong` | the [conversion](#the-conversion-rule) of a coordinate failed |
| `NonTriangularFace`, `FaceFieldCount`, `InvalidIndex`, `NegativeIndex`, `IndexOutOfBounds` | the [face rules](#faces) |

## The Conversion Rule

The rule `bunny-decimal-q32-32/1` (`DECIMAL_Q32_32_ID`) is implemented by
`parse_decimal_q32_32`. For decimal text `t` with exact rational value `v(t)`,

```text
raw(t) = round_half_even(v(t) * 2^32)
```

and the result is the Q32.32 value with that raw `i64`, or a typed error. Only
integer arithmetic is used: no floating point is read, written, or compared anywhere
in the implementation or the tests.

**Grammar.** `[+-] digits [. digits] [(e|E) [+-] digits]`, where `digits` may be
empty on one side of the point but not both (`5.`, `.5`, `-.5e1` are valid; `.`, `e5`,
`1e` are not). Leading zeros are allowed. Spaces, `nan`, `inf`, hexadecimal, digit
separators and non-ASCII digits are `Invalid`. A negative zero is zero.

**Rounding.** `round_half_even` is to the nearest integer; an exact tie goes to the
even neighbour, and the sign is applied to the magnitude, so the rule is symmetric
about zero (`-x` converts to `-raw(x)`, except at the end of the range below). Ties
exist: `(2n + 1) / 2^33` is exactly halfway between `n` and `n + 1` raw units.

**Overflow** is decided after rounding. The magnitude of the rounded value must be at
most `2^63 - 1` for a positive value and at most `2^63` for a negative one. So
`2147483647.9999999998` is `i64::MAX`, while `2147483647.9999999999` rounds up to `2^63`
and overflows; `-2147483648.0000000001` rounds back to `i64::MIN` and is accepted, and
`-2147483648.0000000002` overflows.

**Exact decision without reading every digit.** Write the value as `D * 10^e` where
`D` is the digit string with leading and trailing zeros removed, `s` its length, and
`m = s + e`, so that `10^(m-1) <= |v| < 10^m`. Then:

1. If `D` is empty, the value is zero, whatever the exponent.
2. If `m >= 11`, then `|v| >= 1e10 > 2^31`, so the result is `Overflow`, whatever the
   number of digits.
3. If `m <= -10`, then `|v| < 1e-10 < 2^-33` (half a raw unit), so the result is zero
   without a tie, whatever the number of digits.
4. Otherwise (`-9 <= m <= 10`) the value must be computed. If `s > 28` the result is
   `TooLong`: the significand of a 29-digit value does not fit the 128-bit working
   integers with the shift by 32. With `s <= 28` all intermediate values are below
   `2^125`: for `e >= 0` the raw value is `(D * 10^e) << 32` (and `e <= 9`), and for
   `e < 0` it is `(D << 32) / 10^-e` rounded half to even, with `-e <= 37`.

The exponent is read with saturation at `2^100`, so an exponent of any length is
decided by rules 1 to 3 and never overflows.

`TooLong` is therefore a statement about the reader, not about the value: an
exact conversion of those inputs exists but is not attempted. Rules 2 and 3 mean it
is never reported for a value that is certainly out of range or certainly zero.

## Verification

See [`test-plan.md`](test-plan.md). The conversion is checked against hand-derived
vectors (exact ties both ways, negative values, exponents, both ends of the range,
overflow and refusals), against an independent oracle that uses decimal-string
arithmetic (repeated doubling, digit shifts and digit comparison, with no division
and no machine-word arithmetic) on 20,000 seeded inputs, and across 2,000 consecutive
exact ties. One mutation of the tie rule was run and caught; see the pull request.
