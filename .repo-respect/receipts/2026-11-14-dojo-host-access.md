# Receipt: Code Dojo Host Access Beyond Echo's List, And Doc Tests

Task:
Part of flyingrobots/bunny#321: decide which host accesses outside Echo's rule list join
Bunny's. `CODE_STANDARDS.md` Rule 4 says core crates never touch the filesystem, the
environment, process arguments or the current directory; the gate did not see `Path`
methods that read the filesystem (`exists`, `metadata`, ...), `std::os::<platform>::fs`,
`std::path::absolute`, terminal input, or doc tests. The first commit holds the planned
cases and the failing evidence; the second the rules, the doc-test scan and the
documentation, including where Bunny's list and Lumen's still differ.

Files read:
CODE_STANDARDS.md (Rules 4 and 6)
docs/CODE_DOJO.md
docs/topics/code-dojo-determinism/README.md
docs/topics/code-dojo-determinism/test-plan.md
xtask/src/nondeterminism.rs
xtask/src/nondeterminism/tests.rs
xtask/src/nondeterminism/std_api_1.96.0.txt
the std 1.96.0 documentation pages for std::io, std::os and its platform modules,
std::os::fd and std::path, fetched in the container from doc.rust-lang.org
the doc comments with code blocks in the deterministic crates (git grep)
Lumen at origin/main, read only through `git show`: docs/adr/0006-nondeterminism-gate.md

Files edited:
xtask/src/nondeterminism.rs
xtask/src/nondeterminism/doc_tests.rs (new)
xtask/src/nondeterminism/host_access_tests.rs (new)
xtask/src/nondeterminism/tests.rs
docs/CODE_DOJO.md
docs/topics/code-dojo-determinism/README.md
docs/topics/code-dojo-determinism/test-plan.md
CHANGELOG.md
.repo-respect/receipts/2026-11-14-dojo-host-access.md

Topic docs:
docs/topics/code-dojo-determinism/test-plan.md (ND-REQ-008, ND-TP-059..063)
docs/topics/code-dojo-determinism/README.md (rules, host access beyond Echo)
docs/CODE_DOJO.md (mapping, coverage count, Lumen alignment)

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
At the failing-evidence commit, cargo test --locked -p xtask nondeterminism with
`assert_rejects` made to report instead of panic: all 30 new forms slip through (Path
methods, platform modules, `absolute`, `pipe`, terminal input and five doc-test forms).

Known risks:
The rules are not in this commit; their tests fail here by design.

Human reviewer:
James Ross (maintainer); review pending.
