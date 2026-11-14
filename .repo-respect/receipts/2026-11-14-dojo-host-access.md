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
xtask/src/nondeterminism/std_api_tests.rs
CODE_STANDARDS.md (Rule 4 names the rules that enforce it)
docs/CODE_DOJO.md
docs/topics/code-dojo-determinism/README.md
docs/topics/code-dojo-determinism/test-plan.md
CHANGELOG.md
.repo-respect/receipts/2026-11-14-dojo-host-access.md

Topic docs:
docs/topics/code-dojo-determinism/test-plan.md (ND-REQ-008, ND-TP-059..064)
docs/topics/code-dojo-determinism/README.md (rules, host access beyond Echo)
docs/CODE_DOJO.md (mapping, coverage count, Lumen alignment)

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
At the failing-evidence commit, cargo test --locked -p xtask nondeterminism with
`assert_rejects` made to report instead of panic: all 30 new forms slip through (Path
methods, platform modules, `absolute`, `pipe`, terminal input and five doc-test forms).

ND-TP-064 (every documented `Path` method gets its rule) was added with the
implementation.
Mutations, each under `timeout 120`, all caught: the `*` platform wildcard matching
nothing (1 test fails), the zero-argument condition dropped (the look-alike test), macro
tokens not searched for method calls (1), hidden doc-test lines kept hidden (1, after the
hidden-line case was changed to expect the hidden line itself: it first expected the
visible line below, and the mutant survived), `text` blocks scanned as Rust (the
look-alike test), doc-test findings not mapped to their own line (1), block doc comment
lines collapsed to the first (1).
cargo fmt --all; cargo clippy --locked -p xtask --all-targets --all-features -- -D warnings;
cargo test --locked -p xtask: 113 passed; 118 after merging the review fixes made in the
pull requests below this one (#329, #331, #332).
cargo run --locked -q -p xtask -- code-dojo-rust --all over the repository: clean. No
deterministic crate calls a filesystem `Path` method, names a platform module, reads
terminal input, or has a doc test that breaks a rule. Their sources hold three Rust doc
tests (bunny-broadphase `closest_hit`, bunny-geom `Ray3::try_new`, one in
bunny-optics `shading/hit.rs`), which produce no finding, and two `text` blocks in
bunny-optics, which are not read.
cargo run --locked -p xtask -- code-dojo --all with --history origin/main..HEAD --network
at the final commit.

Known risks:
Method calls are matched by name when they take no arguments, so a deterministic type
with an `exists()`, `metadata()` or `absolute()` method would be reported. Doc text from
`#[doc = include_str!(..)]` is not read. Writing to stdout and stderr stays allowed.

Human reviewer:
James Ross (maintainer); review pending.
