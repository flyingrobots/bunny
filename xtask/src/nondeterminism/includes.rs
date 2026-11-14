//! Compile-time access to files and to the build environment in deterministic crates.
//!
//! The gate scans every tracked `.rs` file of a crate, so code reaches a crate unscanned
//! only through `include!` or a `#[path]` module naming another kind of file or a file
//! outside the crate. `include!` is banned outright: a `.rs` file can be a `mod`, and any
//! other target (a non-Rust file, a `concat!` or `env!` path, a build-script output) is
//! code the gate never reads, however many levels of indirection lie behind it. A
//! `#[path]` module must name a `.rs` file inside the crate directory, which the gate
//! then scans as a file of that crate. `include_str!` and `include_bytes!` must name one
//! string literal inside the crate directory: data in the package, not on the host.
//! `env!` and `option_env!` read the build environment and are `std-env` findings.
//!
//! Paths resolve lexically against the including file's directory, which is where the
//! compiler resolves `include_str!` and a top-level `#[path]`. Inside an inline module the
//! compiler resolves `#[path]` against a deeper directory, so the lexical answer can only
//! report a path as outside when it is inside, never the reverse. Symbolic links are not
//! followed.

use std::path::{Component, Path, PathBuf};

use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{Attribute, Expr, ExprLit, Lit, LitStr, Meta, Token};

use super::{ident_name, line_of};

const INCLUDE_SOURCE: &str =
    "code from a file the gate does not scan is banned; declare a `.rs` file inside the crate as a `mod`";
const INCLUDE_DATA: &str =
    "included data must be one string literal naming a file inside the crate directory";
const ENV_MACRO: &str = "compile-time environment access is banned; pass configuration explicitly";

/// Where a source file sits: the directory its relative paths resolve against, and the
/// directory of its crate (`crates/<name>`).
pub(super) struct Origin {
    dir: PathBuf,
    crate_dir: Option<PathBuf>,
}

impl Origin {
    /// The origin of a repository-relative source path.
    pub(super) fn of(path: &Path) -> Self {
        let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
        let mut parts = path.components();
        let crate_dir = match (parts.next(), parts.next()) {
            (Some(Component::Normal(first)), Some(Component::Normal(name)))
                if first == "crates" =>
            {
                Some(Path::new("crates").join(name))
            }
            _ => None,
        };
        Self { dir, crate_dir }
    }

    /// Whether a path written in this file names a place inside its crate directory.
    fn inside_crate(&self, written: &str) -> bool {
        self.resolve(written).is_some()
    }

    /// Whether a module path names a file the gate scans as part of this crate: a `.rs`
    /// file inside the crate directory and outside the directories the gate does not list
    /// (`target`, `vendor`, `third_party`).
    fn scans(&self, path: &ModulePath) -> bool {
        match path {
            ModulePath::Literal(written) => self
                .resolve(written)
                .is_some_and(|resolved| crate::code_dojo::is_rust_source(&resolved)),
            ModulePath::NotLiteral => false,
        }
    }

    /// The repository-relative path that a path written in this file names, when it lies
    /// inside the crate directory.
    fn resolve(&self, written: &str) -> Option<PathBuf> {
        let crate_dir = self.crate_dir.as_ref()?;
        if written.contains('\\') {
            return None;
        }
        let mut resolved = Vec::new();
        for component in self.dir.components().chain(Path::new(written).components()) {
            match component {
                Component::Normal(part) => resolved.push(part),
                Component::CurDir => {}
                Component::ParentDir => {
                    resolved.pop()?;
                }
                Component::RootDir | Component::Prefix(_) => return None,
            }
        }
        let resolved = resolved.iter().collect::<PathBuf>();
        resolved.starts_with(crate_dir).then_some(resolved)
    }
}

/// The rules a macro invocation breaks, from its import-expanded path and its arguments.
/// Only the standard macros count: a bare name, or one directly under `std` or `core`.
pub(super) fn macro_rules(
    expanded: &[String],
    args: &TokenStream,
    origin: &Origin,
) -> Vec<(&'static str, &'static str)> {
    let Some((name, module)) = expanded.split_last() else {
        return Vec::new();
    };
    let standard = match module {
        [] => true,
        [root] => root == "std" || root == "core",
        _ => false,
    };
    if !standard {
        return Vec::new();
    }
    match name.as_str() {
        "include" => vec![("include-source", INCLUDE_SOURCE)],
        "env" | "option_env" => vec![("std-env", ENV_MACRO)],
        "include_str" | "include_bytes" if !literal_inside(args, origin) => {
            vec![("include-data", INCLUDE_DATA)]
        }
        _ => Vec::new(),
    }
}

fn literal_inside(args: &TokenStream, origin: &Origin) -> bool {
    syn::parse2::<LitStr>(args.clone()).is_ok_and(|literal| origin.inside_crate(&literal.value()))
}

/// The line of a `#[path]` attribute on a module that names anything but a file the gate
/// scans as part of this crate, including one set through `cfg_attr`, nested or not.
pub(super) fn module_path_finding(
    attrs: &[Attribute],
    origin: &Origin,
) -> Option<(usize, (&'static str, &'static str))> {
    attrs.iter().find_map(|attr| {
        let path = module_path(&attr.meta)?;
        (!origin.scans(&path)).then(|| (line_of(attr.pound_token.span), INCLUDE_SOURCE_RULE))
    })
}

/// The `#[path]` attributes inside a macro's tokens, which `syn` does not parse into items:
/// the line of each that names anything but a file the gate scans as part of this crate.
pub(super) fn token_module_path_findings(
    tokens: &TokenStream,
    origin: &Origin,
) -> Vec<(usize, (&'static str, &'static str))> {
    let tokens = tokens.clone().into_iter().collect::<Vec<_>>();
    let mut found = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if let TokenTree::Group(group) = token {
            found.extend(token_module_path_findings(&group.stream(), origin));
        }
        if let (TokenTree::Punct(pound), Some(TokenTree::Group(body))) =
            (token, tokens.get(index + 1))
        {
            let attribute = (pound.as_char() == '#' && body.delimiter() == Delimiter::Bracket)
                .then(|| syn::parse2::<Meta>(body.stream()).ok())
                .flatten();
            if let Some(path) = attribute.as_ref().and_then(module_path) {
                if !origin.scans(&path) {
                    found.push((line_of(pound.span()), INCLUDE_SOURCE_RULE));
                }
            }
        }
    }
    found
}

const INCLUDE_SOURCE_RULE: (&str, &str) = ("include-source", INCLUDE_SOURCE);

/// The value of a module's `path` attribute.
enum ModulePath {
    Literal(String),
    NotLiteral,
}

/// The value of `path = ..`, or of a `path` inside `cfg_attr(..)` at any depth; `None` for
/// any other attribute.
fn module_path(meta: &Meta) -> Option<ModulePath> {
    if meta.path().is_ident("path") {
        return Some(match meta {
            Meta::NameValue(pair) => match &pair.value {
                Expr::Lit(ExprLit { lit: Lit::Str(text), .. }) => ModulePath::Literal(text.value()),
                _ => ModulePath::NotLiteral,
            },
            Meta::Path(_) | Meta::List(_) => ModulePath::NotLiteral,
        });
    }
    let Meta::List(list) = meta else {
        return None;
    };
    if !list.path.is_ident("cfg_attr") {
        return None;
    }
    let Ok(nested) = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(list.tokens.clone())
    else {
        return Some(ModulePath::NotLiteral);
    };
    nested.iter().skip(1).find_map(module_path)
}

/// A macro invocation found inside another macro's tokens.
pub(super) struct MacroCall {
    pub(super) line: usize,
    pub(super) path: Vec<String>,
    pub(super) args: TokenStream,
}

/// Every `path!(..)`, `path![..]` and `path! {..}` in a token stream, descending into
/// groups.
pub(super) fn macro_calls(tokens: &TokenStream) -> Vec<MacroCall> {
    let mut calls = Vec::new();
    collect_calls(tokens, &mut calls);
    calls
}

fn collect_calls(tokens: &TokenStream, calls: &mut Vec<MacroCall>) {
    let tokens = tokens.clone().into_iter().collect::<Vec<_>>();
    for (index, token) in tokens.iter().enumerate() {
        match token {
            TokenTree::Group(group) => collect_calls(&group.stream(), calls),
            TokenTree::Punct(bang) if bang.as_char() == '!' => {
                if let (Some(TokenTree::Group(args)), Some((line, path))) =
                    (tokens.get(index + 1), path_ending_at(&tokens, index))
                {
                    calls.push(MacroCall { line, path, args: args.stream() });
                }
            }
            TokenTree::Punct(_) | TokenTree::Ident(_) | TokenTree::Literal(_) => {}
        }
    }
}

/// The `::`-separated identifier path that ends just before `tokens[end]`, with the line
/// of its first segment.
fn path_ending_at(tokens: &[TokenTree], end: usize) -> Option<(usize, Vec<String>)> {
    let mut at = end.checked_sub(1)?;
    let TokenTree::Ident(last) = tokens.get(at)? else {
        return None;
    };
    let mut path = vec![ident_name(last)];
    let mut line = line_of(last.span());
    while let (
        Some(TokenTree::Punct(first)),
        Some(TokenTree::Punct(second)),
        Some(TokenTree::Ident(prev)),
    ) = (
        at.checked_sub(2).and_then(|index| tokens.get(index)),
        at.checked_sub(1).and_then(|index| tokens.get(index)),
        at.checked_sub(3).and_then(|index| tokens.get(index)),
    ) {
        if first.as_char() != ':' || first.spacing() != Spacing::Joint || second.as_char() != ':' {
            break;
        }
        path.insert(0, ident_name(prev));
        line = line_of(prev.span());
        at -= 3;
    }
    Some((line, path))
}
