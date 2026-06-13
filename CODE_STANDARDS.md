# Bunny Rust Code Standards: Strict Deterministic Math & Geometry

## Rule 0: Bit-Level Determinism (Non-Negotiable)

When the graphics engine runs, only one truth matters:

**Do all target architectures (x86_64, ARM64, WASM) compute the exact same bitwise output for any given mathematical input?**

Any variation in rounding, FMA register optimizations, or compiler floating-point instructions is a system crash. If a function behaves differently on macOS than in a WebAssembly sandbox, it is a violation of Rule 0.

---

## Core Philosophy

* **Determinism is Paramount**: No platform-dependent arithmetic, no FPU registers variation, no FMA instructions.
* **Compile-Time Safety over Runtime Panic**: Zero uses of `unwrap()`, `expect()`, or index-out-of-bounds panics in library code. Use type-safe `Result` and `Option` propagation.
* **No Unsafe Code**: All crates must declare `#![deny(unsafe_code)]`. Memory safety is enforced by the compiler without exceptions.
* **Immutability by Default**: All structures and vectors are parsed, validated, and operated on using pure, side-effect-free, value-object semantics.
* **Explicitness**: No hidden casts, no implicit rounding, and no magic numbers.

---

## Mandatory Code Quality Rules

### 1. Clippy & Compiler Warnings
All crates must compile with the following module-level annotations:
```rust
#![deny(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(missing_docs)]
```
Every compiler warning, linter suggestion, or missing documentation line is a build failure.

### 2. Zero Unsafe Code
The use of `unsafe` blocks, raw pointer manipulations, type transmutations (`std::mem::transmute`), or unsafe union access is completely banned.

### 3. Floating-Point Boundary Control
Native `f32` or `f64` arithmetic operators (`+`, `-`, `*`, `/`) are **banned** in core geometric queries and transformations. You must convert coordinates to `FixedQ32_32` for operations, or explicitly prove that the operation is non-variant.

### 4. Zero Panics in Library Code
* Never use `unwrap()`, `expect()`, or indexing that can panic (`slice[index]`).
* Use `.get()`, `.first()`, `.get_mut()` and handle the option or error.
* Checked math operations must be handled gracefully.

### 5. Side-Effect and Ambient State Denial
Core crates must never access:
* System Time (`std::time::SystemTime`)
* Random Number Generators (`rand::thread_rng()`)
* File System or Network Sockets
* Ambient Environment variables (`std::env`)

If any of these are needed, they must be injected as pure, stateless parameters at the boundary.

---

## Scoped Code Limits (Enforced)

To avoid "strict standard theater," limits are scoped strictly by crate category:

### 1. Core Runtime Crates (`bunny-num`, `bunny-linalg`, `bunny-geom`, `bunny-query`, `bunny-broadphase`, `bunny-mesh`)
* **File size**: ≤ **300 lines** of source code (strict).
* **Source line length**: ≤ **100 characters** (excluding URLs/paths).
* **Function size**: ≤ **25 lines** (excluding comments/whitespace).
* **Statements per function**: ≤ **15**.
* **Nesting depth**: ≤ **3** levels.
* **Parameters**: ≤ **4** parameters per function.
* **Cyclomatic complexity**: ≤ **6**.
* **Panics**: **Zero** runtime panic potential (no `unwrap`, `expect`, or indexing).

### 2. Code-Generator Crates (`bunny-wesley`)
* **File size**: ≤ **500 lines** of source code (relaxed for AST generation code).
* **Function size**: ≤ **50 lines**.
* **Nesting depth**: ≤ **4** levels (due to recursive AST traversals).
* **Panics**: Discouraged, but allowed for unrecoverable schema validation errors during build.

### 3. Build Tooling Crates (`xtask`)
* **File size**: Exempt.
* **Panics**: Allowed (`unwrap()` / `expect()`) for scripting convenience.
* **Lints**: Must compile clean under workspace Clippy rules.

---

## Language & Numeric Policy

### Type-Safe Fixed-Point Math
All fixed-point variables must be wrapped in a strongly typed newtype wrapper rather than using raw integer aliases:
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FixedQ32_32(pub i64);
```
Every arithmetic operation must:
1. Promote operands to `i128` to guarantee no intermediate overflow.
2. Apply unbiased **Banker's Rounding (Ties-to-Even)** at the bit boundary.
3. Apply **Saturating Arithmetic** during downcasting back to `i64`.

## Floating-Point Boundary & DTO Doctrine

* **Convenience views only**: Floating-point representations (like `f32` in generated DTO contracts or `Ray3` / `Aabb3` / `Sphere3` in `bunny-geom`) are boundary convenience formats used solely for ingress/egress interfaces (e.g., reading from standard files or passing to/from non-deterministic platforms).
* **Deterministic canonical reality**: Inside the core runtime, all canonical geometry computations are done exclusively in deterministic Q32.32 fixed-point math. Float convenience formats must never be used as the internal source of truth.

---

## PR Review Checklist

- [ ] Bit-level determinism guaranteed? Tested on multiple architectures?
- [ ] `#![deny(unsafe_code)]` declared and active?
- [ ] Zero compiler warnings and zero Clippy warnings (`cargo clippy`)?
- [ ] No `unwrap()`, `expect()`, or array indexing panics?
- [ ] Functions ≤ 25 lines? File length ≤ 300 lines? Nesting depth ≤ 3?
- [ ] Side effects (time, random, filesystem) injected or absent?
- [ ] Standard formatting verified (`cargo fmt --check`)?
