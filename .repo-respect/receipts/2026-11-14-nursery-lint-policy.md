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
Planned; see the pull request.

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

Known risks:
Planned; see the pull request.

Human reviewer:
James Ross (maintainer); review pending.
