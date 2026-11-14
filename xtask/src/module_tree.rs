//! Which Rust files of the workspace's crates are test code, from the module tree.
//!
//! A file is test code when every `mod` declaration that reaches it is under `#[cfg(test)]`
//! (on the declaration, on an enclosing inline module, through a test parent, or as the
//! file's own `#![cfg(test)]`), or when nothing reaches it and it lies under its crate's
//! top-level `tests/` directory. The roots are each crate's `src/lib.rs`, `src/main.rs`,
//! `src/bin/*.rs`, `build.rs`, `benches/*.rs` and `examples/*.rs` (library code) and
//! `tests/*.rs` (test code). A file reached both ways is library code. A file's name or a
//! directory called `tests` decides nothing: `pub mod tests;` without `cfg(test)` is
//! compiled into the library and gets the library's rules. Module files are found as
//! `rustc` finds them: `name.rs` or `name/mod.rs` beside a `mod.rs`-like file (a crate
//! root, a `mod.rs`, or a file loaded by `#[path]`) and below `stem/` beside any other;
//! `#[path]` is relative to the declaring file's directory, and an enclosing inline
//! module adds its name (or its own `#[path]`) to the directory.
//!
//! A `cfg` counts as test-only when it is `test` or an `all(..)` that requires `test`;
//! `any(test, ..)` and `not(test)` are compiled outside tests. A `#[path]` under
//! `cfg_attr` is not followed (the nondeterminism rules report it as `include-source`),
//! and a file the walk cannot read or parse declares nothing.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use syn::{Attribute, Expr, ExprLit, Item, Lit, Meta};

/// The test files of a set of crate sources.
pub(super) struct ModuleTree {
    tests: BTreeSet<PathBuf>,
}

/// Library code outranks test code: a file reached as library code is library code.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Role {
    Test,
    Library,
}

impl ModuleTree {
    /// Walks the module tree of every crate under `crates/` among `files` (repository
    /// relative), reading each reached file with `read`.
    pub(super) fn new(files: &[PathBuf], read: &dyn Fn(&Path) -> Option<String>) -> Self {
        let mut walk = Walk { read, roles: BTreeMap::new() };
        for path in files {
            if let Some(role) = root_role(path) {
                walk.visit(path, role, true);
            }
        }
        let tests = files
            .iter()
            .filter(|path| match walk.roles.get(*path) {
                Some(role) => *role == Role::Test,
                None => in_crate_tests_dir(path),
            })
            .cloned()
            .collect();
        Self { tests }
    }

    /// Whether `path` is test code.
    pub(super) fn is_test(&self, path: &Path) -> bool {
        self.tests.contains(path)
    }
}

/// The parts of `crates/<name>/<rest>`: the crate directory and the rest.
fn crate_parts(path: &Path) -> Option<(PathBuf, Vec<String>)> {
    let parts = path
        .components()
        .map(|part| part.as_os_str().to_str().map(str::to_owned))
        .collect::<Option<Vec<_>>>()?;
    match parts.as_slice() {
        [first, name, rest @ ..] if first == "crates" && !rest.is_empty() => {
            Some((Path::new("crates").join(name), rest.to_vec()))
        }
        _ => None,
    }
}

/// The role of a crate root, or `None` for any other file.
fn root_role(path: &Path) -> Option<Role> {
    let (_, parts) = crate_parts(path)?;
    let parts = parts.iter().map(String::as_str).collect::<Vec<_>>();
    let is_rust = |name: &str| Path::new(name).extension().is_some_and(|ext| ext == "rs");
    match parts.as_slice() {
        ["src", "lib.rs" | "main.rs"]
        | ["build.rs"]
        | ["src", "bin", _, "main.rs"]
        | ["benches" | "examples", _, "main.rs"] => Some(Role::Library),
        ["src", "bin", file] | ["benches" | "examples", file] if is_rust(file) => {
            Some(Role::Library)
        }
        ["tests", file] if is_rust(file) => Some(Role::Test),
        ["tests", _, "main.rs"] => Some(Role::Test),
        _ => None,
    }
}

fn in_crate_tests_dir(path: &Path) -> bool {
    crate_parts(path).is_some_and(|(_, rest)| rest.first().is_some_and(|dir| dir == "tests"))
}

struct Walk<'a> {
    read: &'a dyn Fn(&Path) -> Option<String>,
    roles: BTreeMap<PathBuf, Role>,
}

impl Walk<'_> {
    /// Records `path` in `role` and follows its declarations, unless it already has a role
    /// at least as strong. `mod_rs` says whether its modules live beside it.
    fn visit(&mut self, path: &Path, role: Role, mod_rs: bool) {
        if self.roles.get(path).is_some_and(|known| *known >= role) {
            return;
        }
        let Some(file) = (self.read)(path).and_then(|text| syn::parse_file(&text).ok()) else {
            self.roles.insert(path.to_path_buf(), role);
            return;
        };
        let role = if is_test_cfg(&file.attrs) { Role::Test } else { role };
        self.roles.insert(path.to_path_buf(), role);
        let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
        let child_dir =
            if mod_rs { dir.clone() } else { dir.join(path.file_stem().unwrap_or_default()) };
        let scope = Scope { file_dir: dir, module_dir: child_dir, inline: false, role };
        self.items(&file.items, &scope);
    }

    fn items(&mut self, items: &[Item], scope: &Scope) {
        for item in items {
            let Item::Mod(module) = item else { continue };
            let role = if is_test_cfg(&module.attrs) { Role::Test } else { scope.role };
            let name = module.ident.to_string();
            let path_attr = path_value(&module.attrs);
            if let Some((_, inner)) = &module.content {
                let module_dir = match &path_attr {
                    Some(value) => normalize(&scope.module_dir.join(value)),
                    None => scope.module_dir.join(&name),
                };
                let inner_scope =
                    Scope { file_dir: scope.file_dir.clone(), module_dir, inline: true, role };
                self.items(inner, &inner_scope);
            } else if let Some(value) = path_attr {
                let base = if scope.inline { &scope.module_dir } else { &scope.file_dir };
                self.visit(&normalize(&base.join(value)), role, true);
            } else {
                let flat = scope.module_dir.join(format!("{name}.rs"));
                if (self.read)(&flat).is_some() {
                    self.visit(&flat, role, false);
                } else {
                    self.visit(&scope.module_dir.join(&name).join("mod.rs"), role, true);
                }
            }
        }
    }
}

/// Where a list of items sits: the directory of the file that holds it, the directory its
/// `mod name;` declarations resolve in, whether it is inside an inline module (where a
/// `#[path]` resolves in the module directory too), and the role it is compiled in.
struct Scope {
    file_dir: PathBuf,
    module_dir: PathBuf,
    inline: bool,
    role: Role,
}

/// Removes `.` and `..` components lexically.
fn normalize(path: &Path) -> PathBuf {
    let mut parts: Vec<Component<'_>> = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.iter().collect()
}

/// The literal value of a `#[path = ".."]` attribute.
fn path_value(attrs: &[Attribute]) -> Option<String> {
    attrs.iter().find_map(|attr| match &attr.meta {
        Meta::NameValue(pair) if pair.path.is_ident("path") => match &pair.value {
            Expr::Lit(ExprLit { lit: Lit::Str(text), .. }) => Some(text.value()),
            _ => None,
        },
        _ => None,
    })
}

/// Whether one of `attrs` is a `cfg` that is true only in test builds.
fn is_test_cfg(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| match &attr.meta {
        Meta::List(list) if list.path.is_ident("cfg") => {
            list.parse_args::<Meta>().is_ok_and(|predicate| requires_test(&predicate))
        }
        _ => false,
    })
}

/// `test`, or `all(..)` with a predicate that requires `test`.
fn requires_test(predicate: &Meta) -> bool {
    match predicate {
        Meta::Path(path) => path.is_ident("test"),
        Meta::List(list) if list.path.is_ident("all") => list
            .parse_args_with(syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated)
            .is_ok_and(|items| items.iter().any(requires_test)),
        Meta::List(_) | Meta::NameValue(_) => false,
    }
}

#[cfg(test)]
mod tests;
