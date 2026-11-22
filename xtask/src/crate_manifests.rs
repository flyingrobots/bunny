//! What the manifests let into a deterministic crate besides its dependencies: build
//! scripts and target files outside the crate directory, and workspace members that no
//! list of the gate names.
//!
//! The source rules scan the files under `crates/<name>/` of each deterministic crate, so
//! code that Cargo compiles into such a crate from anywhere else is unscanned. A build
//! script is banned outright in a deterministic crate (`build-script`): it runs on the
//! host at build time and can set a `cfg` or an environment variable the library reads,
//! and none exists today. A target (`[lib]`, `[[bin]]`, `[[test]]`, `[[bench]]`,
//! `[[example]]`) whose `path` leaves the crate directory is `target-outside-crate`. Every
//! workspace member must be a deterministic, generator or tooling crate
//! (`unclassified-crate`), so a new crate is placed deliberately and the dependency check
//! knows whether its code is scanned. These rules have no waiver. This module only finds;
//! `code_dojo` reports.

use std::path::{Component, Path, PathBuf};

use toml_edit::{ImDocument, Item, TableLike};

/// A manifest finding: the 1-based line (0 for the whole file), the rule and the message.
pub(super) struct Finding {
    pub(super) line: usize,
    pub(super) rule: &'static str,
    pub(super) message: String,
}

const TARGET_ARRAYS: &[&str] = &["bin", "test", "bench", "example"];

/// The findings of a deterministic crate's manifest. `build_rs` says whether the crate
/// directory holds a `build.rs`, which Cargo builds unless `build = false`.
pub(super) fn deterministic_manifest(text: &str, build_rs: bool) -> Result<Vec<Finding>, String> {
    let document = ImDocument::parse(text).map_err(|error| format!("cannot parse: {error}"))?;
    let root = document.as_table() as &dyn TableLike;
    let mut found = Vec::new();
    let build = root
        .get("package")
        .and_then(Item::as_table_like)
        .and_then(|package| package.get_key_value("build"));
    match build {
        Some((_, item)) if item.as_bool() == Some(false) => {}
        Some((key, item)) => found.push(Finding {
            line: line_at(text, key.span().or_else(|| item.span())),
            rule: "build-script",
            message: format!(
                "a deterministic crate may not have a build script (`build = {}`): it runs on \
                 the host and can change what the library compiles",
                item.as_str().map_or_else(
                    || item.as_bool().map_or_else(String::new, |flag| flag.to_string()),
                    |text| format!("{text:?}"),
                )
            ),
        }),
        None if build_rs => found.push(Finding {
            line: 0,
            rule: "build-script",
            message: "a deterministic crate may not have a build script (`build.rs`): it runs on \
                      the host and can change what the library compiles"
                .to_owned(),
        }),
        None => {}
    }
    for (path, offset) in target_paths(root) {
        if !inside_crate(&path) {
            found.push(Finding {
                line: line_at(text, offset),
                rule: "target-outside-crate",
                message: format!(
                    "target path `{path}` leaves the crate directory, so the gate does not scan it"
                ),
            });
        }
    }
    Ok(found)
}

/// The `path` of every target with the span of its value.
fn target_paths(root: &dyn TableLike) -> Vec<(String, Option<std::ops::Range<usize>>)> {
    let mut paths = Vec::new();
    let mut push = |target: &dyn TableLike| {
        if let Some(item) = target.get("path") {
            if let Some(text) = item.as_str() {
                paths.push((text.to_owned(), item.span()));
            }
        }
    };
    if let Some(lib) = root.get("lib").and_then(Item::as_table_like) {
        push(lib);
    }
    for kind in TARGET_ARRAYS {
        match root.get(kind) {
            Some(Item::ArrayOfTables(tables)) => tables.iter().for_each(|table| push(table)),
            Some(Item::Value(value)) => value
                .as_array()
                .into_iter()
                .flat_map(|array| array.iter())
                .filter_map(|value| value.as_inline_table())
                .for_each(|table| push(table)),
            _ => {}
        }
    }
    paths
}

/// Whether a path written in a crate's manifest stays inside the crate directory.
fn inside_crate(written: &str) -> bool {
    if written.contains('\\') {
        return false;
    }
    let mut depth = 0_usize;
    for component in Path::new(written).components() {
        match component {
            Component::Normal(_) => depth += 1,
            Component::CurDir => {}
            Component::ParentDir => match depth.checked_sub(1) {
                Some(up) => depth = up,
                None => return false,
            },
            Component::RootDir | Component::Prefix(_) => return false,
        }
    }
    true
}

/// The workspace members of a root manifest that no list names: each member's package name
/// paired with its directory, read with `read_manifest` (relative to the root; a `dir/*`
/// member stands for every directory `list_dirs` returns below `dir`).
pub(super) fn unclassified_members(
    root_manifest: &str,
    classified: &[&str],
    read_manifest: &dyn Fn(&Path) -> Option<String>,
    list_dirs: &dyn Fn(&Path) -> Vec<PathBuf>,
) -> Result<Vec<Finding>, String> {
    let document =
        ImDocument::parse(root_manifest).map_err(|error| format!("cannot parse: {error}"))?;
    let root = document.as_table() as &dyn TableLike;
    let members = root
        .get("workspace")
        .and_then(Item::as_table_like)
        .and_then(|workspace| workspace.get("members"))
        .and_then(Item::as_array);
    let mut found = Vec::new();
    for member in members.into_iter().flat_map(|array| array.iter()) {
        let Some(pattern) = member.as_str() else { continue };
        let line = line_at(root_manifest, member.span());
        let dirs = match pattern.strip_suffix("/*") {
            Some(parent) => list_dirs(Path::new(parent)),
            None => vec![PathBuf::from(pattern)],
        };
        for dir in dirs {
            let name = read_manifest(&dir.join("Cargo.toml")).and_then(|text| package_name(&text));
            let name = name.unwrap_or_else(|| dir.to_string_lossy().into_owned());
            if !classified.contains(&name.as_str()) {
                found.push(Finding {
                    line,
                    rule: "unclassified-crate",
                    message: format!(
                        "workspace member `{name}` is in no list of the gate; add it to the \
                         deterministic, generator or tooling crates in xtask/src/code_dojo.rs"
                    ),
                });
            }
        }
    }
    Ok(found)
}

fn package_name(manifest: &str) -> Option<String> {
    let document = ImDocument::parse(manifest).ok()?;
    let root = document.as_table() as &dyn TableLike;
    root.get("package")?.as_table_like()?.get("name")?.as_str().map(str::to_owned)
}

fn line_at(text: &str, span: Option<std::ops::Range<usize>>) -> usize {
    let offset = span.map_or(0, |span| span.start);
    text.get(..offset).map_or(1, |before| before.matches('\n').count() + 1)
}

#[cfg(test)]
mod tests;
