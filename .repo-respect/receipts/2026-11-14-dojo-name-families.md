# Receipt: Code Dojo Name Families From The Std Documentation

Task:
Part of flyingrobots/bunny#321: `std::thread::sleep_ms(1)` got past `thread-sleep`
because the rules matched whole-segment prefixes and missed sibling names, and float
arithmetic with no float token (`d.mul_f64(d.as_secs_f64())`,
`Duration::from_secs_f64(text.parse().ok()?)`) got past `float-boundary`. The time and
thread rules now name every documented item of `std::thread` and `std::time`
deliberately, from a snapshot of the 1.96.0 documentation tied by a test to
`rust-toolchain.toml`, and names ending in `_f32` or `_f64` are float boundaries in core.
The first commit holds the planned cases, the snapshot and the failing evidence; the
second the rules, the waivers they need on main, and the documentation.

Files read:
CODE_STANDARDS.md (Rules 2, 4, 6, 10)
docs/CODE_DOJO.md
docs/topics/code-dojo-determinism/README.md
docs/topics/code-dojo-determinism/test-plan.md
rust-toolchain.toml
xtask/src/code_dojo.rs
xtask/src/nondeterminism.rs
xtask/src/nondeterminism/tests.rs
xtask/src/nondeterminism/evasion_tests.rs
the std 1.96.0 documentation pages for std::thread, std::time, Duration and Path, fetched
in the container from doc.rust-lang.org
Lumen at origin/main, read only through `git show`: docs/adr/0006-nondeterminism-gate.md

Files edited:
xtask/src/nondeterminism.rs
xtask/src/nondeterminism/std_api_1.96.0.txt (new)
xtask/src/nondeterminism/std_api_tests.rs (new)
xtask/src/nondeterminism/evasion_tests.rs
xtask/src/code_dojo.rs
xtask/src/waivers.rs
xtask/src/waivers/tests.rs
crates/bunny-geom/src/conversions.rs (two waiver comments, no code)
crates/bunny-linalg/src/fixed_vec2.rs (two waiver comments, no code)
crates/bunny-linalg/src/fixed_vec3.rs (two waiver comments, no code)
docs/CODE_DOJO.md
docs/topics/code-dojo-determinism/README.md
docs/topics/code-dojo-determinism/test-plan.md
CHANGELOG.md
.repo-respect/receipts/2026-11-14-dojo-name-families.md

Topic docs:
docs/topics/code-dojo-determinism/test-plan.md (ND-REQ-007, ND-TP-051..058, open gaps)
docs/topics/code-dojo-determinism/README.md (rules)
docs/CODE_DOJO.md (mapping, coverage count, keeping the name lists current)

Generated artifacts:
xtask/src/nondeterminism/std_api_1.96.0.txt, the item lists of the std 1.96.0
documentation, produced by the command recorded in docs/CODE_DOJO.md.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
At the failing-evidence commit, cargo test --locked -p xtask nondeterminism with
`assert_rejects` made to report instead of panic: all 10 new forms slip through; 14 of the
58 path and `use` probes of documented `std::thread`/`std::time` items get the wrong rule
(`sleep_ms`, `sleep_until`, `park`, `park_timeout`, `park_timeout_ms`, `yield_now`,
`SystemTimeError`); all 28 probes of the 14 float methods of `Duration` are unseen. The
inferred-float gap case passes, as it must.

Mutations, each under `timeout 120`, all caught: `sleep_ms` dropped from the rules (2
tests fail), no float suffixes (2), float method names not checked (2), a node starting
only on its first line (1).
The documented regeneration command in docs/CODE_DOJO.md, run in the container with
--network, reproduces std_api_1.96.0.txt byte for byte below its three comment lines.
cargo fmt --all; cargo clippy --locked -p xtask --all-targets --all-features -- -D warnings;
cargo test --locked -p xtask: 107 passed.
cargo run --locked -q -p xtask -- code-dojo-rust --all over the repository: 20 new
findings, all `float-boundary`. 8 were inside functions that already had a waiver placed
between their doc comment and `fn`, which the waiver scope did not reach: a waiver bug,
fixed here (a node also starts on its first line after its attributes, ND-TP-058). 12 on
6 items are real and were invisible before because no float token appears:
`From<Vec2>`/`From<FixedVec2>` and `From<Vec3>`/`From<FixedVec3>` in bunny-linalg and
`fixed_scalar_from_float` and `From<FixedSphere3> for Sphere3` in bunny-geom convert to
and from the f32 DTO types through `from_f32`/`try_from_f32`/`to_f32`. Each item now
carries a waiver with its reason; no code changed. After that the scan is clean.
cargo run --locked -p xtask -- code-dojo --all with --history origin/main..HEAD --network
at the final commit.

Known risks:
A float whose type is only inferred, with no float token, float-suffixed name or literal
on the way, is beyond a syntactic gate (pinned as ND-TP-053); `-D clippy::float_arithmetic`
on the core crates would close the arithmetic part and is proposed, not enforced. A name
ending in `_f32`/`_f64` that does not involve a float is reported; none exists on main.
The std lists cover `std::thread` and `std::time` (and `Duration`'s methods); other
banned modules are banned whole and need no item list.

Human reviewer:
James Ross (maintainer); review pending.
