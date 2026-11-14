# Receipt: Macro-Built Paths, Prelude Macros And The Std Classification

Task:
Implement flyingrobots/bunny#347: five constructions from an independent review that bring
host access or unscanned code into a deterministic crate past the nondeterminism rules
(`std::time::$t::now()` in a macro, `include!`/`env!`/`option_env!` through the prelude
modules, a macro-built `$m!(..)`, `#[path = $p]` from a macro argument,
`Backtrace::capture`), plus a re-check of the std modules that read the host, with a rule
that an unclassified std module fails. The first commit holds the planned cases and the
failing tests; the second the rules and the documentation.

Files read:
CODE_STANDARDS.md
CHANGELOG.md
docs/CODE_DOJO.md
docs/topics/code-dojo-determinism/README.md
docs/topics/code-dojo-determinism/test-plan.md
docs/topics/lint-policy/README.md
xtask/src/code_dojo.rs
xtask/src/nondeterminism.rs and every file under xtask/src/nondeterminism/
xtask/src/lint_policy.rs, xtask/src/lint_policy/tests.rs
xtask/src/generated_check.rs
xtask/src/repo_respect.rs (receipt fields)
.repo-respect/receipts/2026-11-14-dojo-nondeterminism-rules.md
Lumen at origin/main, read only through `git show`: xtask/src/layout.rs,
xtask/src/nondeterminism/tokens.rs, xtask/src/nondeterminism/paths.rs; the bodies of
flyingrobots/lumen#155 to #158 and the open pull request flyingrobots/lumen#166
The std, core and alloc documentation index pages of Rust 1.96.0 and of `std::os`

Files edited:
xtask/src/nondeterminism.rs
xtask/src/nondeterminism/includes.rs
xtask/src/nondeterminism/metavariables.rs (new)
xtask/src/nondeterminism/std_modules.rs (new)
xtask/src/nondeterminism/std_api_1.96.0.txt (root modules and macros, std::os)
xtask/src/nondeterminism/macro_tests.rs (new)
xtask/src/nondeterminism/std_modules_tests.rs (new)
xtask/src/nondeterminism/tests.rs (rule family, waiver witnesses)
docs/CODE_DOJO.md
docs/topics/code-dojo-determinism/README.md
docs/topics/code-dojo-determinism/test-plan.md
CHANGELOG.md
.repo-respect/receipts/2026-11-14-dojo-macro-paths.md

Topic docs:
docs/topics/code-dojo-determinism/README.md (rules table; Macro Names And Metavariables;
The Std Classification)
docs/topics/code-dojo-determinism/test-plan.md (ND-REQ-009..011, ND-TP-071..079)
docs/CODE_DOJO.md (the #347 construction table, limits, the Lumen comparison, the
regeneration command, which reproduces the 1.96.0 snapshot exactly)

Generated artifacts:
None.

Checks run:
All inside the rust-dev container through devrun; nothing on the host.
The new tests against main's rule sources (main's `nondeterminism.rs` and `includes.rs`
with the new test module): 6 tests fail; of the 45 seeded forms 43 pass the old gate and
2 are reported only under another rule (`$root::time::Instant::now()` as `time-instant`,
`cfg_attr(.., path = $p)` as `include-source`). All five forms of the issue reproduce.
`cargo test --locked -p xtask` with the rules: 135 passed.
`cargo run --locked -p xtask -- code-dojo-rust --all` with the new rules on main's code:
no finding (no deterministic crate defines a macro; none uses `file!`, `Location`,
`std::os`, `std::arch` or `std::backtrace`), so no waiver was added.
Mutations, each under `timeout 120` after touching the sources, all caught, none timed
out: the metavariable scan off; each of its four positions off (after `::`, macro name,
method name, attributes); the `doc` exception removed; the `$crate` exception removed;
matchers scanned; standard macros only one segment below the root; imported macros not
reported; the unclassified rule off; the `std::backtrace`, `std::arch`, `Location` and
`std::os` rules removed; `host-platform` kept beside a specific rule; `file!` not
banned; `std::os::raw` not permitted (18 mutants).
The full gate, `code-dojo --all` with `--history origin/main..HEAD --network`, at the
head commit: see the pull request.

Review follow-up (CodeRabbit on flyingrobots/bunny#352, each verified against the code and
reproduced as a failing test before the fix): a function imported as `file` made a bare
`file!()` expand to that function's path and pass; a `use` of `option_env` under another
name inside a macro body passed; a glob of `std::os::fd` got `host-platform` (and, through
the `*` platform pattern, the `net` and `process` rules) beside `std-fs`. Bare macro calls
are now judged as the standard macro too, token paths in macro bodies get the imported-macro
check, `*` no longer matches `fd` or `raw`, and a glob below a specific rule drops
`host-platform`. 4 more mutants (one per fix), all caught; `cargo test -p xtask` 135 passed;
`code-dojo-rust --all` clean.

Known risks:
The metavariable rule bans positions, not paths, so a macro that uses `$t::MAX` safely
needs `<$t>::MAX` or a waiver; no deterministic crate has a macro today. A dependency's
`macro_rules!` is read only at its invocation. A standard macro re-exported by another
crate is reported only where that crate's `use` is written. The std classification is
of root names; below them the rules name banned items one by one, and only `std::time`,
`std::thread`, `Duration` and `Path` are checked item by item against the documentation.

Human reviewer:
James Ross (maintainer); review pending.
