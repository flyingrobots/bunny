# Receipt: Rule 10 In The Core Crates, Enforced

Task:
flyingrobots/bunny#284. Rule 10 of `CODE_STANDARDS.md` bans blanket-denying `clippy::nursery`,
but six core crates denied the whole group. Replace each blanket deny with the nursery lints
the crate relies on, named with reasons; add a Code Dojo check, with seeded violations, that
holds the core crates to the rule; and reduce the stale second copy of the standards to a
pointer to the canonical file.

Files read:
CODE_STANDARDS.md (Rule 10), docs/RUST_CODE_STANDARDS.editors-edition.md
crates/bunny-optics/src/lib.rs and .repo-respect/receipts/2026-11-14-optics-typed-refusals.md
crates/*/src/lib.rs and crates/*/Cargo.toml of the core crates, Cargo.toml
xtask/src/code_dojo.rs, xtask/src/nondeterminism.rs, xtask/src/waivers.rs, xtask/src/main.rs
docs/CODE_DOJO.md, docs/README.md, CONTRIBUTING.md, docs/PROCESS.md

Files edited:
xtask/src/lint_policy.rs, xtask/src/lint_policy/tests.rs (new), xtask/src/main.rs,
xtask/src/code_dojo.rs (check_lint_policy in the full and pre-commit gates)
crates/{bunny-num,bunny-linalg,bunny-geom,bunny-query,bunny-broadphase,bunny-mesh}/src/lib.rs
CODE_STANDARDS.md (Rule 10: named lints, the check), docs/CODE_DOJO.md, docs/README.md
docs/RUST_CODE_STANDARDS.editors-edition.md (reduced to a pointer)
docs/topics/lint-policy/README.md and test-plan.md (new), CHANGELOG.md (Unreleased)

Topic docs:
docs/topics/lint-policy/ (new: README.md, test-plan.md)

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
Before: with the blanket deny removed from the six core crates (and bunny-codec), `cargo clippy
-p <crate> --lib --all-features -- -W clippy::nursery` gave no finding in any library; the
crates' integration tests, which the blanket deny never covered, gave missing_const_for_fn in
every crate, redundant_pub_crate in bunny-linalg, bunny-query and bunny-broadphase, and
suboptimal_flops in bunny-num.
Failing evidence: LINT-TP-003 failed on the unchanged crates, naming line 4 of the six lib.rs
files; LINT-TP-001 and -002 (seeded attributes and manifests) passed from the start.
After: cargo clippy --workspace --all-targets --all-features -D warnings clean; cargo test -p
xtask 122 passed. Seeded lint checks (LINT-TP-004): in each of the six crates a seed for each
named lint, appended to lib.rs, fails Clippy naming that lint (18 of 18). A control run (the
use_self seed with use_self removed) still failed on another lint, so it shows nothing and is
not recorded as evidence.
Nothing links to docs/RUST_CODE_STANDARDS.editors-edition.md; the only text unique to it was
the earlier wording of the receipt rule, which CODE_STANDARDS.md states in full.
cargo fmt --check; code-dojo --all at the final commit (see the PR).

Known risks:
The choice of the three named lints is a judgement, recorded in the chapter: no core library
had any nursery finding, so the evidence is that the code follows these patterns, not that
they caught a defect. The manifest scan is line-based and does not read inline-table lint
forms or command-line lint flags. bunny-codec still denies the group; Rule 10 covers only
core crates. The `#[allow(clippy::module_name_repetitions)]` attributes in
bunny-broadphase's lib.rs carry no reason, as before (not changed here).

Human reviewer:
James Ross (maintainer); review pending.
