//! Rule 10 of `CODE_STANDARDS.md` for the core crates: `clippy::nursery` is cherry-picked,
//! never set to `deny`, `warn` or `forbid` as a group, at any scope.
//!
//! A group level can reach a core crate from four places: a lint attribute in any Rust
//! file of the crate (inner or outer, on the crate, a module, an item or a statement, also
//! inside `cfg_attr`), its manifest's `lints.clippy` table, the workspace's
//! `workspace.lints.clippy` table, which every core crate inherits, and the `rustflags` of
//! a Cargo configuration (`.cargo/config.toml` or `.cargo/config` at the repository root or
//! in the crate's directory). Manifests and configurations are parsed as TOML with
//! `toml_edit`, so every spelling of a table (dotted keys, whitespace around the dots,
//! inline tables, quoted keys) is the same table. This module only finds; `code_dojo`
//! reports. See `docs/topics/lint-policy/README.md`.

use std::path::{Path, PathBuf};

use proc_macro2::TokenStream;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Attribute, Meta, Token};
use toml_edit::{ImDocument, Item, TableLike};

/// The rule name the gate reports.
pub(super) const RULE: &str = "blanket-nursery";

/// The levels that turn the group on; `allow` and `expect` do not.
const LEVELS: &[&str] = &["deny", "warn", "forbid"];

const CONFIG_FILES: &[&str] = &[".cargo/config.toml", ".cargo/config"];

/// A place that sets the group: the file relative to the repository root, its 1-based line
/// (0 for a file that cannot be read or parsed) and what is wrong.
pub(super) struct Finding {
    pub(super) path: PathBuf,
    pub(super) line: usize,
    pub(super) message: String,
}

/// Every place under `root` that sets `clippy::nursery` as a group for one of
/// `core_crates`: the workspace manifest, the root Cargo configuration, and each crate's
/// manifest, Cargo configuration and Rust files (outside `target`).
pub(super) fn findings(root: &Path, core_crates: &[&str]) -> Vec<Finding> {
    let mut found = toml_findings(root, PathBuf::from("Cargo.toml"), Kind::Manifest);
    found.extend(config_findings(root, Path::new("")));
    for name in core_crates {
        let crate_dir = PathBuf::from("crates").join(name);
        found.extend(toml_findings(root, crate_dir.join("Cargo.toml"), Kind::Manifest));
        found.extend(config_findings(root, &crate_dir));
        let mut sources = Vec::new();
        rust_files(root, &crate_dir, &mut sources);
        sources.sort();
        for path in sources {
            found.extend(attribute_findings(root, path));
        }
    }
    found
}

#[derive(Clone, Copy)]
enum Kind {
    Manifest,
    Config,
}

fn config_findings(root: &Path, dir: &Path) -> Vec<Finding> {
    CONFIG_FILES
        .iter()
        .map(|name| dir.join(name))
        .filter(|path| root.join(path).is_file())
        .flat_map(|path| toml_findings(root, path, Kind::Config))
        .collect()
}

/// The Rust files below `dir` (repository-relative), outside `target` directories.
fn rust_files(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root.join(dir)) else { return };
    for entry in entries.flatten() {
        let relative = dir.join(entry.file_name());
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            if entry.file_name() != "target" {
                rust_files(root, &relative, files);
            }
        } else if relative.extension().is_some_and(|ext| ext == "rs") {
            files.push(relative);
        }
    }
}

fn toml_findings(root: &Path, path: PathBuf, kind: Kind) -> Vec<Finding> {
    let lines = std::fs::read_to_string(root.join(&path))
        .map_err(|error| format!("cannot read: {error}"))
        .and_then(|text| match kind {
            Kind::Manifest => manifest_lines(&text),
            Kind::Config => config_lines(&text),
        });
    let message = match kind {
        Kind::Manifest => {
            "sets `clippy::nursery` as a group; name the nursery lints instead (Rule 10)"
        }
        Kind::Config => {
            "passes `clippy::nursery` as a group in rustflags; name the nursery lints instead \
             (Rule 10)"
        }
    };
    match lines {
        Ok(lines) => lines
            .into_iter()
            .map(|line| Finding { path: path.clone(), line, message: message.to_owned() })
            .collect(),
        Err(message) => vec![Finding { path, line: 0, message }],
    }
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

/// The 1-based line of each attribute anywhere in `source` (inner or outer, on the file, a
/// module, an item, a statement or an expression) that sets `clippy::nursery` to a level in
/// [`LEVELS`], or `Err` with the parse error.
pub(super) fn attribute_lines(source: &str) -> Result<Vec<usize>, syn::Error> {
    let file = syn::parse_file(source)?;
    let mut collector = Attributes::default();
    collector.visit_file(&file);
    collector.lines.sort_unstable();
    collector.lines.dedup();
    Ok(collector.lines)
}

#[derive(Default)]
struct Attributes {
    lines: Vec<usize>,
}

impl<'ast> Visit<'ast> for Attributes {
    fn visit_attribute(&mut self, node: &'ast Attribute) {
        if meta_sets_group(&node.meta) {
            self.lines.push(node.span().start().line);
        }
        visit::visit_attribute(self, node);
    }
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
/// Cargo manifest, in its `lints.clippy` or `workspace.lints.clippy` table however the
/// table is spelled, or `Err` when the manifest is not TOML.
pub(super) fn manifest_lines(manifest: &str) -> Result<Vec<usize>, String> {
    let document = parse(manifest)?;
    let root = document.as_table() as &dyn TableLike;
    let mut found = Vec::new();
    for lints in
        [lookup(root, &["lints"]), lookup(root, &["workspace", "lints"])].into_iter().flatten()
    {
        if let Some((key, item)) =
            lookup(lints, &["clippy"]).and_then(|clippy| clippy.get_key_value("nursery"))
        {
            if level_of(item).is_some_and(|level| LEVELS.contains(&level)) {
                let offset = key.span().or_else(|| item.span()).map_or(0, |span| span.start);
                found.push(line_at(manifest, offset));
            }
        }
    }
    found.sort_unstable();
    Ok(found)
}

/// The 1-based line of each `rustflags` value of a Cargo configuration (`build.rustflags`
/// or any `target.<triple or cfg>.rustflags`, a string or an array) that passes
/// `clippy::nursery` with `-D`, `-W`, `-F`, `--deny`, `--warn` or `--forbid`, joined or
/// separate, or `Err` when the configuration is not TOML.
pub(super) fn config_lines(config: &str) -> Result<Vec<usize>, String> {
    let document = parse(config)?;
    let root = document.as_table() as &dyn TableLike;
    let mut flags = Vec::new();
    flags.extend(lookup(root, &["build"]).and_then(|build| build.get("rustflags")));
    if let Some(targets) = lookup(root, &["target"]) {
        flags.extend(
            targets.iter().filter_map(|(_, target)| target.as_table_like()?.get("rustflags")),
        );
    }
    let mut found = flags
        .into_iter()
        .filter(|item| sets_group_in_flags(&flag_words(item)))
        .map(|item| line_at(config, item.span().map_or(0, |span| span.start)))
        .collect::<Vec<_>>();
    found.sort_unstable();
    Ok(found)
}

fn parse(text: &str) -> Result<ImDocument<&str>, String> {
    ImDocument::parse(text).map_err(|error| format!("cannot parse: {error}"))
}

/// The table at `keys` below `table`, whatever form each level takes.
fn lookup<'a>(table: &'a dyn TableLike, keys: &[&str]) -> Option<&'a dyn TableLike> {
    keys.iter().try_fold(table, |table, key| table.get(key)?.as_table_like())
}

/// A lint's level: the string itself, or the `level` of its table.
fn level_of(item: &Item) -> Option<&str> {
    item.as_str().or_else(|| item.as_table_like()?.get("level")?.as_str())
}

/// The words of a `rustflags` value: an array of strings, or one string split on spaces.
fn flag_words(item: &Item) -> Vec<String> {
    match item.as_array() {
        Some(array) => array.iter().filter_map(|value| value.as_str().map(str::to_owned)).collect(),
        None => item
            .as_str()
            .map(|text| text.split_whitespace().map(str::to_owned).collect())
            .unwrap_or_default(),
    }
}

fn sets_group_in_flags(words: &[String]) -> bool {
    let level = |flag: &str| matches!(flag, "-D" | "-W" | "-F" | "--deny" | "--warn" | "--forbid");
    words.iter().enumerate().any(|(index, word)| {
        let joined = ["-D", "-W", "-F", "--deny=", "--warn=", "--forbid="]
            .iter()
            .any(|prefix| word.strip_prefix(prefix) == Some("clippy::nursery"));
        let separate =
            level(word) && words.get(index + 1).is_some_and(|next| next == "clippy::nursery");
        joined || separate
    })
}

fn line_at(text: &str, offset: usize) -> usize {
    text.get(..offset).map_or(1, |before| before.matches('\n').count() + 1)
}

#[cfg(test)]
mod tests;
