//! Rule 10 of `CODE_STANDARDS.md` for the core crates: `clippy::nursery` is cherry-picked,
//! never set to `deny`, `warn` or `forbid` as a group.
//!
//! A group level can reach a core crate from three places: an inner attribute of its
//! `src/lib.rs` (also inside `cfg_attr`), its manifest's `[lints.clippy]` table, and the
//! workspace's `[workspace.lints.clippy]` table, which every core crate inherits. This module
//! only finds; `code_dojo` reports. See `docs/topics/lint-policy/README.md`.

use std::path::{Path, PathBuf};

use proc_macro2::TokenStream;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Attribute, Meta, Token};

/// The rule name the gate reports.
pub(super) const RULE: &str = "blanket-nursery";

/// The levels that turn the group on; `allow` and `expect` do not.
const LEVELS: &[&str] = &["deny", "warn", "forbid"];

/// A place that sets the group: the file relative to the repository root, its 1-based line
/// (0 for a file that cannot be read or parsed) and what is wrong.
pub(super) struct Finding {
    pub(super) path: PathBuf,
    pub(super) line: usize,
    pub(super) message: String,
}

/// Every place under `root` that sets `clippy::nursery` as a group for one of `core_crates`:
/// the workspace manifest, and each crate's manifest and `src/lib.rs`.
pub(super) fn findings(root: &Path, core_crates: &[&str]) -> Vec<Finding> {
    let mut found = manifest_findings(root, PathBuf::from("Cargo.toml"));
    for name in core_crates {
        let crate_dir = PathBuf::from("crates").join(name);
        found.extend(manifest_findings(root, crate_dir.join("Cargo.toml")));
        found.extend(attribute_findings(root, crate_dir.join("src").join("lib.rs")));
    }
    found
}

fn manifest_findings(root: &Path, path: PathBuf) -> Vec<Finding> {
    let lines = match std::fs::read_to_string(root.join(&path)) {
        Ok(text) => manifest_lines(&text),
        Err(error) => {
            return vec![Finding { path, line: 0, message: format!("cannot read: {error}") }]
        }
    };
    let message = "sets `clippy::nursery` as a group; name the nursery lints instead (Rule 10)";
    lines
        .into_iter()
        .map(|line| Finding { path: path.clone(), line, message: message.to_owned() })
        .collect()
}

fn attribute_findings(root: &Path, path: PathBuf) -> Vec<Finding> {
    let lines = std::fs::read_to_string(root.join(&path))
        .map_err(|error| format!("cannot read: {error}"))
        .and_then(|source| {
            attribute_lines(&source).map_err(|error| format!("cannot parse: {error}"))
        });
    let message = "denies, warns or forbids `clippy::nursery` as a group; name the nursery lints \
                   with a reason instead (Rule 10)";
    match lines {
        Ok(lines) => lines
            .into_iter()
            .map(|line| Finding { path: path.clone(), line, message: message.to_owned() })
            .collect(),
        Err(message) => vec![Finding { path, line: 0, message }],
    }
}

/// The 1-based line of each inner attribute of `source` (a crate root) that sets
/// `clippy::nursery` to a level in [`LEVELS`], or `Err` with the parse error.
pub(super) fn attribute_lines(source: &str) -> Result<Vec<usize>, syn::Error> {
    let file = syn::parse_file(source)?;
    Ok(file
        .attrs
        .iter()
        .filter(|attr| sets_group(attr))
        .map(|attr| attr.span().start().line)
        .collect())
}

fn sets_group(attr: &Attribute) -> bool {
    meta_sets_group(&attr.meta)
}

/// Whether `meta` is `deny(..)`, `warn(..)` or `forbid(..)` naming `clippy::nursery`, or a
/// `cfg_attr(predicate, ..)` whose attributes include one.
fn meta_sets_group(meta: &Meta) -> bool {
    let Meta::List(list) = meta else { return false };
    let Some(name) = list.path.get_ident().map(ToString::to_string) else { return false };
    let Some(items) = nested(&list.tokens) else { return false };
    if name == "cfg_attr" {
        return items.iter().skip(1).any(meta_sets_group);
    }
    LEVELS.contains(&name.as_str()) && items.iter().any(is_nursery_group)
}

fn nested(tokens: &TokenStream) -> Option<Punctuated<Meta, Token![,]>> {
    syn::parse::Parser::parse2(Punctuated::<Meta, Token![,]>::parse_terminated, tokens.clone()).ok()
}

fn is_nursery_group(meta: &Meta) -> bool {
    let Meta::Path(path) = meta else { return false };
    let names: Vec<String> =
        path.segments.iter().map(|segment| segment.ident.to_string()).collect();
    names == ["clippy", "nursery"]
}

/// The 1-based line of each `nursery` key that sets the group to a level in [`LEVELS`] in a
/// Cargo manifest: in a `[lints.clippy]` or `[workspace.lints.clippy]` table, or as the
/// dotted key `clippy.nursery` in a `[lints]` or `[workspace.lints]` table. The scan reads one
/// key per line, as the repository's manifests are written; it does not read inline-table
/// forms such as `lints = { clippy = { nursery = "deny" } }`.
pub(super) fn manifest_lines(manifest: &str) -> Vec<usize> {
    let mut table = String::new();
    let mut found = Vec::new();
    for (index, line) in manifest.lines().enumerate() {
        let code = line.split('#').next().unwrap_or_default().trim();
        if let Some(header) = code.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')) {
            header.trim().clone_into(&mut table);
            continue;
        }
        let Some((key, value)) = code.split_once('=') else { continue };
        let key = key.trim().replace(['"', '\''], "");
        let group_key = match table.as_str() {
            "lints.clippy" | "workspace.lints.clippy" => key == "nursery",
            "lints" | "workspace.lints" => key == "clippy.nursery",
            _ => false,
        };
        if group_key && LEVELS.contains(&level_of(value).as_str()) {
            found.push(index + 1);
        }
    }
    found
}

/// The level of a lint value: `"deny"`, or the `level` of `{ level = "deny", .. }`.
fn level_of(value: &str) -> String {
    let value = value.trim();
    let level = value
        .strip_prefix('{')
        .and_then(|table| table.split(',').find_map(|pair| pair.trim().strip_prefix("level")))
        .and_then(|rest| rest.trim().strip_prefix('='))
        .unwrap_or(value);
    level.trim().trim_end_matches('}').trim().trim_matches(['"', '\'']).to_owned()
}

#[cfg(test)]
mod tests;
