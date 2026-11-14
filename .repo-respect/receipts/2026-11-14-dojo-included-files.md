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

Mutations, each under `timeout 120`, all caught: every path inside the crate (2 tests
fail), backslashes accepted (1), any `#[path]` extension accepted (1), `cfg_attr` not
read (1), macro tokens not searched for calls (3), any module path counted as a standard
macro (the look-alike test).
Review findings (CodeRabbit on #332, verified here): `#[path]` emitted by a macro, a
`path` inside a nested `cfg_attr`, and a `.rs` target under `vendor`, `third_party` or
`target`, which the gate does not list, all escaped. ND-TP-069 seeds five forms; all
five slipped through on the unfixed code. `#[path]` attributes are now also read from
macro tokens, `cfg_attr` is parsed as nested attributes at any depth, and a module path
must name a file the gate lists. Mutations, under `timeout 120`, all caught: macro tokens
not searched for path attributes, `cfg_attr` read one level deep, unlisted directories
accepted. A follow-up review note showed that only the first `path` inside a `cfg_attr`
was checked, though the compiler may drop it and keep a later one; every `path` is now
checked (a sixth ND-TP-069 form, red first), and the mutation that checks only the first
fails it.
cargo fmt --all; cargo clippy --locked -p xtask --all-targets --all-features -- -D warnings;
cargo test --locked -p xtask: 103 passed; cargo test -p bunny-contract --test
generated_version_tests: 2 passed.
cargo run --locked -q -p xtask -- code-dojo-rust --all over the repository: three new
findings, all real and all in crates/bunny-contract/tests/generated_version_tests.rs:
`env!("CARGO_PKG_VERSION")` (`std-env`) and two `include_str!` of the committed
generator outputs under `generated/` (`include-data`). Each now has a waiver with its
reason; after that the scan is clean. The five `#[path = "support/*.rs"]` test modules
and the four `include_str!`/`include_bytes!` fixtures in bunny-codec are inside their
crates and pass.
cargo run --locked -p xtask -- code-dojo --all with --history origin/main..HEAD --network
at the final commit.

Known risks:
Paths resolve lexically: symbolic links are not followed, and inside an inline module
the compiler resolves `#[path]` against a deeper directory, which can only make the gate
report an inside path as outside. A user macro named `include`, `env` or `option_env`
called by its bare name is reported as the standard one.

Human reviewer:
James Ross (maintainer); review pending.
