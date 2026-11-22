# Receipt: Pixel Path Inlining

Task:
Implement flyingrobots/bunny#271: `pixel/resolve_srgb_rgba8` got slower when unrelated code
was added to `bunny-optics`. Mark the small pixel-path functions `#[inline]` so the path's
cost no longer depends on the crate's codegen-unit split; change no output bit.

Files read:
crates/bunny-optics/src/pixel.rs
crates/bunny-optics/src/pixel/*.rs
crates/bunny-optics/tests/pixel_tests.rs
crates/bunny-perf/src/cases.rs
docs/topics/optics/README.md
docs/topics/performance/README.md

Files edited:
crates/bunny-optics/src/pixel.rs
crates/bunny-optics/src/pixel/srgb.rs
crates/bunny-optics/src/pixel/tone.rs
crates/bunny-optics/src/pixel/accumulate.rs
docs/topics/performance/README.md
CHANGELOG.md
.repo-respect/receipts/2026-11-14-pixel-path-inline.md

Topic docs:
docs/topics/performance/README.md (noise section, ledger entry)

Generated artifacts:
None.

Checks run:
All in the rust-dev container through devrun: cargo fmt --all -- --check; cargo clippy
--locked --workspace --all-targets --all-features -D warnings; cargo test --locked
--workspace; code-dojo --all; `perf --base origin/main --rounds 11 --align-code --exclusive`
runs (fix alone, three; fix merged with the lighting branch on a throwaway local branch,
two; lighting without the fix, one). Results are in the pull request and ledger entry 11.

Known risks:
With the lighting module present the pixel case is still about 7 % slower (median) than on
main, not within the 3 % noise of an idle run; the x86-64 reading comes from CI. No
objdump comparison of the symbols was made; the fix was chosen and verified by timing.

Human reviewer:
James Ross (maintainer); review pending.
