# Receipt: Binary STL Import

Task:
flyingrobots/bunny#127: a deterministic binary STL import profile,
`bunny-binary-stl-triangle-mesh/1`, read by a new `parse_binary_stl` in `bunny-codec`. Exact
length rule (`84 + 50 n` in `u64`), typed refusals naming the triangle, vertex and axis of a
non-finite or out-of-range coordinate, coordinates converted by `FixedQ32_32::try_from_f32`
unchanged, stored vertex order kept with no welding, stored normal and attribute ignored.
Additive: no existing decoder, error type, fixture, golden, digest or ID changes.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*, .repo-respect/receipts/2026-11-23-shape-library.md
crates/bunny-codec/src/{lib.rs,ply.rs,ply/header.rs,ply_ascii.rs,ply_ascii/{body,error}.rs}
crates/bunny-codec/tests/{ply_ascii_limits_tests,ply_ascii_allocation_tests}.rs (conventions)
crates/bunny-codec/{Cargo.toml,README.md,tests/fixtures/README.md,tests/fixtures/stanford-bunny-ascii/README.md}
crates/bunny-num/src/fixed_q32_32.rs and fixed_q32_32/conversions.rs (try_from_f32)
docs/topics/{ascii-ply,fixed-point-rounding,release-packaging}/README.md, docs/NUMERIC_CONSTITUTION.md
xtask/src/{topic_docs.rs,code_dojo.rs} (ledger and lint rules)
flyingrobots/bunny#127

Files edited:
crates/bunny-codec/tests/stl_support/mod.rs (new: byte builders and seeded generator)
crates/bunny-codec/tests/{stl_tests,stl_oracle_tests,stl_allocation_tests}.rs (new)
docs/topics/binary-stl/README.md and test-plan.md (new)
.repo-respect/receipts/2026-11-23-binary-stl-import.md

Topic docs:
docs/topics/binary-stl/ is a new topic with its own STL- prefix (STL-REQ-001 to -007,
STL-TP-001 to -022 and -030); no other topic uses it.

Generated artifacts:
None.

Checks run:
First commit (tests and planned cases only): the STL tests fail to compile because
`parse_binary_stl` and its types do not exist yet. topic-docs and repo-respect check --branch.

Known risks:
None yet beyond the choices listed in the pull request.

Human reviewer:
James Ross (maintainer); review pending.
