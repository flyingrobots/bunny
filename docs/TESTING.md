# Bunny — Testing Doctrine

Determinism is paramount. This document defines the testing strategies used to enforce bit-level computational reproducibility across platforms.

---

## 1. Bit-Level Determinism Assertions

Because float representations differ across host FPUs, compilers, and hardware execution lanes (e.g., Fused Multiply-Add instruction differences), Bunny uses software-defined fixed-point arithmetic (`FixedQ32_32`).

* **Exact Parity**: Tests operating on `FixedQ32_32` or `FixedVec3` must use strict byte-level assertions (`assert_eq!`) rather than delta-based floating-point comparisons (`assert!((a - b).abs() < epsilon)`).
* **Float Isolation**: Floating-point operations are tested strictly at the boundary conversions. Core logic is checked within the fixed-point bounds.

---

## 2. Cross-Architecture Verification

To prove platform independence, tests are executed in CI across:
* **Ubuntu Linux (x86_64)**
* **macOS (ARM64 Apple Silicon)**
* **Windows (x86_64)**

If the compiler or optimizer alters the arithmetic execution on any runner, the test suite will fail.

---

## 3. WebAssembly Target Verification

To ensure portability to browser runtimes and sandboxed edge workers, the core crates (`bunny-num`, `bunny-linalg`, `bunny-geom`, `bunny-contract`) are checked in CI for target `wasm32-unknown-unknown`:
```bash
cargo check -p bunny-num -p bunny-linalg -p bunny-geom -p bunny-contract --target wasm32-unknown-unknown
```

---

## 4. Integration-First Test Structure

To comply with the strict 300-line file size limit enforced in `CODE_STANDARDS.md`, unit tests are organized as **integration tests** placed outside of source files:
* Core logic is placed in `crates/<crate-name>/src/`.
* Test code is placed in `crates/<crate-name>/tests/` (e.g., `crates/bunny-num/tests/fixed_q32_32_tests.rs`).
* This layout ensures that test imports use only the public API boundaries.

---

## 5. Cross-Language Parity Witnesses

To ensure that Rust DTOs and TypeScript DTOs do not drift, the `bunny-wesley` compiler generates an integrity manifest containing:
* The schema's SHA-256 hash.
* Generator metadata and core versions.
* Output file targets.

Any change to the schema must be rebuilt via `cargo run --bin xtask generate` and verified by checking the manifest checksum matches.

