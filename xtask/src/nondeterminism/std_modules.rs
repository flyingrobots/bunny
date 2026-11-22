//! The classification of every module and macro at the root of `std`, `core` and `alloc`.
//!
//! Each documented name is permitted, banned whole, or banned item by item by the rules in
//! `nondeterminism.rs`; a path through a name this table does not list is an
//! `unclassified-std` finding, so a module that a new toolchain adds cannot be used until
//! someone decides what it reaches. `std_modules_tests.rs` compares the table with the
//! pinned toolchain's documentation (`std_api_<version>.txt`) and with what the gate
//! reports for each name.

/// How the rules treat a root module or macro.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Handling {
    /// Nothing below it reads host state, the clock or files.
    Permitted,
    /// Banned whole under the listed roots, under the given rule, and permitted under the
    /// others (`std::net` is the network; `core::net` holds only address values).
    Banned(&'static str, &'static [&'static str]),
    /// Banned items below it are named one by one in the rules; the rest is permitted.
    Items,
}

use Handling::{Banned, Items, Permitted};

const STD: &[&str] = &["std"];
const STD_CORE: &[&str] = &["std", "core"];

/// Every module at the root of `std`, `core` or `alloc` in the pinned documentation.
pub(super) const MODULES: &[(&str, Handling)] = &[
    // Host state: the environment and paths of the build, files, processes, the network,
    // the platform's own APIs, the CPU, and ambient randomness.
    ("backtrace", Banned("std-env", STD)),
    ("env", Banned("std-env", STD)),
    ("fs", Banned("std-fs", STD)),
    ("net", Banned("network", STD)),
    ("process", Banned("std-process", STD)),
    ("os", Banned("host-platform", STD)),
    ("arch", Banned("host-cpu", STD_CORE)),
    ("random", Banned("random", STD_CORE)),
    // Mixed modules whose host items are named in the rules: `HashMap` and `HashSet`;
    // `RandomState`; `stdin`, `IsTerminal` and `pipe`; `Location` and the backtrace
    // style; `absolute` and the `Path` methods that read the filesystem; sleeping and
    // `available_parallelism`; the clocks.
    ("collections", Items),
    ("hash", Items),
    ("io", Items),
    ("panic", Items),
    ("path", Items),
    ("thread", Items),
    ("time", Items),
    // Values, traits and algorithms. The float modules are `float-boundary` in core.
    ("alloc", Permitted),
    ("any", Permitted),
    ("array", Permitted),
    ("ascii", Permitted),
    ("async_iter", Permitted),
    ("autodiff", Permitted),
    ("borrow", Permitted),
    ("boxed", Permitted),
    ("bstr", Permitted),
    ("cell", Permitted),
    ("char", Permitted),
    ("clone", Permitted),
    ("cmp", Permitted),
    ("contracts", Permitted),
    ("convert", Permitted),
    ("default", Permitted),
    ("error", Permitted),
    ("f16", Permitted),
    ("f32", Permitted),
    ("f64", Permitted),
    ("f128", Permitted),
    ("ffi", Permitted),
    ("field", Permitted),
    ("fmt", Permitted),
    ("from", Permitted),
    ("future", Permitted),
    ("hint", Permitted),
    ("i8", Permitted),
    ("i16", Permitted),
    ("i32", Permitted),
    ("i64", Permitted),
    ("i128", Permitted),
    ("index", Permitted),
    ("intrinsics", Permitted),
    ("isize", Permitted),
    ("iter", Permitted),
    ("marker", Permitted),
    ("mem", Permitted),
    ("num", Permitted),
    ("ops", Permitted),
    ("option", Permitted),
    ("panicking", Permitted),
    ("pat", Permitted),
    ("pin", Permitted),
    ("prelude", Permitted),
    ("primitive", Permitted),
    ("profiling", Permitted),
    ("ptr", Permitted),
    ("range", Permitted),
    ("rc", Permitted),
    ("result", Permitted),
    ("simd", Permitted),
    ("slice", Permitted),
    ("str", Permitted),
    ("string", Permitted),
    ("sync", Permitted),
    ("task", Permitted),
    ("u8", Permitted),
    ("u16", Permitted),
    ("u32", Permitted),
    ("u64", Permitted),
    ("u128", Permitted),
    ("ub_checks", Permitted),
    ("unsafe_binder", Permitted),
    ("usize", Permitted),
    ("vec", Permitted),
];

/// Every macro at the root of `std`, `core` or `alloc` in the pinned documentation.
pub(super) const MACROS: &[(&str, Handling)] = &[
    // Files and the build environment: `include!` brings in unscanned code, `env!`,
    // `option_env!` and `file!` the build's environment and paths.
    ("include", Banned("include-source", STD_CORE)),
    ("env", Banned("std-env", STD_CORE)),
    ("option_env", Banned("std-env", STD_CORE)),
    ("file", Banned("std-env", STD_CORE)),
    ("is_x86_feature_detected", Banned("host-cpu", STD)),
    ("hash_map", Banned("unordered-container", STD)),
    // Included data must name a literal file inside the crate (`include-data`).
    ("include_bytes", Items),
    ("include_str", Items),
    // Panics are `panic-path` in library code, outside this family.
    ("assert", Permitted),
    ("assert_eq", Permitted),
    ("assert_matches", Permitted),
    ("assert_ne", Permitted),
    ("assert_unsafe_precondition", Permitted),
    ("debug_assert", Permitted),
    ("debug_assert_eq", Permitted),
    ("debug_assert_matches", Permitted),
    ("debug_assert_ne", Permitted),
    ("panic", Permitted),
    ("todo", Permitted),
    ("unimplemented", Permitted),
    ("unreachable", Permitted),
    // Output, formatting and source text: writing to standard output or error is
    // permitted; `line!`, `column!` and `module_path!` depend on the source alone.
    ("cfg", Permitted),
    ("cfg_select", Permitted),
    ("column", Permitted),
    ("compile_error", Permitted),
    ("concat", Permitted),
    ("concat_bytes", Permitted),
    ("const_format_args", Permitted),
    ("dbg", Permitted),
    ("eprint", Permitted),
    ("eprintln", Permitted),
    ("format", Permitted),
    ("format_args", Permitted),
    ("line", Permitted),
    ("log_syntax", Permitted),
    ("matches", Permitted),
    ("module_path", Permitted),
    ("pattern_type", Permitted),
    ("print", Permitted),
    ("println", Permitted),
    ("stringify", Permitted),
    ("thread_local", Permitted),
    ("trace_macros", Permitted),
    ("try", Permitted),
    ("vec", Permitted),
    ("write", Permitted),
    ("writeln", Permitted),
];

/// The roots this table classifies.
pub(super) const ROOTS: &[&str] = &["std", "core", "alloc"];

/// Whether `name`, directly below `std`, `core` or `alloc`, is a classified module or macro.
pub(super) fn is_classified(name: &str) -> bool {
    MODULES.iter().chain(MACROS).any(|(known, _)| *known == name)
}
