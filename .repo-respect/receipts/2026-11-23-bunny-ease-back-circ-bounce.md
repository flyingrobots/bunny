# Receipt: Bunny Ease Back, Circ and Bounce

Task:
flyingrobots/bunny#305: `bunny-ease/1`, part 2, in `bunny-num`. `ease::back_in`, `back_out`
and `back_in_out(overshoot, t)`, `circ_in`, `circ_out` and `circ_in_out(t)`, and `bounce_in`,
`bounce_out` and `bounce_in_out(t)`, with `BACK_DEFAULT_OVERSHOOT` (GSAP's 1.70158, raw
7 308 230 452). Back and bounce are GSAP's definitions as exact rationals rounded once to
nearest, ties to even. Circ takes the floor of an exact 128-bit root, with nothing rounded before it: `circ_in` is
`2^32 - isqrt(2^64 - t_raw^2)`, `circ_out` is its reflection, and `circ_in_out` is
`2^31 - isqrt(2^62 - t_raw^2)` below `1/2` and reflected from there. Each is within one raw
unit on a stated side. "Out"
and "in-out" are reflections, so the symmetries hold bit for bit. Additive: no part-1 item,
golden, digest or ID changes.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, .githooks/*, xtask/src/code_dojo.rs (limits),
xtask/src/topic_docs.rs (statuses)
.repo-respect/receipts/2026-11-23-ray-footprint.md, 2026-11-23-bunny-ease-power.md
crates/bunny-num/src/{lib.rs,ease.rs,fixed_q32_32.rs}, crates/bunny-num/tests/ease_tests.rs
crates/bunny-num/README.md, CHANGELOG.md, docs/README.md
docs/topics/ease/README.md and test-plan.md
flyingrobots/bunny#305 and pull request #455 (part 1, with its Python model)

Files edited:
crates/bunny-num/src/ease/back_circ_bounce.rs (new)
crates/bunny-num/src/ease.rs (one `mod` line and the re-exports, at the end)
crates/bunny-num/tests/ease_back_circ_bounce_tests.rs (new)
crates/bunny-num/README.md, CHANGELOG.md, docs/README.md (the ease row)
docs/topics/ease/README.md (Back, Circ and Bounce)
docs/topics/ease/test-plan.md (scope; EASE-REQ-010 to -014; EASE-TP-020 to -031; Part 2
Oracles; Part 2 Mutation Checks)
.repo-respect/receipts/2026-11-23-bunny-ease-back-circ-bounce.md

Topic docs:
docs/topics/ease/README.md and test-plan.md. IDs are in the range assigned to this task
(EASE-REQ-010 to -019, EASE-TP-020 to -039); the sine eases (flyingrobots/bunny#306) use
EASE-REQ-020 on and EASE-TP-040 on.

Generated artifacts:
None. The Python model that printed the pinned digests and golden values is posted on the pull
request, not committed.

Checks run:
First commit (tests and planned cases): cargo test --locked -q -p bunny-num --test
ease_back_circ_bounce_tests fails to compile (the kernel does not exist); topic-docs clean.
Kernel commit, each its own command on the host (local Rust 1.100.0; CI uses the pinned 1.96.0):
cargo fmt --all --check; cargo test --locked --workspace --all-targets --all-features (119
test result lines, every one `ok`); cargo build and cargo test -p bunny-num
--no-default-features (native host, all `ok`); cargo clippy --locked -p bunny-num
--all-targets --all-features -- -D warnings (clean); cargo run --locked -p xtask --
code-dojo-generated; code-dojo-rust --all; code-dojo-determinism --enforce;
code-dojo-perf-groups; topic-docs; repo-respect check --branch. All clean. Workspace-wide
clippy stops on a 1.100.0-only lint in crates/bunny-perf/src/memory.rs, a file this change does
not touch.
Mutations: fifteen, each applied, run with cargo test --locked -q -p bunny-num --no-fail-fast
--test ease_back_circ_bounce_tests, and restored; fourteen fail at least one case and one is
proved equivalent (the test plan's table).
Not run locally: wasm-pack (CI's WebAssembly job runs it), code-dojo-no-std (needs rustup; the
bare-metal build runs only in CI), cargo deny and markdownlint-cli2 (CI's Dojo job runs them).

Known risks:
Circ is a floor, not a nearest rounding, as the issue specifies: its bound is one raw unit on a
stated side, not half a unit. Bounce's "continuity within 1 raw unit" is stated against the
tangent from the common breakpoint value; adjacent raw outputs can differ by up to 6 raw units
there because the slope reaches 5.5. Back accepts every overshoot, which the issue left open.

Human reviewer:
James Ross (maintainer); review pending.
