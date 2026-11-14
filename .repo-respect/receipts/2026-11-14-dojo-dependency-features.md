# Receipt: Code Dojo Dependency Check Over All Features And Build Edges

Task:
Implement flyingrobots/bunny#322: the dependency check ran `cargo tree` over the
deterministic crates' normal edges with the features the workspace enables, so a banned
crate behind an optional dependency nobody enables, or as a build-dependency, passed.
The check now lists the graph with the default features and with `--all-features`, over
normal and build edges. The first commit extracts the check into a function with no change
in behaviour and adds a failing fixture-workspace test; the second makes the change.

Files read:
docs/CODE_DOJO.md
docs/topics/code-dojo-determinism/README.md
docs/topics/code-dojo-determinism/test-plan.md
xtask/src/code_dojo.rs
xtask/src/nondeterminism.rs
xtask/src/nondeterminism/tests.rs
xtask/src/feature_configs.rs
every crates/bunny-*/Cargo.toml of the nine deterministic crates

Files edited:
xtask/src/code_dojo.rs
xtask/src/nondeterminism.rs
xtask/src/nondeterminism/dependency_graph_tests.rs (new)
docs/CODE_DOJO.md
docs/topics/code-dojo-determinism/README.md
docs/topics/code-dojo-determinism/test-plan.md
CHANGELOG.md
.repo-respect/receipts/2026-11-14-dojo-dependency-features.md

Topic docs:
docs/topics/code-dojo-determinism/test-plan.md (ND-REQ-005 restated, ND-TP-065)
docs/topics/code-dojo-determinism/README.md (dependency rules)
docs/CODE_DOJO.md (dependency check)

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
cargo test --locked -p xtask dependency at the failing-evidence commit: the fixture test
fails; the unchanged check reports nothing for `rand` behind an unused feature of
`bunny-num` or `serde_json` as a build-dependency of `bunny-mesh`.

Known risks:
The change is not in this commit; its test fails here by design.

Human reviewer:
James Ross (maintainer); review pending.
