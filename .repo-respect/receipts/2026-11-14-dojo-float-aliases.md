# Receipt: Float Aliases And Test Code By The Module Tree

Task:
Implement flyingrobots/bunny#348: `float-boundary` missed the C float aliases
(`core::ffi::c_double`, `c_float`, `std::os::raw`) and type aliases of a float, and any
path with a `tests` component was treated as test code even when `pub mod tests;` compiled
it into the library. The first commit holds the planned cases; the second the tests, the rules, the waivers they needed on main and the documentation. Stacked on
flyingrobots/bunny#352 (#347).

Files read:
xtask/src/code_dojo.rs
xtask/src/nondeterminism.rs and xtask/src/nondeterminism/tests.rs
crates/bunny-num/src/lib.rs, crates/bunny-geom/src/lib.rs,
crates/bunny-geom/src/conversions.rs, crates/bunny-linalg/src/lib.rs
crates/bunny-broadphase/src/bvh/sah.rs, crates/bunny-optics/src/shading/vertex_normals.rs
docs/CODE_DOJO.md
docs/topics/code-dojo-determinism/README.md
docs/topics/code-dojo-determinism/test-plan.md
Lumen at origin/main, read only through `git show`: xtask/src/layout.rs and commit b046eb6
(flyingrobots/lumen#124), the bodies of flyingrobots/lumen#156

Files edited:
xtask/src/main.rs
xtask/src/code_dojo.rs
xtask/src/float_aliases.rs (new)
xtask/src/module_tree.rs (new)
xtask/src/module_tree/tests.rs (new)
xtask/src/nondeterminism.rs (test module registration)
xtask/src/nondeterminism/float_alias_tests.rs (new)
crates/bunny-num/src/lib.rs, crates/bunny-geom/src/lib.rs,
crates/bunny-geom/src/conversions.rs, crates/bunny-linalg/src/lib.rs (waiver comments only)
docs/CODE_DOJO.md
docs/topics/code-dojo-determinism/README.md
docs/topics/code-dojo-determinism/test-plan.md
CHANGELOG.md
.repo-respect/receipts/2026-11-14-dojo-float-aliases.md

Topic docs:
docs/topics/code-dojo-determinism/README.md (rules table; Scope: which files are test
code, which names are floats)
docs/topics/code-dojo-determinism/test-plan.md (ND-REQ-012..013, ND-TP-080..089)
docs/CODE_DOJO.md (the #348 construction table, what it found on main, limits, Lumen)

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
The new float tests against #347's `code_dojo.rs`: of the 17 alias forms 2 pass
(`core::primitive::f64` and `std::primitive::f32`, through their `f64`/`f32` segment);
the issue's `src/tests/float.rs` with `f64` and `v[0]` gets no finding. Both issue forms
reproduce.
`cargo test --locked -p xtask` with the rules, sources touched first: 145 passed. (One
earlier run reported a stale failure and one a stale pass from the container's build cache;
every run since touches the sources.)
`cargo run --locked -p xtask -- code-dojo-rust --all` with the rules on main's code: 18
`float-boundary` findings on 15 lines, every one a use of the f32 DTO alias `Scalar`
(`bunny_num::is_finite`; `Vec2`, `Vec3` and their constructors in `bunny-linalg`;
`Sphere3`, `validate_sphere3`, which compares two floats, and `Sphere3::try_new` in
`bunny-geom`; three imports). All are the documented DTO boundary; each item has a
waiver with its reason. The module tree changed no category on main. After the waivers:
clean.
Mutations, each under `timeout 120` after touching the sources, all caught, none timed
out: `c_double` and `f16` not float names; file aliases off; workspace aliases ignored;
a file's own non-float name not shadowing; one pass instead of a fixed point (survived
until the alias declared after its use was added); the gate ignoring the tree; `cfg(test)`
on a declaration ignored; `all(..)` not followed; library code not outranking test code;
the old `tests`-component rule; unreached `tests/` files not test code; inline modules
adding no directory; `#[path]` files not `mod.rs`-like; a file's own `#![cfg(test)]`
ignored (15 mutants).
The full gate, `code-dojo --all` with `--history origin/main..HEAD --network`, at the
head commit: see the pull request.

Known risks:
Aliases are matched by name across the core crates, not resolved: an unrelated item of
the same name in another core file over-reports (a waiver resolves it). A float alias
declared outside the core crates is not seen. The module walk does not follow a `#[path]`
under `cfg_attr` (the nondeterminism rules already report it), treats an unreachable file
under `src/` as library code, and in the pre-commit gate reads the tree from the index.

Human reviewer:
James Ross (maintainer); review pending.
