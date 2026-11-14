# ASCII PLY Test Plan

This document defines how Bunny proves the ASCII PLY reader and the decimal to
Q32.32 conversion rule. The contract is [`README.md`](README.md).

## Scope

`parse_ascii_ply` and `parse_decimal_q32_32` in `bunny-codec`: the profile, the
line, whitespace and comment rules, the conversion rule, and the error contract.

## Test Goals

- Prove every conversion decision with hand-derived or exact-integer expected values.
- Prove the conversion against an oracle written differently from the code under test.
- Give every refusal its own typed error and line number.
- Keep the tests native and WASM-compatible (the allocation witness is native only).
- Use no floating point, including in expected values.

## Non-Goals

- Conversion to `f64` or `f32`: positions are Q32.32 only.
- Faces with more than three vertices, face properties other than `vertex_indices`,
  list properties on vertices, and binary or big-endian formats.

## Requirements

| ID | Requirement | Current Source |
| --- | --- | --- |
| APLY-REQ-001 | The profile accepts `format ascii 1.0`, a vertex element with `x`, `y`, `z` (float or double types, any order) and other scalar properties skipped by declaration, and a face element with one `vertex_indices` list; empty extra elements are ignored. | `README.md#the-profile` |
| APLY-REQ-002 | Coordinates convert by `bunny-decimal-q32-32/1`: the exact rational value times 2^32 rounded to nearest with ties to even, symmetric in sign, with integer arithmetic only. | `README.md#the-conversion-rule` |
| APLY-REQ-003 | The decimal grammar, overflow after rounding, certain overflow and zero by magnitude, and TooLong above 28 significant digits are typed errors. | `README.md#the-conversion-rule` |
| APLY-REQ-004 | Lines end at LF or CRLF, mixed freely; a lone CR is an ordinary byte; the `end_header` line needs LF; the last body line may lack one. | `README.md#line-endings` |
| APLY-REQ-005 | Words are split on spaces and tabs with leading and trailing ones ignored; header blank lines, `comment` and `obj_info` lines are ignored; body blank lines are skipped and body comments are refused. | `README.md#whitespace-blank-lines-and-comments` |
| APLY-REQ-006 | Header strictness: magic, format, element, count and property mistakes are typed errors. | `README.md#errors-and-line-numbers` |
| APLY-REQ-007 | Faces must be triangles with unsigned decimal indices below the vertex count. | `README.md#faces` |
| APLY-REQ-008 | Errors are typed, carry a 1-based line number, and display it. | `README.md#errors-and-line-numbers` |
| APLY-REQ-009 | The reader allocates nothing and never reserves memory from a declared count; the body must hold exactly the declared records. | `README.md#counts-and-allocation` |
| APLY-REQ-010 | The binary reader is unchanged. | `README.md#relation-to-the-binary-reader` |

```toml
[[requirement]]
id = "APLY-REQ-001"
summary = "The profile accepts `format ascii 1.0`, a vertex element with `x`, `y`, `z` (float or double types, any order) and other scalar properties skipped by declaration, and a face element with one `vertex_indices` list; empty extra elements are ignored."
status = "active"

[[requirement]]
id = "APLY-REQ-002"
summary = "Coordinates convert by `bunny-decimal-q32-32/1`: the exact rational value times 2^32 rounded to nearest with ties to even, symmetric in sign, with integer arithmetic only."
status = "active"

[[requirement]]
id = "APLY-REQ-003"
summary = "The decimal grammar, overflow after rounding, certain overflow and zero by magnitude, and TooLong above 28 significant digits are typed errors."
status = "active"

[[requirement]]
id = "APLY-REQ-004"
summary = "Lines end at LF or CRLF, mixed freely; a lone CR is an ordinary byte; the `end_header` line needs LF; the last body line may lack one."
status = "active"

[[requirement]]
id = "APLY-REQ-005"
summary = "Words are split on spaces and tabs with leading and trailing ones ignored; header blank lines, `comment` and `obj_info` lines are ignored; body blank lines are skipped and body comments are refused."
status = "active"

[[requirement]]
id = "APLY-REQ-006"
summary = "Header strictness: magic, format, element, count and property mistakes are typed errors."
status = "active"

[[requirement]]
id = "APLY-REQ-007"
summary = "Faces must be triangles with unsigned decimal indices below the vertex count."
status = "active"

[[requirement]]
id = "APLY-REQ-008"
summary = "Errors are typed, carry a 1-based line number, and display it."
status = "active"

[[requirement]]
id = "APLY-REQ-009"
summary = "The reader allocates nothing and never reserves memory from a declared count; the body must hold exactly the declared records."
status = "active"

[[requirement]]
id = "APLY-REQ-010"
summary = "The binary reader is unchanged."
status = "active"
```

## Test Cases

| ID | Category | Requirements | Oracle | Test |
| --- | --- | --- | --- | --- |
| APLY-TP-001 | Golden path | APLY-REQ-001, APLY-REQ-002 | Hand-derived raw coordinates for a Geordi-style mesh with confidence and intensity properties. | `crates/bunny-codec/tests/ply_ascii_tests.rs::aply_tp_001_geordi_style_mesh_skips_extra_properties_by_declaration` |
| APLY-TP-002 | Golden path, layout | APLY-REQ-001 | Exact raw values with extra properties first, middle and last and z, x, y declared in that order. | `crates/bunny-codec/tests/ply_ascii_tests.rs::aply_tp_002_coordinate_slots_follow_the_declared_order_and_types` |
| APLY-TP-003 | Line endings | APLY-REQ-004 | Identical results for LF, CRLF, mixed endings and an unterminated last line. | `crates/bunny-codec/tests/ply_ascii_tests.rs::aply_tp_003_lf_crlf_and_mixed_line_endings_are_accepted` |
| APLY-TP-004 | Line endings, negative | APLY-REQ-004, APLY-REQ-008 | Exact error kinds and lines for a lone CR in a body number, at the end of input, and in the header. | `crates/bunny-codec/tests/ply_ascii_tests.rs::aply_tp_004_a_lone_carriage_return_is_not_whitespace_or_a_line_end` |
| APLY-TP-005 | Whitespace, comments | APLY-REQ-005 | Exact results for a header and body full of blanks, tabs, comments and obj_info lines. | `crates/bunny-codec/tests/ply_ascii_tests.rs::aply_tp_005_blank_lines_comments_and_whitespace_follow_the_stated_rules` |
| APLY-TP-006 | Body comments, trailing data | APLY-REQ-005, APLY-REQ-008 | Exact error kinds and lines for a body comment, a record after the last, and a long face line. | `crates/bunny-codec/tests/ply_ascii_tests.rs::aply_tp_006_body_comments_and_content_after_the_records_are_refused` |
| APLY-TP-007 | Header refusals | APLY-REQ-006, APLY-REQ-008 | Exact error kind and line for twenty-two malformed headers. | `crates/bunny-codec/tests/ply_ascii_tests.rs::aply_tp_007_header_refusals_name_their_kind_and_line` |
| APLY-TP-008 | Property refusals | APLY-REQ-001, APLY-REQ-006, APLY-REQ-008 | Exact UnsupportedProperty and line for ten bad or repeated properties, and one before any element. | `crates/bunny-codec/tests/ply_ascii_tests.rs::aply_tp_008_property_refusals_name_their_kind_and_line` |
| APLY-TP-009 | Empty elements, empty mesh | APLY-REQ-001, APLY-REQ-004 | An empty extra element with properties is ignored; a zero-count mesh parses; an unterminated end_header is refused. | `crates/bunny-codec/tests/ply_ascii_tests.rs::aply_tp_009_empty_elements_are_ignored_with_their_properties` |
| APLY-TP-010 | Header encoding | APLY-REQ-005, APLY-REQ-008 | Invalid UTF-8 in the header is HeaderUtf8 at its line; UTF-8 comments and a non-ASCII body byte behave as stated. | `crates/bunny-codec/tests/ply_ascii_tests.rs::aply_tp_010_header_text_must_be_utf8_and_body_may_be_any_ascii` |
| APLY-TP-011 | Vertex refusals | APLY-REQ-002, APLY-REQ-003, APLY-REQ-008 | Exact error kind at line 13 for each vertex-line mistake, and uninterpreted extra properties. | `crates/bunny-codec/tests/ply_ascii_tests.rs::aply_tp_011_vertex_line_refusals_name_their_kind_and_line` |
| APLY-TP-012 | Face refusals | APLY-REQ-007, APLY-REQ-008 | Exact error kind at line 17 for fifteen malformed face lines, and the last valid index. | `crates/bunny-codec/tests/ply_ascii_tests.rs::aply_tp_012_face_refusals_name_their_kind_and_line` |
| APLY-TP-013 | Counts | APLY-REQ-009, APLY-REQ-008 | Exact errors for missing records and for counts of 4294967295 over short bodies. | `crates/bunny-codec/tests/ply_ascii_tests.rs::aply_tp_013_missing_records_and_huge_declared_counts_are_refused_without_allocating` |
| APLY-TP-014 | Error contract, siblings | APLY-REQ-008, APLY-REQ-010 | Exact Display text, std Error use, and mutual refusal of the two formats. | `crates/bunny-codec/tests/ply_ascii_tests.rs::aply_tp_014_errors_display_their_line_and_the_formats_refuse_each_other` |
| APLY-TP-015 | Iterators | APLY-REQ-001, APLY-REQ-009 | Iterators restart and yield exactly the declared counts. | `crates/bunny-codec/tests/ply_ascii_tests.rs::aply_tp_015_iterators_restart_and_yield_exactly_the_declared_counts` |
| APLY-TP-020 | Identity | APLY-REQ-002 | The rule identifier string. | `crates/bunny-codec/tests/ply_ascii_decimal_tests.rs::aply_tp_020_rule_has_a_stable_identity` |
| APLY-TP-021 | Golden path, conversion | APLY-REQ-002 | Exact raw values of dyadic decimals, derived as value times 2^32. | `crates/bunny-codec/tests/ply_ascii_decimal_tests.rs::aply_tp_021_whole_and_dyadic_values_are_exact` |
| APLY-TP-022 | Grammar | APLY-REQ-002, APLY-REQ-003 | Ten spellings of 1.2 agree on raw 5153960755; valid forms with an empty side of the point. | `crates/bunny-codec/tests/ply_ascii_decimal_tests.rs::aply_tp_022_grammar_forms_agree_on_one_value` |
| APLY-TP-023 | Exponents | APLY-REQ-002 | Hand-derived raw values with positive, negative and enormous exponents, and zero with any exponent. | `crates/bunny-codec/tests/ply_ascii_decimal_tests.rs::aply_tp_023_exponents_scale_by_powers_of_ten` |
| APLY-TP-024 | Rounding | APLY-REQ-002 | Hand-derived nearest-rounding vectors in raw units, both signs. | `crates/bunny-codec/tests/ply_ascii_decimal_tests.rs::aply_tp_024_rounding_is_to_nearest_with_ties_to_even` |
| APLY-TP-025 | Rounding, exact ties | APLY-REQ-002 | Exact ties built from 5^33 and one last digit either side, both signs. | `crates/bunny-codec/tests/ply_ascii_decimal_tests.rs::aply_tp_025_exact_ties_go_to_the_even_neighbour` |
| APLY-TP-026 | Range edges | APLY-REQ-002, APLY-REQ-003 | i64::MAX, i64::MIN, rounding back into range, the smallest nonzero value, and the first overflow. | `crates/bunny-codec/tests/ply_ascii_decimal_tests.rs::aply_tp_026_range_edges_are_exact` |
| APLY-TP-027 | Overflow | APLY-REQ-003 | Exact Overflow for out-of-range values, huge exponents and very long digit strings. | `crates/bunny-codec/tests/ply_ascii_decimal_tests.rs::aply_tp_027_values_outside_the_range_overflow` |
| APLY-TP-028 | Underflow | APLY-REQ-002, APLY-REQ-003 | Zero for values below 1e-10 however long, one raw unit just above half. | `crates/bunny-codec/tests/ply_ascii_decimal_tests.rs::aply_tp_028_tiny_values_round_to_zero_however_long` |
| APLY-TP-029 | TooLong | APLY-REQ-003 | 28 digits convert, 29 are TooLong, trailing zeros do not count. | `crates/bunny-codec/tests/ply_ascii_decimal_tests.rs::aply_tp_029_twenty_eight_significant_digits_convert_and_more_are_refused` |
| APLY-TP-030 | Invalid | APLY-REQ-003 | Exact Invalid for thirty malformed texts. | `crates/bunny-codec/tests/ply_ascii_decimal_tests.rs::aply_tp_030_text_outside_the_grammar_is_invalid` |
| APLY-TP-031 | Property, oracle | APLY-REQ-002, APLY-REQ-003 | 20,000 seeded inputs against an independent decimal-string-arithmetic oracle, all three outcomes reached. | `crates/bunny-codec/tests/ply_ascii_decimal_tests.rs::aply_tp_031_seeded_inputs_match_the_decimal_string_oracle` |
| APLY-TP-032 | Exhaustive ties | APLY-REQ-002 | 2,000 consecutive exact ties against the even-neighbour rule and the oracle. | `crates/bunny-codec/tests/ply_ascii_decimal_tests.rs::aply_tp_032_every_tie_in_a_range_goes_to_even` |
| APLY-TP-033 | Oracle check | APLY-REQ-002 | The oracle against hand-derived values. | `crates/bunny-codec/tests/ply_ascii_decimal_tests.rs::aply_tp_033_oracle_agrees_with_hand_derived_vectors` |
| APLY-TP-040 | Allocation | APLY-REQ-009 | Zero allocation blocks for parse and iteration after warm-up, and for a huge declared count. | `crates/bunny-codec/tests/ply_ascii_allocation_tests.rs::aply_tp_040_ascii_reader_allocates_zero_times_after_warm_up` |
| APLY-TP-050 | Sibling, unchanged | APLY-REQ-010 | The existing binary reader's golden test passes unchanged. | `crates/bunny-codec/tests/ply_tests.rs::parses_binary_triangle_ply_as_borrowed_view` |

```toml
[[case]]
id = "APLY-TP-001"
requirements = ["APLY-REQ-001", "APLY-REQ-002"]
evidence = "test"
test = "crates/bunny-codec/tests/ply_ascii_tests.rs::aply_tp_001_geordi_style_mesh_skips_extra_properties_by_declaration"
oracle = "Hand-derived raw coordinates for a Geordi-style mesh with confidence and intensity properties."
tier = "fast"
status = "implemented"

[[case]]
id = "APLY-TP-002"
requirements = ["APLY-REQ-001"]
evidence = "test"
test = "crates/bunny-codec/tests/ply_ascii_tests.rs::aply_tp_002_coordinate_slots_follow_the_declared_order_and_types"
oracle = "Exact raw values with extra properties first, middle and last and z, x, y declared in that order."
tier = "fast"
status = "implemented"

[[case]]
id = "APLY-TP-003"
requirements = ["APLY-REQ-004"]
evidence = "test"
test = "crates/bunny-codec/tests/ply_ascii_tests.rs::aply_tp_003_lf_crlf_and_mixed_line_endings_are_accepted"
oracle = "Identical results for LF, CRLF, mixed endings and an unterminated last line."
tier = "fast"
status = "implemented"

[[case]]
id = "APLY-TP-004"
requirements = ["APLY-REQ-004", "APLY-REQ-008"]
evidence = "test"
test = "crates/bunny-codec/tests/ply_ascii_tests.rs::aply_tp_004_a_lone_carriage_return_is_not_whitespace_or_a_line_end"
oracle = "Exact error kinds and lines for a lone CR in a body number, at the end of input, and in the header."
tier = "fast"
status = "implemented"

[[case]]
id = "APLY-TP-005"
requirements = ["APLY-REQ-005"]
evidence = "test"
test = "crates/bunny-codec/tests/ply_ascii_tests.rs::aply_tp_005_blank_lines_comments_and_whitespace_follow_the_stated_rules"
oracle = "Exact results for a header and body full of blanks, tabs, comments and obj_info lines."
tier = "fast"
status = "implemented"

[[case]]
id = "APLY-TP-006"
requirements = ["APLY-REQ-005", "APLY-REQ-008"]
evidence = "test"
test = "crates/bunny-codec/tests/ply_ascii_tests.rs::aply_tp_006_body_comments_and_content_after_the_records_are_refused"
oracle = "Exact error kinds and lines for a body comment, a record after the last, and a long face line."
tier = "fast"
status = "implemented"

[[case]]
id = "APLY-TP-007"
requirements = ["APLY-REQ-006", "APLY-REQ-008"]
evidence = "test"
test = "crates/bunny-codec/tests/ply_ascii_tests.rs::aply_tp_007_header_refusals_name_their_kind_and_line"
oracle = "Exact error kind and line for twenty-two malformed headers."
tier = "fast"
status = "implemented"

[[case]]
id = "APLY-TP-008"
requirements = ["APLY-REQ-001", "APLY-REQ-006", "APLY-REQ-008"]
evidence = "test"
test = "crates/bunny-codec/tests/ply_ascii_tests.rs::aply_tp_008_property_refusals_name_their_kind_and_line"
oracle = "Exact UnsupportedProperty and line for ten bad or repeated properties, and one before any element."
tier = "fast"
status = "implemented"

[[case]]
id = "APLY-TP-009"
requirements = ["APLY-REQ-001", "APLY-REQ-004"]
evidence = "test"
test = "crates/bunny-codec/tests/ply_ascii_tests.rs::aply_tp_009_empty_elements_are_ignored_with_their_properties"
oracle = "An empty extra element with properties is ignored; a zero-count mesh parses; an unterminated end_header is refused."
tier = "fast"
status = "implemented"

[[case]]
id = "APLY-TP-010"
requirements = ["APLY-REQ-005", "APLY-REQ-008"]
evidence = "test"
test = "crates/bunny-codec/tests/ply_ascii_tests.rs::aply_tp_010_header_text_must_be_utf8_and_body_may_be_any_ascii"
oracle = "Invalid UTF-8 in the header is HeaderUtf8 at its line; UTF-8 comments and a non-ASCII body byte behave as stated."
tier = "fast"
status = "implemented"

[[case]]
id = "APLY-TP-011"
requirements = ["APLY-REQ-002", "APLY-REQ-003", "APLY-REQ-008"]
evidence = "test"
test = "crates/bunny-codec/tests/ply_ascii_tests.rs::aply_tp_011_vertex_line_refusals_name_their_kind_and_line"
oracle = "Exact error kind at line 13 for each vertex-line mistake, and uninterpreted extra properties."
tier = "fast"
status = "implemented"

[[case]]
id = "APLY-TP-012"
requirements = ["APLY-REQ-007", "APLY-REQ-008"]
evidence = "test"
test = "crates/bunny-codec/tests/ply_ascii_tests.rs::aply_tp_012_face_refusals_name_their_kind_and_line"
oracle = "Exact error kind at line 17 for fifteen malformed face lines, and the last valid index."
tier = "fast"
status = "implemented"

[[case]]
id = "APLY-TP-013"
requirements = ["APLY-REQ-009", "APLY-REQ-008"]
evidence = "test"
test = "crates/bunny-codec/tests/ply_ascii_tests.rs::aply_tp_013_missing_records_and_huge_declared_counts_are_refused_without_allocating"
oracle = "Exact errors for missing records and for counts of 4294967295 over short bodies."
tier = "fast"
status = "implemented"

[[case]]
id = "APLY-TP-014"
requirements = ["APLY-REQ-008", "APLY-REQ-010"]
evidence = "test"
test = "crates/bunny-codec/tests/ply_ascii_tests.rs::aply_tp_014_errors_display_their_line_and_the_formats_refuse_each_other"
oracle = "Exact Display text, std Error use, and mutual refusal of the two formats."
tier = "fast"
status = "implemented"

[[case]]
id = "APLY-TP-015"
requirements = ["APLY-REQ-001", "APLY-REQ-009"]
evidence = "test"
test = "crates/bunny-codec/tests/ply_ascii_tests.rs::aply_tp_015_iterators_restart_and_yield_exactly_the_declared_counts"
oracle = "Iterators restart and yield exactly the declared counts."
tier = "fast"
status = "implemented"

[[case]]
id = "APLY-TP-020"
requirements = ["APLY-REQ-002"]
evidence = "test"
test = "crates/bunny-codec/tests/ply_ascii_decimal_tests.rs::aply_tp_020_rule_has_a_stable_identity"
oracle = "The rule identifier string."
tier = "fast"
status = "implemented"

[[case]]
id = "APLY-TP-021"
requirements = ["APLY-REQ-002"]
evidence = "test"
test = "crates/bunny-codec/tests/ply_ascii_decimal_tests.rs::aply_tp_021_whole_and_dyadic_values_are_exact"
oracle = "Exact raw values of dyadic decimals, derived as value times 2^32."
tier = "fast"
status = "implemented"

[[case]]
id = "APLY-TP-022"
requirements = ["APLY-REQ-002", "APLY-REQ-003"]
evidence = "test"
test = "crates/bunny-codec/tests/ply_ascii_decimal_tests.rs::aply_tp_022_grammar_forms_agree_on_one_value"
oracle = "Ten spellings of 1.2 agree on raw 5153960755; valid forms with an empty side of the point."
tier = "fast"
status = "implemented"

[[case]]
id = "APLY-TP-023"
requirements = ["APLY-REQ-002"]
evidence = "test"
test = "crates/bunny-codec/tests/ply_ascii_decimal_tests.rs::aply_tp_023_exponents_scale_by_powers_of_ten"
oracle = "Hand-derived raw values with positive, negative and enormous exponents, and zero with any exponent."
tier = "fast"
status = "implemented"

[[case]]
id = "APLY-TP-024"
requirements = ["APLY-REQ-002"]
evidence = "test"
test = "crates/bunny-codec/tests/ply_ascii_decimal_tests.rs::aply_tp_024_rounding_is_to_nearest_with_ties_to_even"
oracle = "Hand-derived nearest-rounding vectors in raw units, both signs."
tier = "fast"
status = "implemented"

[[case]]
id = "APLY-TP-025"
requirements = ["APLY-REQ-002"]
evidence = "test"
test = "crates/bunny-codec/tests/ply_ascii_decimal_tests.rs::aply_tp_025_exact_ties_go_to_the_even_neighbour"
oracle = "Exact ties built from 5^33 and one last digit either side, both signs."
tier = "fast"
status = "implemented"

[[case]]
id = "APLY-TP-026"
requirements = ["APLY-REQ-002", "APLY-REQ-003"]
evidence = "test"
test = "crates/bunny-codec/tests/ply_ascii_decimal_tests.rs::aply_tp_026_range_edges_are_exact"
oracle = "i64::MAX, i64::MIN, rounding back into range, the smallest nonzero value, and the first overflow."
tier = "fast"
status = "implemented"

[[case]]
id = "APLY-TP-027"
requirements = ["APLY-REQ-003"]
evidence = "test"
test = "crates/bunny-codec/tests/ply_ascii_decimal_tests.rs::aply_tp_027_values_outside_the_range_overflow"
oracle = "Exact Overflow for out-of-range values, huge exponents and very long digit strings."
tier = "fast"
status = "implemented"

[[case]]
id = "APLY-TP-028"
requirements = ["APLY-REQ-002", "APLY-REQ-003"]
evidence = "test"
test = "crates/bunny-codec/tests/ply_ascii_decimal_tests.rs::aply_tp_028_tiny_values_round_to_zero_however_long"
oracle = "Zero for values below 1e-10 however long, one raw unit just above half."
tier = "fast"
status = "implemented"

[[case]]
id = "APLY-TP-029"
requirements = ["APLY-REQ-003"]
evidence = "test"
test = "crates/bunny-codec/tests/ply_ascii_decimal_tests.rs::aply_tp_029_twenty_eight_significant_digits_convert_and_more_are_refused"
oracle = "28 digits convert, 29 are TooLong, trailing zeros do not count."
tier = "fast"
status = "implemented"

[[case]]
id = "APLY-TP-030"
requirements = ["APLY-REQ-003"]
evidence = "test"
test = "crates/bunny-codec/tests/ply_ascii_decimal_tests.rs::aply_tp_030_text_outside_the_grammar_is_invalid"
oracle = "Exact Invalid for thirty malformed texts."
tier = "fast"
status = "implemented"

[[case]]
id = "APLY-TP-031"
requirements = ["APLY-REQ-002", "APLY-REQ-003"]
evidence = "test"
test = "crates/bunny-codec/tests/ply_ascii_decimal_tests.rs::aply_tp_031_seeded_inputs_match_the_decimal_string_oracle"
oracle = "20,000 seeded inputs against an independent decimal-string-arithmetic oracle, all three outcomes reached."
tier = "fast"
status = "implemented"

[[case]]
id = "APLY-TP-032"
requirements = ["APLY-REQ-002"]
evidence = "test"
test = "crates/bunny-codec/tests/ply_ascii_decimal_tests.rs::aply_tp_032_every_tie_in_a_range_goes_to_even"
oracle = "2,000 consecutive exact ties against the even-neighbour rule and the oracle."
tier = "fast"
status = "implemented"

[[case]]
id = "APLY-TP-033"
requirements = ["APLY-REQ-002"]
evidence = "test"
test = "crates/bunny-codec/tests/ply_ascii_decimal_tests.rs::aply_tp_033_oracle_agrees_with_hand_derived_vectors"
oracle = "The oracle against hand-derived values."
tier = "fast"
status = "implemented"

[[case]]
id = "APLY-TP-040"
requirements = ["APLY-REQ-009"]
evidence = "test"
test = "crates/bunny-codec/tests/ply_ascii_allocation_tests.rs::aply_tp_040_ascii_reader_allocates_zero_times_after_warm_up"
oracle = "Zero allocation blocks for parse and iteration after warm-up, and for a huge declared count."
tier = "fast"
status = "implemented"

[[case]]
id = "APLY-TP-050"
requirements = ["APLY-REQ-010"]
evidence = "test"
test = "crates/bunny-codec/tests/ply_tests.rs::parses_binary_triangle_ply_as_borrowed_view"
oracle = "The existing binary reader's golden test passes unchanged."
tier = "fast"
status = "implemented"
```

## Fixtures

No checked-in fixtures: every input is a short text written in the test, and the
expected raw values are derived in comments. The real Stanford bunny PLY used as
evidence for the pull request is not copied into the repository.

## Oracle

The independent oracle in `ply_ascii_decimal_tests.rs` multiplies the digit string
by 2^32 with 32 decimal doublings, shifts by appending or dropping decimal digits,
and rounds by comparing the dropped digits with `5 * 10^(k-1)` as strings. It uses
no division, no shifts and no 128-bit arithmetic, so it shares no code path with the
reader. `APLY-TP-033` checks the oracle itself against hand-derived values.

## Mutation Check

The tie rule `quotient + (quotient & 1)` in `round_half_even_div` was mutated to
round half up and to round half down; `APLY-TP-025` and `APLY-TP-032` fail in both
cases and every other conversion test still passes, because random inputs almost
never land on an exact tie. Each mutant ran under `timeout 120`.

## Open Gaps

| Gap | Blocking item |
| --- | --- |
| Conversion to `f64` at a float boundary. | A consumer that needs it and states the rounding rule. |
| Values with more than 28 significant digits. | A consumer with such data; needs a wider working integer. |
