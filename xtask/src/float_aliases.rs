//! Float names for `float-boundary`: the float types under every spelling Rust gives them,
//! and the names the core crates bind to a float with `type` or a renamed `use`.
//!
//! Rule 2 of `CODE_STANDARDS.md` bans the float types in core, not their spellings. The
//! names are `f16`, `f32`, `f64`, `f128`, the C aliases `c_float` and `c_double` (from
//! `core::ffi`, `std::ffi`, `std::os::raw` or any crate), and any name ending in `_f32` or
//! `_f64`, such as `Duration::mul_f64`.
//!
//! Aliases are resolved to a fixed point over every file of the core crates together:
//! `pub type Scalar = f32;` in `bunny-num` makes `Scalar` a float name in every core file,
//! and so do `type B = Scalar;` and `use core::ffi::c_double as D;` anywhere in them. The
//! declaration is reported where it is written, and a waiver there covers only that item,
//! so every use of the alias is reported too. Names are matched, not resolved: a file that
//! defines its own non-float item of the same name (`struct Real(i64);`, `type Real =
//! Fixed;`) does not take the alias, and in any other core file a path segment of that name
//! counts as the float. A float alias declared outside the core crates is not seen; no
//! core crate depends on a crate outside them.

use std::collections::BTreeSet;

use syn::visit::{self, Visit};
use syn::{File, ItemEnum, ItemStruct, ItemTrait, ItemType, ItemUnion, TypePath, UseTree};

/// The float aliases bound anywhere in `files`, the parsed core sources.
pub(super) fn workspace_aliases<'a>(files: impl IntoIterator<Item = &'a File>) -> BTreeSet<String> {
    let mut collector = Collector::default();
    for file in files {
        collector.visit_file(file);
    }
    fixed_point(&collector.bindings)
}

/// The float names in force in `file`: its own aliases and the workspace's, less the names
/// it defines as something other than a float.
pub(super) fn file_aliases(file: &File, workspace: &BTreeSet<String>) -> BTreeSet<String> {
    let mut collector = Collector::default();
    collector.visit_file(file);
    let mut known = collector.bindings.clone();
    known.extend(workspace.iter().map(|name| (name.clone(), BTreeSet::from(["f64".to_string()]))));
    let floats = fixed_point(&known);
    let own_floats = fixed_point(&collector.bindings);
    floats
        .into_iter()
        .filter(|name| own_floats.contains(name) || !collector.defined.contains(name))
        .collect()
}

fn fixed_point(bindings: &[(String, BTreeSet<String>)]) -> BTreeSet<String> {
    let mut aliases = BTreeSet::new();
    loop {
        let before = aliases.len();
        for (name, mentioned) in bindings {
            if mentioned.iter().any(|name| is_float_name(name) || aliases.contains(name)) {
                aliases.insert(name.clone());
            }
        }
        if aliases.len() == before {
            return aliases;
        }
    }
}

/// Whether `name` is a float type or a float-suffixed name.
pub(super) fn is_float_name(name: &str) -> bool {
    matches!(name, "f16" | "f32" | "f64" | "f128" | "c_float" | "c_double")
        || name.ends_with("_f32")
        || name.ends_with("_f64")
}

/// Each binding with the names its definition mentions, and every name a file defines.
#[derive(Default)]
struct Collector {
    bindings: Vec<(String, BTreeSet<String>)>,
    defined: BTreeSet<String>,
}

fn name(ident: &proc_macro2::Ident) -> String {
    crate::nondeterminism::ident_name(ident)
}

impl<'ast> Visit<'ast> for Collector {
    fn visit_item_type(&mut self, node: &'ast ItemType) {
        let mut names = Names::default();
        names.visit_type(&node.ty);
        self.bindings.push((name(&node.ident), names.0));
        self.defined.insert(name(&node.ident));
        visit::visit_item_type(self, node);
    }

    fn visit_item_struct(&mut self, node: &'ast ItemStruct) {
        self.defined.insert(name(&node.ident));
        visit::visit_item_struct(self, node);
    }

    fn visit_item_enum(&mut self, node: &'ast ItemEnum) {
        self.defined.insert(name(&node.ident));
        visit::visit_item_enum(self, node);
    }

    fn visit_item_union(&mut self, node: &'ast ItemUnion) {
        self.defined.insert(name(&node.ident));
        visit::visit_item_union(self, node);
    }

    fn visit_item_trait(&mut self, node: &'ast ItemTrait) {
        self.defined.insert(name(&node.ident));
        visit::visit_item_trait(self, node);
    }

    fn visit_use_tree(&mut self, node: &'ast UseTree) {
        if let UseTree::Rename(rename) = node {
            let mentioned = BTreeSet::from([name(&rename.ident)]);
            self.bindings.push((name(&rename.rename), mentioned));
        }
        visit::visit_use_tree(self, node);
    }
}

/// Every path segment named in a type.
#[derive(Default)]
struct Names(BTreeSet<String>);

impl<'ast> Visit<'ast> for Names {
    fn visit_type_path(&mut self, node: &'ast TypePath) {
        self.0.extend(node.path.segments.iter().map(|segment| name(&segment.ident)));
        visit::visit_type_path(self, node);
    }
}
