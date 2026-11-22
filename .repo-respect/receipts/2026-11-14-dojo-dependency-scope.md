# Receipt: Deterministic Crates Depend Only On Scanned Or Reviewed Crates

Task:
Implement flyingrobots/bunny#349: a deterministic crate could depend on a workspace crate
outside the scanned list or on any crates.io crate not on a 14-name blocklist, and could
have a build script outside its directory (`build = "../../scripts/x.rs"`) that the gate
never scanned. The first commit holds the planned cases; the second the tests, the rules
and the documentation. Stacked on the pull request for flyingrobots/bunny#350.

Files read:
xtask/src/code_dojo.rs (the dependency check and the crate lists)
xtask/src/nondeterminism.rs, xtask/src/nondeterminism/tests.rs,
xtask/src/nondeterminism/dependency_graph_tests.rs
xtask/src/waivers.rs (manifest waivers)
Cargo.toml, every crates/*/Cargo.toml, deny.toml
docs/CODE_DOJO.md, docs/topics/code-dojo-determinism/README.md and test-plan.md
The open pull request flyingrobots/lumen#166 (Lumen's derived crate set)
The `cargo tree` listing of the nine deterministic crates on main, both feature sets

Files edited:
xtask/src/main.rs
xtask/src/code_dojo.rs
xtask/src/crate_manifests.rs (new)
xtask/src/crate_manifests/tests.rs (new)
xtask/src/nondeterminism.rs
xtask/src/nondeterminism/dependency_scope_tests.rs (new)
xtask/src/nondeterminism/dependency_graph_tests.rs
docs/CODE_DOJO.md
docs/topics/code-dojo-determinism/README.md
docs/topics/code-dojo-determinism/test-plan.md
CHANGELOG.md
.repo-respect/receipts/2026-11-14-dojo-dependency-scope.md

Topic docs:
docs/topics/code-dojo-determinism/README.md (Dependency Rules: the allow list, the
unscanned and unlisted rules, build scripts, targets, workspace members; Waivers)
docs/topics/code-dojo-determinism/test-plan.md (ND-REQ-014..015, ND-TP-090..094; ND-TP-065
extended with the issue's forms)
docs/CODE_DOJO.md (Dependency Check; the Lumen comparison)

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
`cargo tree --locked --edges normal,build --target all --no-dedupe --prefix depth --format
{p}` for the nine deterministic crates on main, with and without `--all-features`: the
only crates.io crates are `sha2`, `digest`, `block-buffer`, `crypto-common`,
`generic-array`, `typenum`, `version_check`, `cpufeatures`, `libc` and `cfg-if`, which
became the allow list; no workspace crate outside the set, no build script.
The new tests against the rules of the #350 branch: 3 fail. All ten listings give no
finding but the banned `fastrand`; the manifest waivers are not refused because nothing is
reported; the fixture workspace reports only `rand` and `serde_json`, not the workspace
member `zz-probe-crate` that `bunny-geom` depends on, not `build = "../../scripts/zz_build.rs"`
and not the members no list names. Both issue forms reproduce.
`cargo test --locked -p xtask`, sources touched: 154 passed, the fixture workspace through
real `cargo tree` included.
Mutations, each under `timeout 120` after touching the sources, all caught: the allow list
ignored; path crates accepted; a deterministic name accepted at any path; any source
accepted for an allowed name; the scope rules waivable; `build.rs` ignored; `build` paths
ignored; `..` out of the crate allowed; members unchecked; `dir/*` members not expanded;
the subtree below a deterministic crate not skipped (11 mutants).
The full gate, `code-dojo --all` with `--history origin/main..HEAD --network`, at the
head commit: see the pull request.

Known risks:
The allow list is by name for crates.io crates; a crate's version can change under it
through `cargo update`, which `cargo deny` and review see. Dev-dependencies are outside the
check. A path crate is recognised by the source `cargo tree` prints. Build scripts of
allow-listed registry crates (`generic-array`'s, through `version_check`) run on the host;
they are reviewed by being on the list.

Human reviewer:
James Ross (maintainer); review pending.
