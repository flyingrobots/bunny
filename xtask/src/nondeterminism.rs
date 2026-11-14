//! Syntax-aware nondeterminism rules for Bunny's deterministic library crates.
//!
//! The rule IDs are the vocabulary of Echo's `ban-nondeterminism` scan, so one name means
//! one ban in both repositories. `docs/CODE_DOJO.md` maps every Echo rule onto this module.
//!
//! The source rules resolve each path through the file's `use` declarations before
//! matching, so a renamed import, a module alias, a `use` group or a glob cannot hide a
//! banned item, and they read the token streams of macro invocations, which `syn` leaves
//! unparsed. This module only finds; `code_dojo` applies waivers and reports.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::{Spacing, Span, TokenStream, TokenTree};
use syn::visit::{self, Visit};
use syn::{File, ItemUse, Lit, Macro, Path as SynPath, UseTree};

/// A source finding: the 1-based line and the rule it breaks.
pub(super) struct Finding {
    pub(super) line: usize,
    pub(super) rule: &'static str,
    pub(super) message: &'static str,
}

/// A banned crate in the normal dependency graph of a deterministic library crate.
pub(super) struct DependencyFinding {
    pub(super) root: String,
    pub(super) dependency: String,
    pub(super) rule: &'static str,
}

/// A path that is banned together with everything below it.
struct PrefixRule {
    prefix: &'static [&'static str],
    rule: &'static str,
    message: &'static str,
}

const CLOCK: &str = "wall-clock time is banned; inject a deterministic clock";
const TICK: &str = "host monotonic clocks are banned; inject a deterministic tick source";
const SLEEP: &str =
    "sleeping on the host scheduler is banned; nothing in a deterministic crate waits";
const PARALLELISM: &str = "host core counts are banned; take the worker count as an explicit input";
const RANDOM: &str = "ambient randomness is banned; inject a seeded generator";
const CALLBACK: &str = "host callbacks are banned; core results must not depend on host code";
const NETWORK: &str = "network access is banned in deterministic crates";
const UNORDERED: &str = "hash containers are banned; use BTreeMap, BTreeSet or a sorted Vec";
const HAND_HASHED: &str =
    "a hand-written hash table needs a waiver showing its order cannot reach output";
const JSON: &str = "JSON serialization is banned in deterministic crates; use the canonical codecs";
const WASM_SERDE: &str = "serde-wasm-bindgen is banned in deterministic crates";
const ENV: &str = "environment access is banned; pass configuration explicitly";
const FS: &str = "filesystem access is banned; take bytes from an adapter at the edge";
const PROCESS: &str = "process access is banned in deterministic crates";

const PREFIX_RULES: &[PrefixRule] = &[
    PrefixRule { prefix: &["std", "time", "SystemTime"], rule: "time-system", message: CLOCK },
    PrefixRule { prefix: &["std", "time", "UNIX_EPOCH"], rule: "time-system", message: CLOCK },
    PrefixRule { prefix: &["std", "time", "Instant"], rule: "time-instant", message: TICK },
    PrefixRule { prefix: &["std", "thread", "sleep"], rule: "thread-sleep", message: SLEEP },
    PrefixRule { prefix: &["tokio", "time", "sleep"], rule: "thread-sleep", message: SLEEP },
    PrefixRule { prefix: &["async_std", "time", "sleep"], rule: "thread-sleep", message: SLEEP },
    PrefixRule { prefix: &["async_std", "task", "sleep"], rule: "thread-sleep", message: SLEEP },
    PrefixRule {
        prefix: &["std", "thread", "available_parallelism"],
        rule: "host-parallelism",
        message: PARALLELISM,
    },
    PrefixRule { prefix: &["rand"], rule: "random", message: RANDOM },
    PrefixRule { prefix: &["getrandom"], rule: "random", message: RANDOM },
    PrefixRule { prefix: &["fastrand"], rule: "random", message: RANDOM },
    PrefixRule { prefix: &["js_sys"], rule: "host-callback", message: CALLBACK },
    PrefixRule { prefix: &["wasm_bindgen"], rule: "host-callback", message: CALLBACK },
    PrefixRule { prefix: &["web_sys"], rule: "host-callback", message: CALLBACK },
    PrefixRule { prefix: &["std", "net"], rule: "network", message: NETWORK },
    PrefixRule { prefix: &["reqwest"], rule: "network", message: NETWORK },
    PrefixRule { prefix: &["ureq"], rule: "network", message: NETWORK },
    PrefixRule {
        prefix: &["std", "collections", "HashMap"],
        rule: "unordered-container",
        message: UNORDERED,
    },
    PrefixRule {
        prefix: &["std", "collections", "HashSet"],
        rule: "unordered-container",
        message: UNORDERED,
    },
    PrefixRule {
        prefix: &["std", "collections", "hash_map"],
        rule: "unordered-container",
        message: UNORDERED,
    },
    PrefixRule {
        prefix: &["std", "collections", "hash_set"],
        rule: "unordered-container",
        message: UNORDERED,
    },
    PrefixRule {
        prefix: &["std", "hash", "RandomState"],
        rule: "unordered-container",
        message: UNORDERED,
    },
    PrefixRule { prefix: &["hashbrown"], rule: "unordered-container", message: UNORDERED },
    PrefixRule { prefix: &["rustc_hash"], rule: "unordered-container", message: UNORDERED },
    PrefixRule { prefix: &["serde_json"], rule: "json-serialization", message: JSON },
    PrefixRule { prefix: &["serde_wasm_bindgen"], rule: "wasm-serde", message: WASM_SERDE },
    PrefixRule { prefix: &["std", "env"], rule: "std-env", message: ENV },
    PrefixRule { prefix: &["std", "fs"], rule: "std-fs", message: FS },
    PrefixRule { prefix: &["std", "process"], rule: "std-process", message: PROCESS },
];

/// Names banned wherever they appear in a path, even unresolved. These keep the
/// coverage of Code Dojo's earlier `ambient-state` and `nondeterministic-map` rules,
/// which matched names without resolving imports.
fn name_rules(segments: &[String]) -> Vec<(&'static str, &'static str)> {
    let mut rules = Vec::new();
    let names = segments.iter().map(String::as_str).collect::<Vec<_>>();
    if names.ends_with(&["SystemTime", "now"]) {
        rules.push(("time-system", CLOCK));
    }
    if names.ends_with(&["Instant", "now"]) {
        rules.push(("time-instant", TICK));
    }
    if names.last() == Some(&"available_parallelism") {
        rules.push(("host-parallelism", PARALLELISM));
    }
    if names.last() == Some(&"thread_rng") {
        rules.push(("random", RANDOM));
    }
    if names.ends_with(&["TcpStream", "connect"]) || names.ends_with(&["UdpSocket", "bind"]) {
        rules.push(("network", NETWORK));
    }
    if names.iter().any(|name| matches!(*name, "HashMap" | "HashSet" | "FxHashMap" | "FxHashSet")) {
        rules.push(("unordered-container", UNORDERED));
    }
    if names.first() == Some(&"fs") || names.ends_with(&["File", "open"]) {
        rules.push(("std-fs", FS));
    }
    rules
}

fn starts_with(segments: &[String], prefix: &[&str]) -> bool {
    segments.len() >= prefix.len() && segments.iter().zip(prefix).all(|(left, right)| left == right)
}

/// Items below a banned prefix that are deterministic. `DefaultHasher::new()` uses fixed
/// `SipHash` keys; only `RandomState` seeds a hasher from the host.
const PERMITTED: &[&[&str]] = &[&["std", "collections", "hash_map", "DefaultHasher"]];

/// Every rule a fully expanded path breaks.
fn path_rules(expanded: &[String], written: &[String]) -> Vec<(&'static str, &'static str)> {
    if PERMITTED.iter().any(|permitted| starts_with(expanded, permitted)) {
        return Vec::new();
    }
    let mut rules = PREFIX_RULES
        .iter()
        .filter(|rule| starts_with(expanded, rule.prefix))
        .map(|rule| (rule.rule, rule.message))
        .collect::<Vec<_>>();
    rules.extend(name_rules(written));
    rules
}

/// Every rule a glob import of `module` would bring into scope: the rules of the module
/// itself and of every banned item below it.
fn glob_rules(module: &[String]) -> Vec<(&'static str, &'static str)> {
    PREFIX_RULES
        .iter()
        .filter(|rule| starts_with(module, rule.prefix) || is_strict_prefix(module, rule.prefix))
        .map(|rule| (rule.rule, rule.message))
        .collect()
}

fn is_strict_prefix(module: &[String], banned: &[&str]) -> bool {
    module.len() < banned.len() && module.iter().zip(banned).all(|(left, right)| left == right)
}

/// One leaf of a `use` tree: the full imported path, the local name it binds, and where.
struct UseLeaf {
    path: Vec<String>,
    local: Option<String>,
    glob: bool,
    line: usize,
}

fn use_leaves(tree: &UseTree, prefix: &mut Vec<String>, leaves: &mut Vec<UseLeaf>) {
    match tree {
        UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            use_leaves(&path.tree, prefix, leaves);
            prefix.pop();
        }
        UseTree::Name(name) => {
            let (path, local) = named_leaf(prefix, &name.ident.to_string(), None);
            leaves.push(UseLeaf { path, local, glob: false, line: line_of(name.ident.span()) });
        }
        UseTree::Rename(rename) => {
            let alias = rename.rename.to_string();
            let (path, local) = named_leaf(prefix, &rename.ident.to_string(), Some(alias));
            leaves.push(UseLeaf { path, local, glob: false, line: line_of(rename.rename.span()) });
        }
        UseTree::Glob(glob) => leaves.push(UseLeaf {
            path: prefix.clone(),
            local: None,
            glob: true,
            line: line_of(glob.star_token.span),
        }),
        UseTree::Group(group) => {
            for item in &group.items {
                use_leaves(item, prefix, leaves);
            }
        }
    }
}

/// Resolves `self` in a group (`use std::{self as s}`) and picks the bound local name.
fn named_leaf(
    prefix: &[String],
    ident: &str,
    alias: Option<String>,
) -> (Vec<String>, Option<String>) {
    let mut path = prefix.to_vec();
    if ident != "self" {
        path.push(ident.to_string());
    }
    let local = alias.or_else(|| path.last().cloned()).filter(|name| name != "_");
    (path, local)
}

/// Collects the file's `use` leaves, wherever they are declared.
#[derive(Default)]
struct UseCollector {
    leaves: Vec<UseLeaf>,
}

impl<'ast> Visit<'ast> for UseCollector {
    fn visit_item_use(&mut self, node: &'ast ItemUse) {
        use_leaves(&node.tree, &mut Vec::new(), &mut self.leaves);
        visit::visit_item_use(self, node);
    }
}

/// Maps each imported local name to the full path it stands for. The map is file-wide,
/// not per module scope, so it can only over-report.
fn alias_map(leaves: &[UseLeaf]) -> BTreeMap<String, Vec<String>> {
    leaves
        .iter()
        .filter(|leaf| !leaf.glob)
        .filter_map(|leaf| Some((leaf.local.clone()?, leaf.path.clone())))
        .collect()
}

struct Scanner {
    aliases: BTreeMap<String, Vec<String>>,
    seen: BTreeSet<(usize, &'static str)>,
    findings: Vec<Finding>,
}

impl Scanner {
    fn report(&mut self, line: usize, rules: Vec<(&'static str, &'static str)>) {
        for (rule, message) in rules {
            if self.seen.insert((line, rule)) {
                self.findings.push(Finding { line, rule, message });
            }
        }
    }

    /// Replaces an imported first segment with the path it was imported from.
    fn expand(&self, written: &[String]) -> Vec<String> {
        let Some((first, rest)) = written.split_first() else {
            return Vec::new();
        };
        let mut expanded = self.aliases.get(first).cloned().unwrap_or_else(|| vec![first.clone()]);
        expanded.extend(rest.iter().cloned());
        expanded
    }

    fn check_path(&mut self, line: usize, written: &[String]) {
        let expanded = self.expand(written);
        let rules = path_rules(&expanded, written);
        self.report(line, rules);
    }

    fn check_use_leaf(&mut self, leaf: &UseLeaf) {
        let expanded = self.expand(&leaf.path);
        let rules =
            if leaf.glob { glob_rules(&expanded) } else { path_rules(&expanded, &leaf.path) };
        self.report(leaf.line, rules);
    }
}

impl<'ast> Visit<'ast> for Scanner {
    fn visit_path(&mut self, node: &'ast SynPath) {
        let written =
            node.segments.iter().map(|segment| segment.ident.to_string()).collect::<Vec<_>>();
        let line = node.segments.first().map_or(0, |segment| line_of(segment.ident.span()));
        self.check_path(line, &written);
        visit::visit_path(self, node);
    }

    fn visit_macro(&mut self, node: &'ast Macro) {
        for path in token_paths(&node.tokens) {
            self.check_path(path.line, &path.segments);
        }
        visit::visit_macro(self, node);
    }
}

/// Finds every source-rule violation in a parsed file, at most once per line and rule.
pub(super) fn source_findings(file: &File) -> Vec<Finding> {
    let mut collector = UseCollector::default();
    collector.visit_file(file);
    let mut scanner = Scanner {
        aliases: alias_map(&collector.leaves),
        seen: BTreeSet::new(),
        findings: Vec::new(),
    };
    for leaf in &collector.leaves {
        scanner.check_use_leaf(leaf);
    }
    scanner.visit_file(file);
    scanner.findings.sort_by_key(|finding| (finding.line, finding.rule));
    scanner.findings
}

/// Finds lines whose text names an open-addressing table, comments included.
pub(super) fn text_findings(lines: &[String]) -> Vec<Finding> {
    lines
        .iter()
        .enumerate()
        .filter(|(_, line)| {
            let lower = line.to_ascii_lowercase();
            lower.contains("open addressing") || lower.contains("open-addressing")
        })
        .map(|(index, _)| Finding {
            line: index + 1,
            rule: "hand-hashed-table",
            message: HAND_HASHED,
        })
        .collect()
}

/// A `::`-separated identifier path found in a macro token stream.
pub(super) struct TokenPath {
    pub(super) line: usize,
    pub(super) segments: Vec<String>,
}

/// Extracts every identifier path from a token stream, descending into groups.
pub(super) fn token_paths(tokens: &TokenStream) -> Vec<TokenPath> {
    let mut paths = Vec::new();
    collect_token_paths(tokens, &mut paths);
    paths
}

fn collect_token_paths(tokens: &TokenStream, paths: &mut Vec<TokenPath>) {
    let tokens = tokens.clone().into_iter().collect::<Vec<_>>();
    let mut index = 0;
    while let Some(token) = tokens.get(index) {
        match token {
            TokenTree::Group(group) => {
                collect_token_paths(&group.stream(), paths);
                index += 1;
            }
            TokenTree::Ident(ident) => {
                let mut segments = vec![ident.to_string()];
                index += 1;
                while let (true, Some(TokenTree::Ident(next))) =
                    (is_path_separator(&tokens, index), tokens.get(index + 2))
                {
                    segments.push(next.to_string());
                    index += 3;
                }
                paths.push(TokenPath { line: line_of(ident.span()), segments });
            }
            TokenTree::Punct(_) | TokenTree::Literal(_) => index += 1,
        }
    }
}

fn is_path_separator(tokens: &[TokenTree], index: usize) -> bool {
    match (tokens.get(index), tokens.get(index + 1)) {
        (Some(TokenTree::Punct(first)), Some(TokenTree::Punct(second))) => {
            first.as_char() == ':' && first.spacing() == Spacing::Joint && second.as_char() == ':'
        }
        _ => false,
    }
}

/// Lines of float literals in a token stream, descending into groups.
pub(super) fn token_float_literal_lines(tokens: &TokenStream) -> Vec<usize> {
    let mut lines = Vec::new();
    for token in tokens.clone() {
        match token {
            TokenTree::Group(group) => lines.extend(token_float_literal_lines(&group.stream())),
            TokenTree::Literal(literal) => {
                let line = line_of(literal.span());
                if matches!(Lit::new(literal), Lit::Float(_)) {
                    lines.push(line);
                }
            }
            TokenTree::Ident(_) | TokenTree::Punct(_) => {}
        }
    }
    lines
}

/// Crates whose presence in a deterministic crate's normal dependency graph is banned,
/// with the Echo rule each one belongs to.
const BANNED_DEPENDENCIES: &[(&str, &str)] = &[
    ("rand", "random"),
    ("getrandom", "random"),
    ("fastrand", "random"),
    ("js-sys", "host-callback"),
    ("wasm-bindgen", "host-callback"),
    ("web-sys", "host-callback"),
    ("reqwest", "network"),
    ("ureq", "network"),
    ("hashbrown", "unordered-container"),
    ("rustc-hash", "unordered-container"),
    ("serde_json", "json-serialization"),
    ("serde-wasm-bindgen", "wasm-serde"),
    ("tokio", "thread-sleep"),
    ("async-std", "thread-sleep"),
];

/// Finds banned crates in `cargo tree --prefix depth --format {p} --no-dedupe` output.
///
/// Each depth-0 line starts a deterministic crate's tree. The subtree below another
/// crate named in `deterministic` is skipped, because that crate's own tree reports it,
/// so a finding names the crate whose manifest declares the path to the banned crate.
pub(super) fn dependency_findings(tree: &str, deterministic: &[&str]) -> Vec<DependencyFinding> {
    let mut findings = Vec::new();
    let mut root = None::<String>;
    let mut skip_below = None::<usize>;
    for (depth, name) in tree.lines().filter_map(tree_entry) {
        if depth == 0 {
            root = Some(name.to_string());
            skip_below = None;
            continue;
        }
        if skip_below.is_some_and(|limit| depth > limit) {
            continue;
        }
        skip_below = deterministic.contains(&name).then_some(depth);
        let banned = BANNED_DEPENDENCIES.iter().find(|(banned, _)| *banned == name);
        if let (Some(root), Some((_, rule))) = (&root, banned) {
            findings.push(DependencyFinding {
                root: root.clone(),
                dependency: name.to_string(),
                rule,
            });
        }
    }
    findings
}

/// Splits `3serde_json v1.0.140` into its depth and crate name.
fn tree_entry(line: &str) -> Option<(usize, &str)> {
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    let depth = line.get(..digits)?.parse().ok()?;
    let name = line.get(digits..)?.split_whitespace().next()?;
    Some((depth, name))
}

fn line_of(span: Span) -> usize {
    span.start().line
}

#[cfg(test)]
mod tests;
