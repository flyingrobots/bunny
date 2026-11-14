# Receipt: Echo Nondeterminism Rules In Code Dojo

Task:
Implement flyingrobots/bunny#279: map every rule of Echo's `ban-nondeterminism` scan onto
Code Dojo, add the rules Bunny lacked as syntax-aware Code Dojo rules with Echo's rule IDs,
and document the mapping, keeping one mechanism in Bunny. The first commit holds the
planned cases and the failing evidence; the second the rules, the waivers they needed on
`main`, and the documentation.

Files read:
CODE_STANDARDS.md
CONTRIBUTING.md
CHANGELOG.md
rustfmt.toml
clippy.toml
deny.toml
Cargo.toml and every crates/*/Cargo.toml
.githooks/pre-commit
.githooks/pre-push
.githooks/commit-msg
scripts/install-githooks.sh
.github/workflows/code-dojo.yml
docs/CODE_DOJO.md
docs/PROCESS.md
docs/README.md
docs/topics/repo-respect/README.md
docs/topics/repo-respect/test-plan.md
xtask/src/main.rs
xtask/src/code_dojo.rs
xtask/src/topic_docs.rs
crates/bunny-num/src/fixed_q32_32/conversions.rs
crates/bunny-num/tests/fixed_q32_32_tests.rs
crates/bunny-contract/tests/generated_version_tests.rs
.repo-respect/receipts/2026-06-17-core-float-boundaries.md
.repo-respect/receipts/2026-06-17-float-literal-gate.md
.repo-respect/receipts/2026-06-17-map-constructor-ban.md
.repo-respect/receipts/2026-06-17-std-fs-ambient-ban.md
Echo at origin/main, read only through `git show`: scripts/ban-nondeterminism.sh,
scripts/lib/determinism-scan.sh, .ban-nondeterminism-allowlist,
docs/adr/ADR-0006-Ban-Non-Determinism.md, .githooks/pre-push-parallel,
.github/workflows/ci.yml, .github/workflows/det-gates.yml

Files edited:
xtask/src/main.rs
xtask/src/code_dojo.rs
xtask/src/nondeterminism.rs
xtask/src/nondeterminism/tests.rs
crates/bunny-num/src/fixed_q32_32/conversions.rs (three waiver comments, no code)
docs/CODE_DOJO.md
docs/README.md
docs/topics/code-dojo-determinism/README.md
docs/topics/code-dojo-determinism/test-plan.md
CODE_STANDARDS.md
CHANGELOG.md
.repo-respect/receipts/2026-11-14-dojo-nondeterminism-rules.md

Topic docs:
docs/topics/code-dojo-determinism/README.md (rules, path resolution, dependency rules,
scope, waivers, where the rules run)
docs/topics/code-dojo-determinism/test-plan.md (ND-REQ-001..005, ND-TP-001..029)
docs/CODE_DOJO.md (Nondeterminism Rules: scope, mapping table with Echo's rule IDs,
coverage before the change, dependency check, where it runs)

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
cargo test --locked -p xtask nondeterminism at the failing-evidence commit (pinned with
--commit): 25 of 29 fail; the messages record form by form what the earlier gate reported,
which is the "Before" column of the mapping table.
cargo test --locked -p xtask after the rules: 76 passed.
cargo run --locked -p xtask -- code-dojo-rust --all on main's code with the new rules:
5 findings. 3 real: `f32::from_bits` in `bunny_num::fixed_q32_32::to_f32`, the documented
float egress, more than two lines below the function's existing waiver; given waivers
with reasons. 2 false positives: `std::collections::hash_map::DefaultHasher` in
`crates/bunny-num/tests/fixed_q32_32_tests.rs`, which has fixed SipHash keys; the rule now
permits `DefaultHasher`, with a look-alike test.
cargo tree for the nine deterministic crates: no banned crate in any normal graph.
Mutations, each under `timeout 120`, all caught, none timed out: ignoring `use` groups
(9 tests fail), not skipping another deterministic crate's dependency subtree (3 fail),
not scanning macro bodies (10 fail).
cargo fmt --all -- --check; cargo clippy --locked -p xtask --all-targets --all-features
-- -D warnings.
cargo run --locked -p xtask -- code-dojo --all with --history origin/main..HEAD at the
final commit.

Known risks:
The import map is file-wide, not per module; it can only over-report. A banned type that
reaches a file only through another crate's inferred return type is invisible to the
source rules; the dependency check covers third-party crates. `float-op` is enforced by
banning floats, not float method names, so a float admitted by a waiver can call float
methods more than two lines below it. The hooks run only where
`scripts/install-githooks.sh` was run; CI is the gate that always applies.

Human reviewer:
James Ross (maintainer); review pending.
