# Receipt: Code Dojo Included Files And Compile-Time Environment

Task:
Part of flyingrobots/bunny#321: `include!("payload.inc")` and `#[path = "payload.inc"] mod`
brought code the gate never scanned into a deterministic crate; `#[path]` could also
leave the crate directory; `include_str!`/`include_bytes!` could read host files outside
the crate; `env!` and `option_env!` read the build environment and were accepted as a
look-alike. The first commit holds the planned cases and the failing evidence; the second
the rules, the waivers they need on main, and the documentation.

Files read:
docs/CODE_DOJO.md
docs/topics/code-dojo-determinism/README.md
docs/topics/code-dojo-determinism/test-plan.md
xtask/src/code_dojo.rs
xtask/src/nondeterminism.rs
xtask/src/nondeterminism/tests.rs
xtask/src/nondeterminism/evasion_tests.rs
crates/bunny-contract/tests/generated_version_tests.rs
every `include!`, `include_str!`, `include_bytes!`, `#[path]`, `env!` and `option_env!` in
the deterministic crates (git grep)
Lumen at origin/main, read only through `git show`: docs/adr/0006-nondeterminism-gate.md

Files edited:
xtask/src/nondeterminism.rs
xtask/src/nondeterminism/includes.rs (new)
xtask/src/nondeterminism/tests.rs
xtask/src/nondeterminism/evasion_tests.rs
xtask/src/code_dojo.rs
crates/bunny-contract/tests/generated_version_tests.rs (waiver comments, no code)
docs/CODE_DOJO.md
docs/topics/code-dojo-determinism/README.md
docs/topics/code-dojo-determinism/test-plan.md
CHANGELOG.md
.repo-respect/receipts/2026-11-14-dojo-included-files.md

Topic docs:
docs/topics/code-dojo-determinism/test-plan.md (ND-TP-047..050; ND-TP-021 and -034 restated)
docs/topics/code-dojo-determinism/README.md (rules, compile-time file access)
docs/CODE_DOJO.md (mapping table, coverage count, Lumen alignment)

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
rustc 1.96.0 --edition 2021 --crate-type lib on ten of the new forms, including
`include!(concat!(..))`, `core::include!`, a renamed `include`, `#[path]` and
`cfg_attr(.., path = ..)` to a non-.rs file, a renamed `include_str` and `std::env!`: all
compile.
cargo test --locked -p xtask nondeterminism at the failing-evidence commit with
`assert_rejects` made to report instead of panic: 23 of the 24 new forms slip through;
only `std::env!` is caught, through the `std::env` prefix.

Known risks:
The rules are not in this commit; their tests fail here by design.

Human reviewer:
James Ross (maintainer); review pending.
