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
docs/CODE_DOJO.md
docs/topics/code-dojo-determinism/README.md
docs/topics/code-dojo-determinism/test-plan.md
CHANGELOG.md
.repo-respect/receipts/2026-11-14-dojo-name-families.md

Topic docs:
docs/topics/code-dojo-determinism/test-plan.md (ND-REQ-007, ND-TP-051..057)
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

Known risks:
The rules are not in this commit; their tests fail here by design.

Human reviewer:
James Ross (maintainer); review pending.
